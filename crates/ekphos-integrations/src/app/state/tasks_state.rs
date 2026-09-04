use super::*;

use ekphos_tasks::Task;
pub use ekphos_tasks::{DueState, Priority, Task as TaskItem};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatusFilter {
    Open,
    Done,
    All,
}

impl TaskStatusFilter {
    pub const fn next(self) -> Self {
        match self {
            Self::Open => Self::Done,
            Self::Done => Self::All,
            Self::All => Self::Open,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Done => "Done",
            Self::All => "All",
        }
    }

    pub const fn is_default(self) -> bool {
        matches!(self, Self::Open)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDueFilter {
    Any,
    Overdue,
    Today,
    Week,
}

impl TaskDueFilter {
    pub const fn next(self) -> Self {
        match self {
            Self::Any => Self::Overdue,
            Self::Overdue => Self::Today,
            Self::Today => Self::Week,
            Self::Week => Self::Any,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Overdue => "overdue",
            Self::Today => "today",
            Self::Week => "this week",
        }
    }

    pub const fn is_default(self) -> bool {
        matches!(self, Self::Any)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriorityFilter {
    Any,
    High,
    Medium,
    Low,
}

impl TaskPriorityFilter {
    pub const fn next(self) -> Self {
        match self {
            Self::Any => Self::High,
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Any,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub const fn is_default(self) -> bool {
        matches!(self, Self::Any)
    }
}

/// One of the interactive controls in the task view header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFilterKind {
    Status,
    Due,
    Priority,
    Search,
}

/// Screen geometry of one rendered task row, recorded by the renderer for
/// mouse hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskRowHit {
    pub position: usize,
    pub row: Rect,
    pub checkbox: Rect,
}

/// Aggregate task view state. Tasks are collected by a background worker from
/// every Markdown note in the vault and filtered on the main thread.
pub struct TaskViewState {
    pub tasks: Vec<Task>,
    pub status: TaskStatusFilter,
    pub due: TaskDueFilter,
    pub priority: TaskPriorityFilter,
    pub query: String,
    pub text_input_active: bool,
    pub visible: Vec<usize>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub row_hits: Vec<TaskRowHit>,
    pub filter_hits: Vec<(TaskFilterKind, Rect)>,
    pub list_area: Rect,
    pub(crate) loading: bool,
    pub(crate) scanned_once: bool,
    pub(crate) dirty: bool,
    pub(crate) signature: (u64, u64),
    pub(crate) request_generation: u64,
    pub(crate) worker: TaskWorker,
}

impl Default for TaskViewState {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            status: TaskStatusFilter::Open,
            due: TaskDueFilter::Any,
            priority: TaskPriorityFilter::Any,
            query: String::new(),
            text_input_active: false,
            visible: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            row_hits: Vec::new(),
            filter_hits: Vec::new(),
            list_area: Rect::default(),
            loading: false,
            scanned_once: false,
            dirty: true,
            signature: (0, 0),
            request_generation: 0,
            worker: TaskWorker::new(),
        }
    }
}

impl TaskViewState {
    /// Indices into `tasks` matching the active filters, ordered by the
    /// canonical task sort.
    pub fn apply_filters(&mut self, today: chrono::NaiveDate) {
        ekphos_tasks::sort_tasks(&mut self.tasks);
        let query = self.query.trim().to_lowercase();
        self.visible = filter_task_indices(&self.tasks, self.status, self.due, self.priority, &query, today);
        self.selected = self.selected.min(self.visible.len().saturating_sub(1));
        self.scroll_offset = self.scroll_offset.min(self.selected);
    }

    /// The selected task, if any row is visible.
    pub fn selected_task(&self) -> Option<&Task> {
        self.visible.get(self.selected).and_then(|&index| self.tasks.get(index))
    }

    /// True once at least one scan has completed.
    pub fn scanned_once(&self) -> bool {
        self.scanned_once
    }

    /// True when any filter differs from its default.
    pub fn has_active_filters(&self) -> bool {
        !self.status.is_default() || !self.due.is_default() || !self.priority.is_default() || !self.query.trim().is_empty()
    }

    /// Bring `scroll_offset` into a range where the selection is visible and
    /// no blank rows trail the list when it fits.
    pub fn clamp_scroll(&mut self, list_height: usize) {
        let count = self.visible.len();
        if list_height == 0 || count == 0 {
            self.scroll_offset = 0;
            return;
        }
        self.selected = self.selected.min(count - 1);
        let max_offset = count.saturating_sub(list_height);
        self.scroll_offset = self.scroll_offset.min(max_offset);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + list_height {
            self.scroll_offset = self.selected + 1 - list_height;
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.visible.len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        let target = if delta < 0 { self.selected.saturating_sub(delta.unsigned_abs()) } else { self.selected.saturating_add(delta as usize) };
        self.selected = target.min(count - 1);
    }

    fn select(&mut self, position: usize) {
        let count = self.visible.len();
        self.selected = if count == 0 { 0 } else { position.min(count - 1) };
    }
}

pub(crate) fn filter_task_indices(tasks: &[Task], status: TaskStatusFilter, due: TaskDueFilter, priority: TaskPriorityFilter, query: &str, today: chrono::NaiveDate) -> Vec<usize> {
    tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| match status {
            TaskStatusFilter::Open => !task.checked,
            TaskStatusFilter::Done => task.checked,
            TaskStatusFilter::All => true,
        })
        .filter(|(_, task)| match due {
            TaskDueFilter::Any => true,
            TaskDueFilter::Overdue => task.due.is_some_and(|date| date < today) && !task.checked,
            TaskDueFilter::Today => task.due == Some(today),
            TaskDueFilter::Week => task.due.is_some_and(|date| date >= today && date <= today + chrono::Duration::days(7)),
        })
        .filter(|(_, task)| match priority {
            TaskPriorityFilter::Any => true,
            TaskPriorityFilter::High => task.priority == Some(ekphos_tasks::Priority::High),
            TaskPriorityFilter::Medium => task.priority == Some(ekphos_tasks::Priority::Medium),
            TaskPriorityFilter::Low => task.priority == Some(ekphos_tasks::Priority::Low),
        })
        .filter(|(_, task)| query.is_empty() || task.text.to_lowercase().contains(query) || task.note_title.to_lowercase().contains(query))
        .map(|(index, _)| index)
        .collect()
}

struct TaskScanRequest {
    generation: u64,
    root: PathBuf,
    notes: Vec<Note>,
}

struct TaskScanResponse {
    generation: u64,
    tasks: Vec<Task>,
}

enum TaskWorkerCommand {
    Scan(Box<TaskScanRequest>),
    Shutdown,
}

pub(crate) struct TaskWorker {
    command_sender: std::sync::mpsc::Sender<TaskWorkerCommand>,
    result_receiver: std::sync::mpsc::Receiver<TaskScanResponse>,
    generation: Arc<AtomicU64>,
    next_generation: u64,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl TaskWorker {
    fn new() -> Self {
        let (command_sender, command_receiver) = std::sync::mpsc::channel();
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let worker_generation = Arc::clone(&generation);
        let thread = std::thread::Builder::new().name("ekphos-tasks".to_string()).spawn(move || task_worker_loop(command_receiver, result_sender, worker_generation)).ok();
        Self { command_sender, result_receiver, generation, next_generation: 0, thread }
    }

    /// Queue a scan, superseding any scan still running. Returns the request
    /// generation, or `None` when no worker thread is available.
    fn request(&mut self, root: PathBuf, notes: Vec<Note>) -> Option<u64> {
        if self.thread.as_ref().is_none_or(std::thread::JoinHandle::is_finished) {
            return None;
        }
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        let generation = self.next_generation;
        self.generation.store(generation, Ordering::Release);
        self.command_sender.send(TaskWorkerCommand::Scan(Box::new(TaskScanRequest { generation, root, notes }))).ok()?;
        Some(generation)
    }

    fn poll(&self) -> Option<TaskScanResponse> {
        self.result_receiver.try_iter().last()
    }

    fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

impl Drop for TaskWorker {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.command_sender.send(TaskWorkerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn task_worker_loop(receiver: std::sync::mpsc::Receiver<TaskWorkerCommand>, sender: std::sync::mpsc::Sender<TaskScanResponse>, generation: Arc<AtomicU64>) {
    while let Ok(command) = receiver.recv() {
        let TaskWorkerCommand::Scan(mut request) = command else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(TaskWorkerCommand::Scan(newer)) => request = newer,
                Ok(TaskWorkerCommand::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        let Some(tasks) = collect_all_tasks(&request.root, &request.notes, request.generation, &generation) else {
            continue;
        };
        if sender.send(TaskScanResponse { generation: request.generation, tasks }).is_err() {
            return;
        }
    }
}

fn collect_all_tasks(root: &std::path::Path, notes: &[Note], request_generation: u64, generation: &AtomicU64) -> Option<Vec<Task>> {
    let mut tasks = Vec::new();
    for note in notes {
        if generation.load(Ordering::Acquire) != request_generation {
            return None;
        }
        if !note.kind.is_markdown() {
            continue;
        }
        let Some(path) = note.file_path.as_ref() else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let Ok(body) = std::fs::read_to_string(path) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        tasks.extend(ekphos_tasks::collect_tasks(&body, note.id, &relative, &note.title));
    }
    (generation.load(Ordering::Acquire) == request_generation).then_some(tasks)
}

impl App {
    pub fn today(&self) -> chrono::NaiveDate {
        self.dependencies.clock.today()
    }

    pub fn tasks_loading(&self) -> bool {
        self.tasks.loading
    }

    pub fn mark_tasks_dirty(&mut self) {
        self.tasks.dirty = true;
    }

    /// Open the aggregate task view and schedule a fresh scan.
    pub fn open_task_view(&mut self) {
        self.tasks.text_input_active = false;
        self.tasks.dirty = true;
        self.state.dialog = DialogState::TaskView;
    }

    pub fn close_task_view(&mut self) {
        self.tasks.text_input_active = false;
        self.state.dialog = DialogState::None;
    }

    /// Re-run the task filters after a filter or query change.
    pub fn refilter_tasks(&mut self) {
        let today = self.today();
        self.tasks.apply_filters(today);
    }

    /// Advance one filter to its next value, or focus the search box.
    pub fn cycle_task_filter(&mut self, kind: TaskFilterKind) {
        match kind {
            TaskFilterKind::Status => self.tasks.status = self.tasks.status.next(),
            TaskFilterKind::Due => self.tasks.due = self.tasks.due.next(),
            TaskFilterKind::Priority => self.tasks.priority = self.tasks.priority.next(),
            TaskFilterKind::Search => {
                self.tasks.text_input_active = true;
                return;
            }
        }
        self.refilter_tasks();
    }

    /// Reset every filter and the search query.
    pub fn clear_task_filters(&mut self) {
        self.tasks.status = TaskStatusFilter::Open;
        self.tasks.due = TaskDueFilter::Any;
        self.tasks.priority = TaskPriorityFilter::Any;
        self.tasks.query.clear();
        self.tasks.text_input_active = false;
        self.refilter_tasks();
    }

    fn task_scan_signature(&self) -> (u64, u64) {
        (self.vault.catalog_generation, self.document.document_generation)
    }

    fn request_task_scan(&mut self) {
        self.tasks.dirty = false;
        self.tasks.signature = self.task_scan_signature();
        match self.tasks.worker.request(self.vault.root().to_path_buf(), self.vault.notes.clone()) {
            Some(generation) => {
                self.tasks.request_generation = generation;
                self.tasks.loading = true;
            }
            None => {
                self.tasks.loading = false;
                self.tasks.scanned_once = true;
                self.show_error_toast("Task scanner is unavailable");
            }
        }
    }

    /// Request and drain task scans for the open task view. Returns true when
    /// new results arrived.
    pub(crate) fn poll_task_scan(&mut self) -> bool {
        if self.state.dialog != DialogState::TaskView {
            return false;
        }
        let changed = self.tasks.dirty || self.tasks.signature != self.task_scan_signature();
        if changed || (!self.tasks.loading && !self.tasks.scanned_once) {
            self.request_task_scan();
        }
        let Some(response) = self.tasks.worker.poll() else {
            return false;
        };
        if response.generation != self.tasks.request_generation {
            return false;
        }
        self.tasks.tasks = response.tasks;
        self.tasks.loading = false;
        self.tasks.scanned_once = true;
        self.refilter_tasks();
        true
    }

    pub fn task_move_selection(&mut self, delta: isize) {
        self.tasks.move_selection(delta);
    }

    pub fn task_select(&mut self, position: usize) {
        self.tasks.select(position);
    }

    pub fn task_select_first(&mut self) {
        self.tasks.select(0);
    }

    pub fn task_select_last(&mut self) {
        let last = self.tasks.visible.len().saturating_sub(1);
        self.tasks.select(last);
    }

    /// Toggle the checkbox of the selected task, writing through to the source
    /// note on disk and scheduling a rescan. The line is verified against the
    /// scanned task before it is rewritten so a note edited since the scan is
    /// never corrupted.
    pub fn toggle_task_from_view(&mut self) {
        let Some(task) = self.tasks.selected_task().cloned() else {
            return;
        };
        let today = self.today();
        if self.document.active_note_id == Some(task.note_id) {
            let matching = self.document.content_items.iter().position(|item| match item {
                ContentItem::TaskItem { source_line, checked, .. } => *source_line as usize == task.source_line && *checked == task.checked,
                _ => false,
            });
            match matching {
                Some(item_index) => self.toggle_task_at(item_index),
                None => self.show_error_toast("Task moved since the last scan, refreshing"),
            }
            self.tasks.dirty = true;
            return;
        }
        let Some(path) = self.vault.notes.iter().find(|note| note.id == task.note_id).and_then(|note| note.file_path.clone()) else {
            self.show_error_toast("The task's note is no longer in the vault");
            self.tasks.dirty = true;
            return;
        };
        let Ok(mut body) = std::fs::read_to_string(&path) else {
            self.show_error_toast("Could not read the task's note");
            self.tasks.dirty = true;
            return;
        };
        let Some(range) = ekphos_tasks::locate_task_line(&body, task.source_line, &task.text) else {
            self.show_error_toast("Task moved since the last scan, refreshing");
            self.tasks.dirty = true;
            return;
        };
        let Some(updated) = ekphos_tasks::set_checked(&body[range.clone()], !task.checked, today) else {
            self.tasks.dirty = true;
            return;
        };
        body.replace_range(range, &updated);
        if let Err(error) = ekphos_vault::save_note(&path, &body) {
            self.show_error_toast(format!("Could not save note: {error}"));
            return;
        }
        self.vault.body_cache.invalidate(task.note_id);
        self.invalidate_graph_service();
        self.tasks.dirty = true;
    }

    /// Open the selected task's source note at the task's line.
    pub fn open_task_source(&mut self) {
        let Some(task) = self.tasks.selected_task().cloned() else {
            return;
        };
        let Some(note_idx) = self.note_index_for_id(task.note_id) else {
            self.show_error_toast("The task's note is no longer in the vault");
            self.tasks.dirty = true;
            return;
        };
        if !self.navigate_to_note(note_idx) {
            return;
        }
        if let Some(item_index) = self.document.content_items.iter().position(|item| item.source_line() == task.source_line) {
            self.document.content_cursor = item_index;
            let viewport = self.state.content_area.height.saturating_sub(2) as usize;
            self.document.content_scroll_offset = item_index.saturating_sub(viewport / 2);
        }
        self.close_task_view();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn task(text: &str, checked: bool, due: Option<NaiveDate>, priority: Option<ekphos_tasks::Priority>) -> Task {
        Task { note_id: NoteId::new(1), path: "a.md".into(), note_title: "A".into(), source_line: 0, indent: 0, text: text.into(), checked, due, start: None, done: None, priority }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()
    }

    fn state_with(count: usize) -> TaskViewState {
        let mut state = TaskViewState { tasks: (0..count).map(|index| task(&format!("task {index}"), false, None, None)).collect(), status: TaskStatusFilter::All, ..Default::default() };
        state.apply_filters(today());
        state
    }

    #[test]
    fn filters_narrow_by_status_due_priority_and_query() {
        let tasks = vec![task("overdue", false, Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), Some(ekphos_tasks::Priority::High)), task("today", false, Some(today()), None), task("done today", true, Some(today()), None), task("plain", false, None, None)];
        let open = filter_task_indices(&tasks, TaskStatusFilter::Open, TaskDueFilter::Any, TaskPriorityFilter::Any, "", today());
        assert_eq!(open, [0, 1, 3]);
        let overdue = filter_task_indices(&tasks, TaskStatusFilter::Open, TaskDueFilter::Overdue, TaskPriorityFilter::Any, "", today());
        assert_eq!(overdue, [0]);
        let due_today = filter_task_indices(&tasks, TaskStatusFilter::All, TaskDueFilter::Today, TaskPriorityFilter::Any, "", today());
        assert_eq!(due_today, [1, 2]);
        let high = filter_task_indices(&tasks, TaskStatusFilter::Open, TaskDueFilter::Any, TaskPriorityFilter::High, "", today());
        assert_eq!(high, [0]);
        let query = filter_task_indices(&tasks, TaskStatusFilter::Open, TaskDueFilter::Any, TaskPriorityFilter::Any, "plain", today());
        assert_eq!(query, [3]);
        let none = filter_task_indices(&tasks, TaskStatusFilter::Open, TaskDueFilter::Any, TaskPriorityFilter::Any, "zzz", today());
        assert!(none.is_empty());
    }

    #[test]
    fn selection_moves_and_clamps_within_visible_rows() {
        let mut state = state_with(5);
        state.move_selection(3);
        assert_eq!(state.selected, 3);
        state.move_selection(10);
        assert_eq!(state.selected, 4);
        state.move_selection(-100);
        assert_eq!(state.selected, 0);
        state.select(99);
        assert_eq!(state.selected, 4);
        let mut empty = state_with(0);
        empty.move_selection(1);
        empty.select(3);
        assert_eq!(empty.selected, 0);
    }

    #[test]
    fn scroll_follows_selection_and_never_leaves_blank_rows() {
        let mut state = state_with(20);
        state.selected = 15;
        state.clamp_scroll(5);
        assert_eq!(state.scroll_offset, 11);
        state.selected = 2;
        state.clamp_scroll(5);
        assert_eq!(state.scroll_offset, 2);
        state.scroll_offset = 50;
        state.selected = 19;
        state.clamp_scroll(5);
        assert_eq!(state.scroll_offset, 15);
        state.clamp_scroll(0);
        assert_eq!(state.scroll_offset, 0);
        let mut few = state_with(3);
        few.scroll_offset = 2;
        few.selected = 2;
        few.clamp_scroll(10);
        assert_eq!(few.scroll_offset, 0);
    }

    #[test]
    fn apply_filters_keeps_selection_in_range_after_shrinking() {
        let mut state = state_with(10);
        state.selected = 9;
        state.scroll_offset = 6;
        state.query = "task 1".into();
        state.apply_filters(today());
        assert_eq!(state.visible, [1]);
        assert_eq!(state.selected, 0);
        assert_eq!(state.scroll_offset, 0);
        assert!(state.has_active_filters());
        assert_eq!(state.selected_task().map(|task| task.text.as_str()), Some("task 1"));
    }
}
