use super::*;
use crate::core::markdown::{CalloutFold, CalloutKind};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockInsertMode {
    Insert,
    Append,
}

/// Severity of a transient [`Toast`] notification, used to pick its accent color.
///
/// `Info`/`Success` round out the notification API for future callers; only
/// `Error` is raised today (see [`App::show_error_toast`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Error,
    Info,
    Success,
}

/// A short-lived, non-blocking notification shown as a floating overlay.
///
/// Toasts are how recoverable errors (e.g. a clipboard read failing) reach the
/// user without writing to stdout/stderr, which would corrupt the TUI.
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub shown_at: std::time::Instant,
}

impl Toast {
    /// How long a toast stays on screen before auto-dismissing.
    const TTL: std::time::Duration = std::time::Duration::from_secs(4);

    pub fn is_expired_at(&self, now: std::time::Instant) -> bool {
        now.saturating_duration_since(self.shown_at) >= Self::TTL
    }
}

#[derive(Debug, Clone)]
pub struct BlockInsertState {
    pub mode: BlockInsertMode,
    pub rows: (usize, usize),
    pub insert_col: usize,
    pub active_row: usize,
    pub start_col: usize,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: NoteId,
    pub kind: crate::vault::VaultFileKind,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub file_size: u64,
    pub modified_time: Option<std::time::SystemTime>,
    pub created_time: Option<std::time::SystemTime>,
    pub frontmatter: Option<CompactFrontmatter>,
    pub content_start_line: usize,
}

#[derive(Debug, Clone)]
pub struct CompactFrontmatter {
    pub tags: Box<[Box<str>]>,
    pub date: Option<Box<str>>,
}

impl From<crate::core::FrontmatterSummary> for CompactFrontmatter {
    fn from(summary: crate::core::FrontmatterSummary) -> Self {
        Self { tags: summary.tags.into_iter().map(String::into_boxed_str).collect(), date: summary.date.map(String::into_boxed_str) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogState {
    None,
    Onboarding,
    CreateDocument(crate::vault::VaultFileKind),
    CreateFolder,
    CreateNoteInFolder,
    DeleteConfirm,
    DeleteFolderConfirm,
    RenameNote,
    RenameFolder,
    Help,
    EmptyDirectory,
    DirectoryNotFound,
    UnsavedChanges,
    CreateWikiNote,
    GraphView,
    TaskView,
    ThemeSelector,
    EditorModeSelector,
    DiagramViewer,
}

/// State for the theme selector modal (opened with Ctrl+T). Live-previews the
/// highlighted theme as the user navigates; the original theme is restored on
/// cancel and the selected one is persisted to config on confirm.
#[derive(Debug, Clone, Default)]
pub struct ThemePicker {
    pub themes: Vec<ThemeEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub style: StyleMode,
    /// Theme name active when the picker was opened, restored on Esc.
    pub original_theme_name: String,
    pub original_style: StyleMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SortMode {
    #[default]
    NameAsc,
    NameDesc,
    ModifiedOldest,
    ModifiedNewest,
    CreatedOldest,
    CreatedNewest,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::ModifiedOldest,
            SortMode::ModifiedOldest => SortMode::ModifiedNewest,
            SortMode::ModifiedNewest => SortMode::CreatedOldest,
            SortMode::CreatedOldest => SortMode::CreatedNewest,
            SortMode::CreatedNewest => SortMode::NameAsc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::NameAsc => "A→Z",
            SortMode::NameDesc => "Z→A",
            SortMode::ModifiedOldest => "Mod↑",
            SortMode::ModifiedNewest => "Mod↓",
            SortMode::CreatedOldest => "Cre↑",
            SortMode::CreatedNewest => "Cre↓",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphViewState {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub selected_node: Option<usize>,
    pub selected_note_index: Option<usize>,
    pub root_note_index: usize,
    pub mode: GraphMode,
    pub depth: usize,
    pub link_scope: GraphLinkScope,
    pub filter_query: String,
    pub filter_draft: String,
    pub filter_before_edit: String,
    pub filter_editing: bool,
    pub show_orphans: bool,
    pub help_visible: bool,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub index_pending: bool,
    pub layout_pending: bool,
    pub global_positions: Vec<(NoteId, f32, f32)>,
    pub global_fingerprint: Option<u64>,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub zoom: f32,
    pub dirty: bool,
    pub drag_start: Option<(u16, u16)>,
    pub is_panning: bool,
    pub dragging_node: Option<usize>,
    pub view_width: f32,
    pub view_height: f32,
    pub graph_area: Rect,
    pub needs_center: bool,
    pub last_click: Option<(std::time::Instant, usize)>,
}

impl Default for GraphViewState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            selected_node: None,
            selected_note_index: None,
            root_note_index: 0,
            mode: GraphMode::Local,
            depth: 1,
            link_scope: GraphLinkScope::All,
            filter_query: String::new(),
            filter_draft: String::new(),
            filter_before_edit: String::new(),
            filter_editing: false,
            show_orphans: true,
            help_visible: false,
            total_nodes: 0,
            total_edges: 0,
            index_pending: false,
            layout_pending: false,
            global_positions: Vec::new(),
            global_fingerprint: None,
            viewport_x: 0.0,
            viewport_y: 0.0,
            zoom: 1.0,
            dirty: true,
            drag_start: None,
            is_panning: false,
            dragging_node: None,
            view_width: 100.0,
            view_height: 50.0,
            graph_area: Rect::default(),
            needs_center: false,
            last_click: None,
        }
    }
}

pub struct DiagramViewerState {
    pub item_index: usize,
    pub source: String,
    pub kind: &'static str,
    pub style: usize,
    pub scene: Option<Arc<crate::diagram::DiagramScene>>,
    pub scene_key: String,
    pub zoom: f32,
    pub center: (f32, f32),
    pub needs_fit: bool,
    pub canvas: Rect,
    pub font_size: (u16, u16),
    pub drag_origin: Option<(u16, u16)>,
    pub last_click: Option<(std::time::Instant, u16, u16)>,
    pub help_visible: bool,
    pub frame: Option<(DiagramFrameKey, SlicedProtocol)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagramFrameKey {
    pub canvas: (u16, u16),
    pub font_size: (u16, u16),
    pub zoom: f32,
    pub center: (f32, f32),
    pub fill: Option<[u8; 3]>,
}

impl DiagramViewerState {
    pub const STYLES: usize = 3;
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 12.0;
    pub const MARGIN: f32 = 16.0;

    pub fn new(item_index: usize, source: String) -> Self {
        let kind = crate::diagram::diagram_kind(&source);
        Self { item_index, source, kind, style: 0, scene: None, scene_key: String::new(), zoom: 1.0, center: (0.0, 0.0), needs_fit: true, canvas: Rect::default(), font_size: (0, 0), drag_origin: None, last_click: None, help_visible: false, frame: None }
    }

    pub fn natural_scale(&self) -> f32 {
        f32::from(self.font_size.1.max(1)) / crate::diagram::SVG_UNITS_PER_ROW
    }

    pub fn canvas_pixels(&self) -> (f32, f32) {
        (f32::from(self.canvas.width) * f32::from(self.font_size.0.max(1)), f32::from(self.canvas.height) * f32::from(self.font_size.1.max(1)))
    }

    pub fn scene_size(&self) -> Option<(f32, f32)> {
        self.scene.as_ref().map(|scene| (scene.width(), scene.height()))
    }

    pub fn fit_zoom(&self) -> f32 {
        let Some((width, height)) = self.scene_size() else {
            return 1.0;
        };
        let (pixel_width, pixel_height) = self.canvas_pixels();
        let scale = (pixel_width / (width + 2.0 * Self::MARGIN)).min(pixel_height / (height + 2.0 * Self::MARGIN));
        (scale / self.natural_scale()).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM)
    }

    pub fn zoom_bounds(&self) -> (f32, f32) {
        let fit = self.fit_zoom();
        ((fit * 0.5).max(Self::MIN_ZOOM), Self::MAX_ZOOM.max(fit))
    }

    pub fn scale(&self) -> f32 {
        self.natural_scale() * self.zoom
    }

    pub fn fit(&mut self) {
        self.zoom = self.fit_zoom();
        if let Some((width, height)) = self.scene_size() {
            self.center = (width / 2.0, height / 2.0);
        }
        self.needs_fit = false;
    }

    pub fn set_zoom(&mut self, zoom: f32, anchor: Option<(f32, f32)>) {
        let (minimum, maximum) = self.zoom_bounds();
        let zoom = zoom.clamp(minimum, maximum);
        let (pixel_width, pixel_height) = self.canvas_pixels();
        let (anchor_x, anchor_y) = anchor.unwrap_or((pixel_width / 2.0, pixel_height / 2.0));
        let old_scale = self.scale();
        let point = (self.center.0 + (anchor_x - pixel_width / 2.0) / old_scale, self.center.1 + (anchor_y - pixel_height / 2.0) / old_scale);
        self.zoom = zoom;
        let new_scale = self.scale();
        self.center = (point.0 - (anchor_x - pixel_width / 2.0) / new_scale, point.1 - (anchor_y - pixel_height / 2.0) / new_scale);
        self.clamp_center();
    }

    pub fn pan_pixels(&mut self, dx: f32, dy: f32) {
        let scale = self.scale();
        self.center = (self.center.0 + dx / scale, self.center.1 + dy / scale);
        self.clamp_center();
    }

    pub fn pan_view_fraction(&mut self, fraction_x: f32, fraction_y: f32) {
        let (pixel_width, pixel_height) = self.canvas_pixels();
        self.pan_pixels(pixel_width * fraction_x, pixel_height * fraction_y);
    }

    pub fn clamp_center(&mut self) {
        let Some((width, height)) = self.scene_size() else {
            return;
        };
        let scale = self.scale();
        let (pixel_width, pixel_height) = self.canvas_pixels();
        let clamp_axis = |center: f32, extent: f32, view: f32| {
            let half = view / scale / 2.0;
            if extent + 2.0 * Self::MARGIN <= 2.0 * half {
                extent / 2.0
            } else {
                center.clamp(half - Self::MARGIN, extent + Self::MARGIN - half)
            }
        };
        self.center = (clamp_axis(self.center.0, width, pixel_width), clamp_axis(self.center.1, height, pixel_height));
    }

    pub fn visible_fraction(&self) -> Option<((f32, f32), (f32, f32))> {
        let (width, height) = self.scene_size()?;
        let scale = self.scale();
        let (pixel_width, pixel_height) = self.canvas_pixels();
        let axis = |center: f32, extent: f32, view: f32| {
            let total = extent + 2.0 * Self::MARGIN;
            let start = ((center - view / scale / 2.0 + Self::MARGIN) / total).clamp(0.0, 1.0);
            let end = ((center + view / scale / 2.0 + Self::MARGIN) / total).clamp(0.0, 1.0);
            (start, end)
        };
        Some((axis(self.center.0, width, pixel_width), axis(self.center.1, height, pixel_height)))
    }

    pub fn frame_key(&self, fill: Option<[u8; 3]>) -> DiagramFrameKey {
        DiagramFrameKey { canvas: (self.canvas.width, self.canvas.height), font_size: self.font_size, zoom: self.zoom, center: self.center, fill }
    }

    pub fn view(&self) -> crate::diagram::DiagramView {
        let scale = self.scale();
        let (pixel_width, pixel_height) = self.canvas_pixels();
        crate::diagram::DiagramView { pixel_width: pixel_width as u32, pixel_height: pixel_height as u32, scale, origin_x: self.center.0 - pixel_width / scale / 2.0, origin_y: self.center.1 - pixel_height / scale / 2.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    Content,
    Outline,
}

#[derive(Debug, Clone)]
pub struct OutlineItem {
    pub level: u8,
    pub source_line: u32,
    pub line: usize,
}

pub struct ImageState {
    pub image: SlicedProtocol,
    pub size: Size,
    pub source_bytes: usize,
    pub last_visible_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct InlineImageRect {
    pub item_index: usize,
    pub selection_index: usize,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Alignment {
    /// Classify a GFM table separator cell (e.g. `:---`, `---:`, `:---:`, `---`)
    /// into its alignment. Any cell without a leading `:` is treated as Left
    /// (matches GFM's default-left convention).
    pub fn from_separator_cell(cell: &str) -> Alignment {
        let t = cell.trim();
        match (t.starts_with(':'), t.ends_with(':')) {
            (true, true) => Alignment::Center,
            (false, true) => Alignment::Right,
            _ => Alignment::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ContentItem {
    TextLine { range: DocumentRange, source_line: u32, heading_level: u8, callout: Option<CalloutKind> },
    Callout { range: DocumentRange, source_line: u32, body_lines: u32, kind: CalloutKind, fold: CalloutFold },
    MathBlock { range: DocumentRange, source_line: u32, end_line: u32, marker: DocumentRange, indent: u16 },
    Image { path: DocumentRange, source_line: u32 },
    Diagram { range: DocumentRange, source_line: u32, end_line: u32 },
    CodeLine { range: DocumentRange, source_line: u32 },
    CodeFence { language: DocumentRange, source_line: u32 },
    TaskItem { text: DocumentRange, checked: bool, source_line: u32, indent: u16, managed: bool },
    TableRow { cells: Box<[DocumentRange]>, table: u32, source_line: u32, is_separator: bool, is_header: bool },
    Details { summary: Option<DocumentRange>, content_lines: Box<[u32]>, source_line: u32 },
    FrontmatterLine { key: DocumentRange, value: DocumentRange, source_line: u32 },
    FrontmatterDelimiter { source_line: u32 },
    TagBadges,
}

impl ContentItem {
    pub fn source_line(&self) -> usize {
        match self {
            Self::TextLine { source_line, .. }
            | Self::Callout { source_line, .. }
            | Self::MathBlock { source_line, .. }
            | Self::Image { source_line, .. }
            | Self::Diagram { source_line, .. }
            | Self::CodeLine { source_line, .. }
            | Self::CodeFence { source_line, .. }
            | Self::TaskItem { source_line, .. }
            | Self::TableRow { source_line, .. }
            | Self::Details { source_line, .. }
            | Self::FrontmatterLine { source_line, .. }
            | Self::FrontmatterDelimiter { source_line } => *source_line as usize,
            Self::TagBadges => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub column_widths: Box<[u16]>,
    pub alignments: Box<[Alignment]>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DocumentLinkRange {
    pub start: u32,
    pub len: u16,
    pub image_count: u16,
}

#[derive(Default)]
pub struct ContentRenderScratch {
    pub item_text_heights: Vec<u16>,
    pub item_height_keys: Vec<Option<u64>>,
    pub constraints: Vec<Constraint>,
    pub visible_indices: Vec<usize>,
    pub height_generation: u64,
    pub height_width: usize,
}
