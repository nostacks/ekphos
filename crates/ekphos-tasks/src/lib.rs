//! Markdown task parsing for Ekphos.
//!
//! Tasks are plain Markdown list items with a GFM checkbox, optionally carrying
//! Obsidian Tasks plugin-style emoji metadata on the line itself:
//!
//! ```text
//! - [ ] Pay rent +home 📅 2026-06-01 ⏫
//! - [x] Submit report ✅ 2026-05-30
//! ```
//!
//! Because the metadata lives on the task line, tasks stay portable Markdown:
//! any editor (including Obsidian's Tasks plugin) can read and write them.

use chrono::NaiveDate;
use ekphos_core::NoteId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    pub const fn token(self) -> &'static str {
        match self {
            Self::High => "⏫",
            Self::Medium => "🔼",
            Self::Low => "🔽",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub const fn rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }

    fn from_token(token: &str) -> Option<Self> {
        match token {
            "⏫" => Some(Self::High),
            "🔼" => Some(Self::Medium),
            "🔽" => Some(Self::Low),
            _ => None,
        }
    }
}

/// One aggregated task from a Markdown note. `text` is the task body with
/// the emoji metadata tokens removed, suitable for display and search.
#[derive(Debug, Clone)]
pub struct Task {
    pub note_id: NoteId,
    pub path: String,
    pub note_title: String,
    pub source_line: usize,
    pub indent: u16,
    pub text: String,
    pub checked: bool,
    pub due: Option<NaiveDate>,
    pub start: Option<NaiveDate>,
    pub done: Option<NaiveDate>,
    pub priority: Option<Priority>,
}

/// How a task's due date relates to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueState {
    Undated,
    Overdue,
    Today,
    Upcoming,
}

impl Task {
    /// Classify the due date against `today`. Completed tasks are never
    /// overdue or due today, so they render calmly in the done list.
    pub fn due_state(&self, today: NaiveDate) -> DueState {
        match self.due {
            None => DueState::Undated,
            Some(_) if self.checked => DueState::Upcoming,
            Some(due) if due < today => DueState::Overdue,
            Some(due) if due == today => DueState::Today,
            Some(_) => DueState::Upcoming,
        }
    }
}

/// A parsed task line. `body_offset` is the byte offset within the original
/// line where the task text begins (after the `MARKER [x] ` prefix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTaskLine<'a> {
    pub body: &'a str,
    pub body_offset: usize,
    pub indent: u16,
    pub checked: bool,
    pub due: Option<NaiveDate>,
    pub start: Option<NaiveDate>,
    pub done: Option<NaiveDate>,
    pub priority: Option<Priority>,
}

const TASK_PREFIX_LEN: usize = 6;

/// Parse a Markdown line as a checkbox task. Accepts `-`, `*`, and `+`
/// markers with ` `, `x`, or `X` checkbox states, matching the editor's
/// list-continuation rules.
pub fn parse_task_line(line: &str) -> Option<ParsedTaskLine<'_>> {
    let trimmed = line.trim_start_matches([' ', '\t']);
    let leading = line.len().saturating_sub(trimmed.len());
    let bytes = trimmed.as_bytes();
    if bytes.len() < TASK_PREFIX_LEN {
        return None;
    }
    if bytes[1] != b' ' || bytes[2] != b'[' || bytes[4] != b']' || bytes[5] != b' ' {
        return None;
    }
    if !matches!(bytes[0], b'-' | b'*' | b'+') {
        return None;
    }
    let checked = match bytes[3] {
        b' ' => false,
        b'x' | b'X' => true,
        _ => return None,
    };
    let body = &trimmed[TASK_PREFIX_LEN..];
    let indent = line[..leading].chars().map(|character| u16::from(character == '\t') * 3 + 1).sum();
    let due = extract_date(body, "📅");
    let start = extract_date(body, "🛫");
    let done = extract_date(body, "✅");
    let priority = Priority::from_token(body.trim()).or_else(|| find_priority(body));
    Some(ParsedTaskLine { body, body_offset: leading + TASK_PREFIX_LEN, indent, checked, due, start, done, priority })
}

/// Rewrite a task line with the checkbox flipped. Completing stamps a `✅`
/// completion date; reopening removes it. Returns `None` when the line is not
/// a task.
pub fn set_checked(line: &str, checked: bool, today: NaiveDate) -> Option<String> {
    parse_task_line(line)?;
    let trimmed = line.trim_start_matches([' ', '\t']);
    let leading = line.len().saturating_sub(trimmed.len());
    let mut updated = line.to_string();
    let state_index = leading + 3;
    updated.replace_range(state_index..state_index + 1, if checked { "x" } else { " " });
    if checked {
        match done_token_range(&updated) {
            Some(range) => {
                updated.replace_range(range, &format!(" ✅ {today}"));
            }
            None => {
                let end = updated.trim_end().len();
                updated.truncate(end);
                updated.push_str(&format!(" ✅ {today}"));
            }
        }
    } else if let Some(range) = done_token_range(&updated) {
        updated.replace_range(range, "");
    }
    Some(updated)
}

/// Task body with the metadata tokens (`📅`, `🛫`, `✅` dates and priority
/// markers) removed and whitespace collapsed. The result is what the task
/// view shows and searches; the source line keeps the tokens.
pub fn strip_metadata(body: &str) -> String {
    let mut words = body.split_whitespace().peekable();
    let mut parts: Vec<&str> = Vec::new();
    while let Some(word) = words.next() {
        if Priority::from_token(word).is_some() {
            continue;
        }
        if DATE_TOKENS.contains(&word) {
            if words.peek().is_some_and(|next| NaiveDate::parse_from_str(next, "%Y-%m-%d").is_ok()) {
                words.next();
            }
            continue;
        }
        parts.push(word);
    }
    parts.join(" ")
}

const DATE_TOKENS: [&str; 3] = ["📅", "🛫", "✅"];

/// Byte range of line `source_line` in `body` (excluding its terminator),
/// provided that line still parses as a task whose display text is
/// `expected_text`. Returns `None` when the note changed underneath the
/// caller, so a stale task index never rewrites the wrong line.
pub fn locate_task_line(body: &str, source_line: usize, expected_text: &str) -> Option<std::ops::Range<usize>> {
    let mut offset = 0usize;
    for (index, raw) in body.split_inclusive('\n').enumerate() {
        if index == source_line {
            let line = raw.trim_end_matches(['\n', '\r']);
            let parsed = parse_task_line(line)?;
            return (strip_metadata(parsed.body) == expected_text).then_some(offset..offset + line.len());
        }
        offset += raw.len();
    }
    None
}

/// Collect every task in a Markdown body, skipping fenced code blocks.
pub fn collect_tasks(body: &str, note_id: NoteId, path: &str, note_title: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut fence: Option<&str> = None;
    for (source_line, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        let marker = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (&fence, marker) {
            (None, Some(opened)) => fence = Some(opened),
            (Some(opened), Some(_)) if trimmed.starts_with(opened) => fence = None,
            (Some(_), _) => {}
            (None, None) => {
                if let Some(parsed) = parse_task_line(line) {
                    tasks.push(Task { note_id, path: path.to_string(), note_title: note_title.to_string(), source_line, indent: parsed.indent, text: strip_metadata(parsed.body), checked: parsed.checked, due: parsed.due, start: parsed.start, done: parsed.done, priority: parsed.priority });
                }
            }
        }
    }
    tasks
}

/// Order tasks for the aggregate view: open before done, then by due date
/// (undated last), then by priority (unprioritized last), then file order.
pub fn sort_tasks(tasks: &mut [Task]) {
    tasks.sort_by(|left, right| {
        left.checked
            .cmp(&right.checked)
            .then_with(|| match (left.due, right.due) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(left_due), Some(right_due)) => left_due.cmp(&right_due),
            })
            .then_with(|| left.priority.map(Priority::rank).unwrap_or(u8::MAX).cmp(&right.priority.map(Priority::rank).unwrap_or(u8::MAX)))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.source_line.cmp(&right.source_line))
    });
}

fn extract_date(body: &str, emoji: &str) -> Option<NaiveDate> {
    let mut search = 0;
    while let Some(offset) = body[search..].find(emoji) {
        let start = search + offset + emoji.len();
        let candidate = body[start..].split_whitespace().next()?;
        search = start;
        if let Ok(date) = NaiveDate::parse_from_str(candidate, "%Y-%m-%d") {
            return Some(date);
        }
    }
    None
}

fn find_priority(body: &str) -> Option<Priority> {
    body.split_whitespace().find_map(|word| {
        let trimmed = word.trim_matches(|character: char| !character.is_alphanumeric() && !['⏫', '🔼', '🔽'].contains(&character));
        Priority::from_token(trimmed)
    })
}

/// Byte range covering an existing `✅ YYYY-MM-DD` token plus the separating
/// space before it and one after it, so its removal leaves clean spacing.
fn done_token_range(line: &str) -> Option<std::ops::Range<usize>> {
    let emoji_start = line.find("✅")?;
    let mut end = emoji_start + "✅".len();
    let remainder = &line[end..];
    let spaces = remainder.len() - remainder.trim_start_matches(' ').len();
    let candidate_start = end + spaces;
    if line.len() - candidate_start >= 10 && NaiveDate::parse_from_str(&line[candidate_start..candidate_start + 10], "%Y-%m-%d").is_ok() {
        end = candidate_start + 10;
        if line[end..].starts_with(' ') {
            end += 1;
        }
    }
    let start = line[..emoji_start].trim_end().len();
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parses_dash_star_and_plus_markers() {
        for marker in ["- [ ] task", "* [x] task", "+ [X] task"] {
            let parsed = parse_task_line(marker).expect("task");
            assert_eq!(parsed.body, "task");
            assert_eq!(parsed.checked, marker.contains("[x]") || marker.contains("[X]"));
            assert_eq!(parsed.indent, 0);
            assert_eq!(parsed.body_offset, 6);
        }
        assert!(parse_task_line("- task").is_none());
        assert!(parse_task_line("-[]task").is_none());
        assert!(parse_task_line("regular text").is_none());
    }

    #[test]
    fn parses_indented_tasks_with_tab_stops() {
        let parsed = parse_task_line("  - [ ] nested").unwrap();
        assert_eq!(parsed.indent, 2);
        let parsed = parse_task_line("\t- [ ] nested").unwrap();
        assert_eq!(parsed.indent, 4);
    }

    #[test]
    fn extracts_due_start_done_and_priority() {
        let parsed = parse_task_line("- [ ] Pay rent 🛫 2026-05-01 📅 2026-06-01 ⏫").unwrap();
        assert_eq!(parsed.due, Some(date("2026-06-01")));
        assert_eq!(parsed.start, Some(date("2026-05-01")));
        assert_eq!(parsed.done, None);
        assert_eq!(parsed.priority, Some(Priority::High));

        let parsed = parse_task_line("- [x] Submitted ✅ 2026-05-30 🔽").unwrap();
        assert_eq!(parsed.done, Some(date("2026-05-30")));
        assert_eq!(parsed.priority, Some(Priority::Low));
        assert!(parsed.checked);
    }

    #[test]
    fn invalid_dates_are_ignored() {
        let parsed = parse_task_line("- [ ] task 📅 notadate").unwrap();
        assert_eq!(parsed.due, None);
    }

    #[test]
    fn set_checked_stamps_and_removes_completion_date() {
        let today = date("2026-09-04");
        let line = "- [ ] Pay rent 📅 2026-06-01";
        let done = set_checked(line, true, today).unwrap();
        assert_eq!(done, format!("- [x] Pay rent 📅 2026-06-01 ✅ {today}"));
        let reopened = set_checked(&done, false, today).unwrap();
        assert_eq!(reopened, "- [ ] Pay rent 📅 2026-06-01");
    }

    #[test]
    fn set_checked_replaces_existing_completion_date() {
        let today = date("2026-09-04");
        let line = "- [x] Done ✅ 2026-01-01";
        let updated = set_checked(line, true, today).unwrap();
        assert_eq!(updated, format!("- [x] Done ✅ {today}"));
    }

    #[test]
    fn set_checked_preserves_indent_and_marker() {
        let today = date("2026-09-04");
        let updated = set_checked("  * [ ] indented task", true, today).unwrap();
        assert_eq!(updated, format!("  * [x] indented task ✅ {today}"));
    }

    #[test]
    fn set_checked_rejects_non_task_lines() {
        let today = date("2026-09-04");
        assert!(set_checked("- plain item", true, today).is_none());
    }

    #[test]
    fn strip_metadata_removes_tokens_and_keeps_words() {
        assert_eq!(strip_metadata("Pay rent +home 📅 2026-06-01 ⏫"), "Pay rent +home");
        assert_eq!(strip_metadata("Submitted ✅ 2026-05-30 🔽"), "Submitted");
        assert_eq!(strip_metadata("  spaced   words 🛫 2026-01-01  "), "spaced words");
        assert_eq!(strip_metadata("📅 notadate keeps"), "notadate keeps");
        assert_eq!(strip_metadata("⏫"), "");
    }

    #[test]
    fn due_state_classifies_against_today() {
        let today = date("2026-09-05");
        let mut task = collect_tasks("- [ ] a 📅 2026-09-01\n", NoteId::new(1), "a.md", "A").remove(0);
        assert_eq!(task.due_state(today), DueState::Overdue);
        task.due = Some(today);
        assert_eq!(task.due_state(today), DueState::Today);
        task.due = Some(date("2026-09-06"));
        assert_eq!(task.due_state(today), DueState::Upcoming);
        task.checked = true;
        task.due = Some(date("2026-01-01"));
        assert_eq!(task.due_state(today), DueState::Upcoming);
        task.due = None;
        assert_eq!(task.due_state(today), DueState::Undated);
    }

    #[test]
    fn collect_tasks_stores_display_text() {
        let tasks = collect_tasks("- [ ] Pay rent 📅 2026-06-01 ⏫\n", NoteId::new(1), "a.md", "A");
        assert_eq!(tasks[0].text, "Pay rent");
        assert_eq!(tasks[0].due, Some(date("2026-06-01")));
    }

    #[test]
    fn locate_task_line_finds_matching_line_and_preserves_endings() {
        let body = "intro\r\n- [ ] alpha 📅 2026-06-01\r\n- [x] beta\r\n";
        let range = locate_task_line(body, 1, "alpha").unwrap();
        assert_eq!(&body[range.clone()], "- [ ] alpha 📅 2026-06-01");
        let mut rewritten = body.to_string();
        rewritten.replace_range(range, "- [x] alpha 📅 2026-06-01 ✅ 2026-09-05");
        assert_eq!(rewritten, "intro\r\n- [x] alpha 📅 2026-06-01 ✅ 2026-09-05\r\n- [x] beta\r\n");
        let last = locate_task_line("- [ ] only", 0, "only").unwrap();
        assert_eq!(last, 0..10);
    }

    #[test]
    fn locate_task_line_rejects_stale_positions() {
        let body = "- [ ] alpha\n- [ ] beta\n";
        assert!(locate_task_line(body, 0, "beta").is_none());
        assert!(locate_task_line(body, 5, "alpha").is_none());
        assert!(locate_task_line("plain\n", 0, "plain").is_none());
    }

    #[test]
    fn collect_tasks_skips_code_fences() {
        let body = "- [ ] real task\n```md\n- [ ] in fence\n```\n~~~\n- [ ] in tilde fence\n~~~\n- [ ] another real\n";
        let tasks = collect_tasks(body, NoteId::new(1), "notes.md", "Notes");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].source_line, 0);
        assert_eq!(tasks[1].source_line, 7);
    }

    #[test]
    fn sort_tasks_puts_open_urgent_first() {
        let mut tasks = vec![
            Task { note_id: NoteId::new(1), path: "a.md".into(), note_title: "A".into(), source_line: 0, indent: 0, text: "done".into(), checked: true, due: None, start: None, done: None, priority: Some(Priority::High) },
            Task { note_id: NoteId::new(1), path: "a.md".into(), note_title: "A".into(), source_line: 1, indent: 0, text: "undated".into(), checked: false, due: None, start: None, done: None, priority: None },
            Task { note_id: NoteId::new(1), path: "a.md".into(), note_title: "A".into(), source_line: 2, indent: 0, text: "overdue".into(), checked: false, due: Some(date("2026-01-01")), start: None, done: None, priority: None },
            Task { note_id: NoteId::new(1), path: "a.md".into(), note_title: "A".into(), source_line: 3, indent: 0, text: "high".into(), checked: false, due: Some(date("2026-06-01")), start: None, done: None, priority: Some(Priority::High) },
        ];
        sort_tasks(&mut tasks);
        let texts: Vec<&str> = tasks.iter().map(|task| task.text.as_str()).collect();
        assert_eq!(texts, ["overdue", "high", "undated", "done"]);
    }
}
