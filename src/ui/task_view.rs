use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::panel::full_view_block;
use crate::app::{App, DueState, Priority, TaskFilterKind, TaskItem, TaskRowHit};
use crate::config::Theme;

const CHECKBOX_WIDTH: usize = 4;
const PRIORITY_WIDTH: usize = 3;
const DATE_WIDTH: usize = 13;
const COLUMN_GAP: usize = 2;
const MIN_TEXT_WIDTH: usize = 12;
const MIN_NOTE_WIDTH: usize = 10;
const MAX_NOTE_WIDTH: usize = 28;

/// Column widths for one task row. Columns to the right of the text are
/// dropped one by one as the terminal narrows so the task text always keeps
/// a readable minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Columns {
    text: usize,
    priority: bool,
    date: bool,
    note: usize,
}

impl Columns {
    fn fit(available: usize) -> Self {
        let mut columns = Self { text: 0, priority: true, date: true, note: (available / 4).clamp(MIN_NOTE_WIDTH, MAX_NOTE_WIDTH) };
        loop {
            let used = CHECKBOX_WIDTH + columns.trailing_width();
            if available >= used + MIN_TEXT_WIDTH {
                columns.text = available - used;
                return columns;
            }
            if columns.note > 0 {
                columns.note = 0;
            } else if columns.date {
                columns.date = false;
            } else if columns.priority {
                columns.priority = false;
            } else {
                columns.text = available.saturating_sub(CHECKBOX_WIDTH);
                return columns;
            }
        }
    }

    fn trailing_width(self) -> usize {
        let mut width = 0;
        if self.priority {
            width += COLUMN_GAP + PRIORITY_WIDTH;
        }
        if self.date {
            width += COLUMN_GAP + DATE_WIDTH;
        }
        if self.note > 0 {
            width += COLUMN_GAP + self.note;
        }
        width
    }
}

/// Full-screen aggregate task view: every checkbox task in the vault, with
/// status/due/priority/text filters and in-place toggling.
pub fn render_task_view(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let theme = app.state.theme.clone();
    f.render_widget(Clear, area);
    let block = full_view_block(app.state.config.style, &theme, title_line(app, &theme));
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.tasks.row_hits.clear();
    app.tasks.filter_hits.clear();
    app.tasks.list_area = Rect::default();
    if inner.width < 8 || inner.height < 2 {
        return;
    }

    let header_rows = if inner.height >= 8 { 2 } else { 1 };
    let footer_rows = u16::from(inner.height >= 4);
    let list_area = Rect::new(inner.x, inner.y + header_rows, inner.width, inner.height.saturating_sub(header_rows + footer_rows).max(1));
    render_filter_row(f, app, &theme, Rect::new(inner.x, inner.y, inner.width, 1));
    render_rows(f, app, &theme, list_area);
    if footer_rows > 0 {
        render_footer(f, app, &theme, Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1));
    }
}

fn title_line(app: &App, theme: &Theme) -> Line<'static> {
    let open = app.tasks.tasks.iter().filter(|task| !task.checked).count();
    let total = app.tasks.tasks.len();
    let mut spans = vec![Span::styled(" TASKS ", Style::default().fg(theme.dialog.title).add_modifier(Modifier::BOLD))];
    if app.tasks.scanned_once() || total > 0 {
        spans.push(Span::styled(format!(" {open} open · {total} total "), Style::default().fg(theme.muted)));
    }
    if app.tasks_loading() {
        spans.push(Span::styled("◌ scanning ", Style::default().fg(theme.info)));
    }
    Line::from(spans)
}

fn render_filter_row(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let key_style = Style::default().fg(theme.warning).add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.muted);
    let default_style = Style::default().fg(theme.dialog.text);
    let active_style = Style::default().fg(theme.info).add_modifier(Modifier::BOLD);
    let value_style = |is_default: bool| if is_default { default_style } else { active_style };

    let mut chips: Vec<(TaskFilterKind, Vec<Span<'static>>)> = vec![
        (TaskFilterKind::Status, vec![Span::styled("f ", key_style), Span::styled(app.tasks.status.label(), value_style(app.tasks.status.is_default()))]),
        (TaskFilterKind::Due, vec![Span::styled("d ", key_style), Span::styled("due ", label_style), Span::styled(app.tasks.due.label(), value_style(app.tasks.due.is_default()))]),
        (TaskFilterKind::Priority, vec![Span::styled("p ", key_style), Span::styled("priority ", label_style), Span::styled(app.tasks.priority.label(), value_style(app.tasks.priority.is_default()))]),
    ];
    let mut search = vec![Span::styled("/ ", key_style)];
    if app.tasks.text_input_active {
        search.push(Span::styled(app.tasks.query.clone(), Style::default().fg(theme.search.input)));
        search.push(Span::styled("▏", Style::default().fg(theme.primary)));
        search.push(Span::styled("  Enter done · Esc cancel", label_style));
    } else if app.tasks.query.is_empty() {
        search.push(Span::styled("search", label_style));
    } else {
        search.push(Span::styled(app.tasks.query.clone(), active_style));
    }
    chips.push((TaskFilterKind::Search, search));

    let mut spans = vec![Span::raw(" ")];
    let mut x = area.x.saturating_add(1);
    let right = area.x.saturating_add(area.width);
    for (index, (kind, chip)) in chips.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
            x = x.saturating_add(3);
        }
        let width: usize = chip.iter().map(|span| span.content.width()).sum();
        let width = u16::try_from(width).unwrap_or(u16::MAX);
        if width > right.saturating_sub(x) {
            break;
        }
        app.tasks.filter_hits.push((kind, Rect::new(x, area.y, width, 1)));
        x = x.saturating_add(width);
        spans.extend(chip);
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_rows(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    app.tasks.list_area = area;
    let list_height = area.height as usize;
    app.tasks.clamp_scroll(list_height);
    let count = app.tasks.visible.len();
    if count == 0 {
        render_empty_state(f, app, theme, area);
        return;
    }
    let scrollbar = count > list_height;
    let row_width = area.width.saturating_sub(u16::from(scrollbar));
    let available = (row_width as usize).saturating_sub(2);
    let columns = Columns::fit(available);
    let today = app.today();
    let mut hits = Vec::with_capacity(list_height);
    for (row_index, position) in (app.tasks.scroll_offset..count.min(app.tasks.scroll_offset + list_height)).enumerate() {
        let Some(task) = app.tasks.visible.get(position).and_then(|&index| app.tasks.tasks.get(index)) else {
            continue;
        };
        let row = Rect::new(area.x, area.y + row_index as u16, row_width, 1);
        let selected = position == app.tasks.selected;
        let line = task_row(task, task.due_state(today), columns, selected, theme);
        let background = if selected { theme.selection } else { theme.dialog.background };
        f.render_widget(Paragraph::new(line).style(Style::default().bg(background)), row);
        hits.push(TaskRowHit { position, row, checkbox: Rect::new(row.x, row.y, (CHECKBOX_WIDTH as u16 + 1).min(row.width), 1) });
    }
    app.tasks.row_hits = hits;
    if scrollbar {
        let mut state = ScrollbarState::new(count).position(app.tasks.scroll_offset).viewport_content_length(list_height);
        let scrollbar_area = Rect::new(area.x + row_width, area.y, 1, area.height);
        f.render_stateful_widget(Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None).track_symbol(Some("│")).thumb_symbol("┃").style(Style::default().fg(theme.muted)).thumb_style(Style::default().fg(theme.primary)), scrollbar_area, &mut state);
    }
}

fn task_row(task: &TaskItem, due_state: DueState, columns: Columns, selected: bool, theme: &Theme) -> Line<'static> {
    let text_modifier = if selected { Modifier::BOLD } else { Modifier::empty() };
    let text_color = if task.checked { theme.muted } else { theme.dialog.text };
    let checkbox = if task.checked { ("[x] ", theme.success) } else { ("[ ] ", theme.primary) };
    let mut spans = vec![Span::raw(" "), Span::styled(checkbox.0, Style::default().fg(checkbox.1)), Span::styled(fit(&task.text, columns.text), Style::default().fg(text_color).add_modifier(text_modifier))];
    if columns.priority {
        let priority = task.priority.map_or((String::from("   "), theme.muted), |priority| (fit(priority.token(), PRIORITY_WIDTH), priority_color(priority, theme)));
        spans.push(Span::raw(" ".repeat(COLUMN_GAP)));
        spans.push(Span::styled(priority.0, Style::default().fg(priority.1)));
    }
    if columns.date {
        let date = if let Some(due) = task.due {
            let color = match due_state {
                DueState::Overdue => theme.error,
                DueState::Today => theme.warning,
                DueState::Upcoming | DueState::Undated => theme.muted,
            };
            (fit(&format!("📅 {due}"), DATE_WIDTH), color)
        } else if let Some(done) = task.done.filter(|_| task.checked) {
            (fit(&format!("✅ {done}"), DATE_WIDTH), theme.muted)
        } else {
            (" ".repeat(DATE_WIDTH), theme.muted)
        };
        spans.push(Span::raw(" ".repeat(COLUMN_GAP)));
        spans.push(Span::styled(date.0, Style::default().fg(date.1)));
    }
    if columns.note > 0 {
        spans.push(Span::raw(" ".repeat(COLUMN_GAP)));
        spans.push(Span::styled(fit(&task.note_title, columns.note), Style::default().fg(theme.muted)));
    }
    Line::from(spans)
}

fn render_empty_state(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let filtered = app.tasks.has_active_filters();
    let message = if app.tasks_loading() && app.tasks.tasks.is_empty() {
        "Collecting tasks…"
    } else if app.tasks.tasks.is_empty() {
        "No tasks in this vault yet. Add a `- [ ]` line to any note."
    } else {
        "No tasks match the current filters"
    };
    let mut lines = vec![Line::from(Span::styled(message, Style::default().fg(theme.muted)))];
    if filtered && !app.tasks.tasks.is_empty() {
        lines.push(Line::from(vec![Span::styled("Press ", Style::default().fg(theme.muted)), Span::styled("c", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)), Span::styled(" to clear the filters", Style::default().fg(theme.muted))]));
    }
    let height = u16::try_from(lines.len()).unwrap_or(1).min(area.height);
    let y = area.y + area.height.saturating_sub(height) / 2;
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), Rect::new(area.x, y, area.width, height));
}

fn render_footer(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let mut items: Vec<(&'static str, &'static str, u8)> = vec![("Space", "toggle", 0), ("Enter", "open", 1), ("r", "rescan", 4), ("Esc", "close", 2)];
    if app.tasks.has_active_filters() {
        items.insert(2, ("c", "clear", 3));
    }
    let count = app.tasks.visible.len();
    let indicator = if count == 0 { String::from("0/0 ") } else { format!("{}/{} ", app.tasks.selected + 1, count) };
    let total = area.width as usize;
    let item_width = |(key, hint, _): &(&str, &str, u8)| key.width() + 1 + hint.width();
    let mut show_indicator = true;
    loop {
        let help_width = 1 + items.iter().map(item_width).sum::<usize>() + COLUMN_GAP * items.len().saturating_sub(1);
        let needed = help_width + if show_indicator { COLUMN_GAP + indicator.width() } else { 0 };
        if needed <= total || items.is_empty() {
            break;
        }
        let optional = items.iter().enumerate().filter(|(_, item)| item.2 >= 3).max_by_key(|(_, item)| item.2).map(|(index, _)| index);
        if let Some(drop) = optional {
            items.remove(drop);
        } else if show_indicator {
            show_indicator = false;
        } else if let Some(drop) = items.iter().enumerate().max_by_key(|(_, item)| item.2).map(|(index, _)| index) {
            items.remove(drop);
        }
    }
    let mut spans = vec![Span::raw(" ")];
    for (index, (key_text, hint_text, _)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" ".repeat(COLUMN_GAP)));
        }
        spans.push(key(key_text, theme));
        spans.push(Span::raw(" "));
        spans.push(hint(hint_text, theme));
    }
    if show_indicator {
        let indicator_width = indicator.width() as u16;
        let help_area = Rect::new(area.x, area.y, area.width.saturating_sub(indicator_width), 1);
        f.render_widget(Paragraph::new(Line::from(spans)), help_area);
        f.render_widget(Paragraph::new(Line::from(Span::styled(indicator, Style::default().fg(theme.muted)))).alignment(Alignment::Right), Rect::new(area.x + help_area.width, area.y, indicator_width, 1));
    } else {
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

fn priority_color(priority: Priority, theme: &Theme) -> ratatui::style::Color {
    match priority {
        Priority::High => theme.error,
        Priority::Medium => theme.warning,
        Priority::Low => theme.info,
    }
}

fn key(text: &'static str, theme: &Theme) -> Span<'static> {
    Span::styled(text, Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
}

fn hint(text: &'static str, theme: &Theme) -> Span<'static> {
    Span::styled(text, Style::default().fg(theme.muted))
}

/// Truncate `text` to at most `width` terminal columns (with an ellipsis when
/// it does not fit) and pad it with spaces to exactly `width` columns so
/// columns stay aligned regardless of glyph widths.
fn fit(text: &str, width: usize) -> String {
    let mut fitted = truncate(text, width);
    let used = fitted.width();
    if used < width {
        fitted.extend(std::iter::repeat_n(' ', width - used));
    }
    fitted
}

fn truncate(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.width() <= width {
        return text.to_string();
    }
    let mut used = 0usize;
    let mut end = 0usize;
    for (index, character) in text.char_indices() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > width - 1 {
            break;
        }
        used += character_width;
        end = index + character.len_utf8();
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_limits_text_to_available_columns() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
        assert_eq!(truncate("hello", 3), "he…");
        assert_eq!(truncate("hello", 1), "…");
        assert_eq!(truncate("hello", 0), "");
        assert_eq!(truncate("日本語です", 5), "日本…");
    }

    #[test]
    fn fit_pads_to_exact_width_including_wide_glyphs() {
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("📅 2026-06-01", DATE_WIDTH).width(), DATE_WIDTH);
        assert_eq!(fit("⏫", PRIORITY_WIDTH), "⏫ ");
        assert_eq!(fit("a very long title", 6), "a ver…");
        assert_eq!(fit("", 3), "   ");
    }

    #[test]
    fn columns_drop_from_the_right_as_width_shrinks() {
        let wide = Columns::fit(120);
        assert!(wide.priority && wide.date);
        assert_eq!(wide.note, MAX_NOTE_WIDTH);
        assert_eq!(wide.text, 120 - CHECKBOX_WIDTH - wide.trailing_width());

        let medium = Columns::fit(60);
        assert!(medium.priority && medium.date);
        assert_eq!(medium.note, 15);
        assert!(medium.text >= MIN_TEXT_WIDTH);

        let narrow = Columns::fit(40);
        assert_eq!(narrow.note, 0);
        assert!(narrow.date && narrow.priority);
        assert_eq!(narrow.text, 40 - CHECKBOX_WIDTH - COLUMN_GAP - PRIORITY_WIDTH - COLUMN_GAP - DATE_WIDTH);

        let tiny = Columns::fit(22);
        assert!(!tiny.date && tiny.priority && tiny.note == 0);
        assert_eq!(tiny.text, 22 - CHECKBOX_WIDTH - COLUMN_GAP - PRIORITY_WIDTH);

        let bare = Columns::fit(20);
        assert!(!bare.priority && !bare.date && bare.note == 0);
        assert_eq!(bare.text, 16);

        let minimal = Columns::fit(6);
        assert!(!minimal.priority && !minimal.date && minimal.note == 0);
        assert_eq!(minimal.text, 2);
        assert_eq!(Columns::fit(0).text, 0);
    }
}
