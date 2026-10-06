mod base_view;
mod canvas_view;
mod content;
mod context_menu;
mod diagram_viewer;
mod dialogs;
mod editor;
mod file_picker;
mod graph_view;
mod outline;
mod panel;
mod search_dialog;
mod sidebar;
mod status_bar;
mod task_view;
mod theme_picker;
mod toast;
mod wiki_autocomplete;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Widget},
    Frame,
};

use crate::app::{App, ContextMenuState, DialogState, Mode, SearchPickerState, WikiAutocompleteState};
use crate::config::Config;
fn main_layout_constraints(zen_mode: bool, sidebar_collapsed: bool, outline_collapsed: bool, sidebar_width_percent: u16, outline_width_percent: u16) -> [Constraint; 3] {
    let sidebar_constraint = if zen_mode {
        Constraint::Length(0)
    } else if sidebar_collapsed || sidebar_width_percent < Config::MINIMIZED_PANEL_WIDTH_PERCENT {
        Constraint::Length(5)
    } else {
        Constraint::Percentage(sidebar_width_percent)
    };
    let outline_constraint = if zen_mode {
        Constraint::Length(0)
    } else if outline_collapsed || outline_width_percent < Config::MINIMIZED_PANEL_WIDTH_PERCENT {
        Constraint::Length(5)
    } else {
        Constraint::Percentage(outline_width_percent)
    };
    [sidebar_constraint, Constraint::Min(20), outline_constraint]
}

pub(crate) use content::content_item_click_col;
pub use content::render_content;
pub use dialogs::{
    render_changelog_dialog, render_create_document_dialog, render_create_folder_dialog, render_create_note_in_folder_dialog, render_create_wiki_note_dialog, render_delete_confirm_dialog, render_delete_folder_confirm_dialog, render_directory_not_found_dialog, render_editor_mode_selector,
    render_empty_directory_dialog, render_help_dialog, render_keybinding_warning, render_onboarding_dialog, render_rename_folder_dialog, render_rename_note_dialog, render_unsaved_changes_dialog, render_welcome_dialog,
};
pub use outline::{render_outline, OutlineView};
pub use sidebar::{render_sidebar, SidebarView};
pub use status_bar::render_status_bar;

pub fn render(f: &mut Frame, app: &mut App) {
    if !app.state.config.transparent_bg {
        let bg = Block::default().style(Style::default().bg(app.state.theme.background));
        bg.render(f.area(), f.buffer_mut());
    }
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main area
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());
    let main_constraints = main_layout_constraints(app.state.zen_mode, app.state.sidebar_collapsed, app.state.outline_collapsed, app.state.config.effective_sidebar_width_percent(), app.state.config.effective_outline_width_percent());
    let chunks = Layout::default().direction(Direction::Horizontal).constraints(main_constraints).split(vertical_chunks[0]);
    let sidebar_area = render_sidebar(f, SidebarView { theme: &app.state.theme, config: &app.state.config, vault: &app.vault, search: &app.search, focus: app.state.focus, mode: app.editor.mode, minimized: app.is_sidebar_minimized() }, chunks[0]);
    match app.editor.mode {
        Mode::Normal => match app.active_document_kind() {
            Some(crate::vault::VaultFileKind::Base) => base_view::render_base_view(f, app, chunks[1]),
            Some(crate::vault::VaultFileKind::Canvas) => canvas_view::render_canvas_view(f, app, chunks[1]),
            Some(crate::vault::VaultFileKind::Markdown) | None => render_content(f, app, chunks[1]),
        },
        Mode::Edit => {
            let layout = editor::editor_layout(app.state.zen_mode, app.state.config.style, chunks[1]);
            app.editor.editor_area = layout.area;
            app.editor.set_view_size(layout.inner_width, layout.inner_height);
            app.update_editor_scroll(layout.inner_height);
            editor::render_editor(f, editor::EditorView { theme: &app.state.theme, config: &app.state.config, editor: &app.editor, editing_mode: app.state.config.editor.mode, keymap: &app.state.keymap, zen_mode: app.state.zen_mode }, layout);
        }
    }
    let outline = render_outline(f, OutlineView { theme: &app.state.theme, config: &app.state.config, document: &app.document, snapshot: app.document(), editor: &app.editor, focus: app.state.focus, minimized: app.is_outline_minimized() }, chunks[2]);
    app.state.sidebar_area = sidebar_area;
    app.state.outline_area = outline.area;
    app.document.outline_state = outline.state;
    render_status_bar(f, app, vertical_chunks[1]);
    match app.state.dialog {
        DialogState::Onboarding => render_onboarding_dialog(f, app),
        DialogState::CreateDocument(kind) => render_create_document_dialog(f, app, kind),
        DialogState::CreateFolder => render_create_folder_dialog(f, app),
        DialogState::CreateNoteInFolder => render_create_note_in_folder_dialog(f, app),
        DialogState::DeleteConfirm => render_delete_confirm_dialog(f, app),
        DialogState::DeleteFolderConfirm => render_delete_folder_confirm_dialog(f, app),
        DialogState::RenameNote => render_rename_note_dialog(f, app),
        DialogState::RenameFolder => render_rename_folder_dialog(f, app),
        DialogState::Help => app.state.help_scroll = render_help_dialog(f, app),
        DialogState::EmptyDirectory => render_empty_directory_dialog(f, app),
        DialogState::DirectoryNotFound => render_directory_not_found_dialog(f, app),
        DialogState::UnsavedChanges => render_unsaved_changes_dialog(f, app),
        DialogState::CreateWikiNote => render_create_wiki_note_dialog(f, app),
        DialogState::GraphView => {
            let action = graph_view::prepare_graph_view(app, f.area());
            let view = &mut app.graph.graph_view;
            view.graph_area = action.area;
            view.view_width = action.view_width;
            view.view_height = action.view_height;
            if let Some((x, y, zoom)) = action.camera {
                view.viewport_x = x;
                view.viewport_y = y;
                view.zoom = zoom;
            }
            if action.clear_dirty {
                view.dirty = false;
            }
            if action.clear_needs_center {
                view.needs_center = false;
            }
            graph_view::render_graph_view(f, app);
        }
        DialogState::TaskView => task_view::render_task_view(f, app),
        DialogState::ThemeSelector => {
            if let Some(picker) = app.state.theme_picker.as_ref() {
                let syntax_label = picker.themes.get(picker.selected).map(|entry| app.syntax_theme_label(&entry.name)).unwrap_or_default();
                let scroll = theme_picker::render_theme_picker(f, theme_picker::ThemePickerView { theme: &app.state.theme, picker, syntax_label: &syntax_label });
                if let (Some(scroll), Some(picker)) = (scroll, app.state.theme_picker.as_mut()) {
                    picker.scroll_offset = scroll;
                }
            }
        }
        DialogState::EditorModeSelector => render_editor_mode_selector(f, app),
        DialogState::DiagramViewer => diagram_viewer::render_diagram_viewer(f, app),
        DialogState::None => {
            if app.state.show_welcome {
                render_welcome_dialog(f, &app.state.theme);
            } else if app.state.show_changelog {
                let action = render_changelog_dialog(f, app);
                app.state.changelog_scroll = action.scroll;
                app.state.changelog_links = action.links;
            }
        }
    }
    if app.editor.mode == Mode::Edit && app.editor.context_menu_state != ContextMenuState::None {
        context_menu::render_context_menu(f, app);
    }
    if app.editor.mode == Mode::Edit && !matches!(app.editor.wiki_autocomplete, WikiAutocompleteState::None) {
        wiki_autocomplete::render_wiki_autocomplete(f, app);
    }
    if app.search.buffer_search.active {
        search_dialog::render_search_dialog(f, app, app.editor.editor_area);
    }
    if !matches!(app.search.search_picker, SearchPickerState::Closed) {
        app.ensure_search_hydrated();
        if let Some(action) = file_picker::render_search_picker(f, file_picker::SearchPickerView { theme: &app.state.theme, keymap: &app.state.keymap, picker: &app.search.search_picker }) {
            app.search.search_picker_area = action.area;
            app.search.search_picker_results_area = action.results_area;
        }
    }
    if app.state.keybinding_warning.is_some() {
        render_keybinding_warning(f, app);
    }
    toast::render_toast(f, app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppDependencies, DialogState};
    use crate::syntax_service::SyntaxServiceStatus;
    use image::{Rgba, RgbaImage};
    use ratatui::layout::Rect;
    use ratatui::{backend::TestBackend, Terminal};
    use ratatui_image::picker::Picker;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    use unicode_width::UnicodeWidthStr;
    static NEXT_GOLDEN_ROOT: AtomicU64 = AtomicU64::new(0);
    struct GoldenApp {
        app: App,
        root: PathBuf,
    }
    impl GoldenApp {
        fn new() -> Self {
            Self::with_content("---\ntags: [golden]\n---\n# Golden fixture\n\nA [[fixture]] link.\n\n- [ ] stable task\n")
        }
        fn with_content(content: &str) -> Self {
            let id = NEXT_GOLDEN_ROOT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!("ekphos-golden-{}-{id}", std::process::id()));
            let vault = root.join("vault");
            fs::create_dir_all(&vault).unwrap();
            fs::write(vault.join("fixture.md"), content).unwrap();
            let config = Config { general: crate::config::GeneralConfig { welcome_shown: false, check_updates: false, ..Default::default() }, ..Default::default() };
            let dependencies = AppDependencies::headless(root.join("config"), root.join("cache"));
            let mut app = App::new_injected(config, vault, None, dependencies);
            app.state.show_welcome = false;
            app.state.dialog = DialogState::None;
            let started = Instant::now();
            while (app.search.indexing_in_progress || app.graph.is_indexing()) && started.elapsed() < Duration::from_secs(5) {
                app.poll_index_build();
                app.poll_graph_workers();
                std::thread::yield_now();
            }
            app.state.config.notes_dir = "/fixture/vault".to_string();
            app.state.input_buffer = "/fixture/vault".to_string();
            if let Some(note) = app.vault.notes.first_mut() {
                note.file_path = Some(PathBuf::from("/fixture/vault/fixture.md"));
            }
            Self { app, root }
        }
        fn hash(&mut self, width: u16, height: u16) -> u64 {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| render(frame, &mut self.app)).unwrap();
            let buffer = terminal.backend().buffer();
            let mut hash = 0xcbf29ce484222325u64;
            for y in 0..height {
                for x in 0..width {
                    for byte in buffer[(x, y)].symbol().as_bytes() {
                        hash ^= u64::from(*byte);
                        hash = hash.wrapping_mul(0x100000001b3);
                    }
                }
                hash ^= u64::from(b'\n');
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash
        }
    }
    impl Drop for GoldenApp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn golden_main_view_100x30() {
        let mut fixture = GoldenApp::new();
        assert_eq!(fixture.hash(100, 30), 2_737_915_595_076_798_973);
    }

    #[test]
    fn phase11_terminal_goldens_cover_tiny_narrow_normal_and_wide_sizes() {
        let sizes = [(20, 8), (60, 18), (100, 30), (160, 50)];
        let actual: Vec<_> = sizes
            .into_iter()
            .map(|(width, height)| {
                let mut fixture = GoldenApp::new();
                fixture.hash(width, height)
            })
            .collect();
        assert_eq!(actual, [11_602_399_305_202_422_691, 4_212_271_247_269_996_847, 2_737_915_595_076_798_973, 8_070_470_284_126_182_397]);
    }

    #[test]
    fn golden_edit_view_80x24() {
        let mut fixture = GoldenApp::new();
        fixture.app.enter_edit_mode();
        assert_eq!(fixture.hash(80, 24), 15_822_003_958_405_314_542);
    }

    #[test]
    fn preview_formats_nested_unordered_markers_as_bullets() {
        let mut fixture = GoldenApp::with_content("- parent\n\t- tab child\n    * space child\n\t+ plus child\n");
        let buffer = draw(&mut fixture, 100, 20);
        let content = (0..buffer.area.height).map(|y| row_text(&buffer, y)).collect::<Vec<_>>().join("\n");

        assert!(content.contains("• parent"), "{content}");
        assert!(content.contains("    • tab child"), "{content}");
        assert!(content.contains("    • space child"), "{content}");
        assert!(content.contains("    • plus child"), "{content}");
        assert!(!content.contains("- tab child"), "{content}");
    }

    #[test]
    fn preview_renders_obsidian_callouts_and_toggles_foldable_ones() {
        let mut fixture = GoldenApp::with_content("> [!warning]- Mind the **gap**\n> Hidden body\n\n> [!tip]\n> Shown body\n> - listed\n>\n> > nested quote\n\nAfter\n");
        let render = |fixture: &mut GoldenApp| {
            let buffer = draw(fixture, 100, 20);
            (0..buffer.area.height).map(|y| row_text(&buffer, y)).collect::<Vec<_>>().join("\n")
        };
        let content = render(&mut fixture);
        assert!(content.contains("┃ ⚠ Mind the gap ▸"), "{content}");
        assert!(!content.contains("Hidden body"), "{content}");
        assert!(content.contains("┃ ✦ Tip "), "{content}");
        assert!(content.contains("┃ Shown body"), "{content}");
        assert!(content.contains("┃ • listed"), "{content}");
        assert!(content.contains("┃ ┃ nested quote"), "{content}");
        assert!(!content.contains("[!"), "{content}");
        assert!(!fixture.app.is_callout_foldable_at(3));

        fixture.app.toggle_callout_fold_at(0);
        let content = render(&mut fixture);
        assert!(content.contains("┃ ⚠ Mind the gap ▾"), "{content}");
        assert!(content.contains("┃ Hidden body"), "{content}");
    }

    #[test]
    fn preview_connects_only_nested_tasks_and_keeps_them_toggleable() {
        let mut fixture = GoldenApp::with_content("- [ ] parent\n    - [ ] first child\n    - [ ] second child\n        - [ ] grandchild\n");
        let buffer = draw(&mut fixture, 100, 20);
        let content = (0..buffer.area.height).map(|y| row_text(&buffer, y)).collect::<Vec<_>>().join("\n");

        assert!(content.contains("[ ] parent"), "{content}");
        assert!(content.contains("├── [ ] first child"), "{content}");
        assert!(content.contains("└── [ ] second child"), "{content}");
        assert!(content.contains("    └── [ ] grandchild"), "{content}");
        assert!(!content.contains("└── [ ] parent"), "{content}");

        let content_x = fixture.app.state.content_area.x;
        assert!(fixture.app.is_click_on_task_checkbox(1, content_x + 6, content_x));
        let note_path = fixture.root.join("vault").join("fixture.md");
        if let Some(note) = fixture.app.vault.notes.first_mut() {
            note.file_path = Some(note_path);
        }
        fixture.app.toggle_task_at(1);
        assert!(fixture.app.document.content_items.get(1).is_some_and(|item| matches!(item, crate::app::ContentItem::TaskItem { checked: true, .. })));
    }

    fn flat_fixture() -> GoldenApp {
        let mut fixture = GoldenApp::new();
        fixture.app.state.config.style = crate::config::StyleMode::Flat;
        fixture.app.update_editor_block();
        fixture
    }

    #[test]
    fn theme_selector_previews_confirms_and_cancels_style() {
        let mut fixture = GoldenApp::new();
        fixture.app.open_theme_selector();
        assert_eq!(fixture.app.state.dialog, DialogState::ThemeSelector);
        fixture.app.theme_selector_toggle_style();
        assert_eq!(fixture.app.state.config.style, crate::config::StyleMode::Flat);
        fixture.app.cancel_theme_selection();
        assert_eq!(fixture.app.state.config.style, crate::config::StyleMode::Outlined);
        assert_eq!(fixture.app.state.dialog, DialogState::None);
        fixture.app.open_theme_selector();
        fixture.app.theme_selector_toggle_style();
        fixture.app.confirm_theme_selection();
        assert_eq!(fixture.app.state.config.style, crate::config::StyleMode::Flat);
        let saved = fs::read_to_string(fixture.root.join("config").join("config.toml")).unwrap();
        assert!(saved.contains("style = \"flat\""), "{saved}");
    }

    fn select_theme(fixture: &mut GoldenApp, name: &str) {
        let picker = fixture.app.state.theme_picker.as_mut().unwrap();
        let index = picker.themes.iter().position(|entry| entry.name == name).unwrap();
        picker.selected = (index + picker.themes.len() - 1) % picker.themes.len();
        fixture.app.theme_selector_select_next();
    }

    fn wait_for_highlighter(fixture: &mut GoldenApp) {
        fixture.app.ensure_highlighter();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !fixture.app.poll_highlighter() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(fixture.app.syntax_service_status(), SyntaxServiceStatus::Ready);
    }

    fn active_syntax(fixture: &GoldenApp) -> &str {
        fixture.app.state.syntax_service.active_theme().unwrap()
    }

    #[test]
    fn theme_selector_auto_pairs_syntax_with_theme_lightness() {
        let mut fixture = GoldenApp::new();
        wait_for_highlighter(&mut fixture);
        assert_eq!(active_syntax(&fixture), "base16-eighties.dark");
        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "gruvbox-light-hard");
        assert_eq!(active_syntax(&fixture), "InspiredGitHub");
        fixture.app.cancel_theme_selection();
        assert_eq!(active_syntax(&fixture), "base16-eighties.dark");
        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "catppuccin-latte");
        fixture.app.confirm_theme_selection();
        assert_eq!(active_syntax(&fixture), "InspiredGitHub");
        assert_eq!(fixture.app.state.config.syntax_theme, "auto");
        assert!(fixture.app.state.config.syntax_themes.is_empty());
        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "dracula");
        fixture.app.confirm_theme_selection();
        assert_eq!(active_syntax(&fixture), "base16-eighties.dark");
        assert!(fixture.app.state.toast.is_none());
    }

    #[test]
    fn theme_selector_syntax_choice_previews_persists_per_theme_and_returns_to_auto() {
        let mut fixture = GoldenApp::new();
        wait_for_highlighter(&mut fixture);
        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "gruvbox-light");
        assert_eq!(fixture.app.syntax_theme_label("gruvbox-light"), "Auto (InspiredGitHub)");
        fixture.app.theme_selector_cycle_syntax(true);
        fixture.app.theme_selector_cycle_syntax(true);
        assert_eq!(active_syntax(&fixture), "Solarized (dark)");
        assert_eq!(fixture.app.syntax_theme_label("gruvbox-light"), "Solarized (dark)");
        fixture.app.cancel_theme_selection();
        assert!(fixture.app.state.config.syntax_themes.is_empty());
        assert_eq!(active_syntax(&fixture), "base16-eighties.dark");

        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "gruvbox-light");
        fixture.app.theme_selector_cycle_syntax(false);
        assert_eq!(active_syntax(&fixture), "base16-ocean.light");
        fixture.app.confirm_theme_selection();
        assert_eq!(fixture.app.state.config.syntax_themes.get("gruvbox-light").map(String::as_str), Some("base16-ocean.light"));
        let saved = fs::read_to_string(fixture.root.join("config").join("config.toml")).unwrap();
        assert!(saved.contains("[syntax_themes]") && saved.contains("gruvbox-light = \"base16-ocean.light\""), "{saved}");

        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "dracula");
        assert_eq!(active_syntax(&fixture), "base16-eighties.dark");
        select_theme(&mut fixture, "gruvbox-light");
        assert_eq!(active_syntax(&fixture), "base16-ocean.light");
        fixture.app.theme_selector_cycle_syntax(true);
        assert_eq!(active_syntax(&fixture), "InspiredGitHub");
        assert_eq!(fixture.app.syntax_theme_label("gruvbox-light"), "Auto (InspiredGitHub)");
        fixture.app.confirm_theme_selection();
        assert!(fixture.app.state.config.syntax_themes.is_empty());
    }

    #[test]
    fn theme_selector_renders_the_syntax_row_and_hint() {
        let mut fixture = GoldenApp::new();
        wait_for_highlighter(&mut fixture);
        fixture.app.open_theme_selector();
        select_theme(&mut fixture, "gruvbox-light");
        let buffer = draw(&mut fixture, 100, 30);
        let screen: Vec<String> = (0..30).map(|y| (0..100).map(|x| buffer[(x, y)].symbol().to_string()).collect()).collect();
        assert!(screen.iter().any(|row| row.contains("Syntax  Auto (InspiredGitHub)")), "{}", screen.join("\n"));
        assert!(screen.iter().any(|row| row.contains("s syntax")), "{}", screen.join("\n"));
    }

    #[test]
    fn unknown_syntax_theme_is_reported_instead_of_silently_replaced() {
        let mut fixture = GoldenApp::new();
        wait_for_highlighter(&mut fixture);
        assert!(fixture.app.state.toast.is_none());
        fixture.app.state.config.syntax_theme = "base16-ocean.drak".to_string();
        fixture.app.open_theme_selector();
        fixture.app.confirm_theme_selection();
        let toast = fixture.app.state.toast.as_ref().expect("unknown syntax theme toast");
        assert!(toast.message.contains("base16-ocean.drak"), "{}", toast.message);
    }

    #[test]
    fn flat_style_applies_to_full_screen_views() {
        let mut fixture = flat_fixture();
        fixture.app.state.dialog = DialogState::TaskView;
        let buffer = draw(&mut fixture, 80, 20);
        let row0: String = (0..80).map(|x| buffer[(x, 0)].symbol().to_string()).collect();
        assert!(!row0.contains('┌'), "{row0}");
        assert!(row0.contains("TASKS"), "{row0}");
        assert_eq!(fixture.app.tasks.list_area.x, 1);
        assert_eq!(fixture.app.tasks.list_area.y, 3);
        fixture.app.build_graph();
        fixture.app.state.dialog = DialogState::GraphView;
        let buffer = draw(&mut fixture, 80, 20);
        let row0: String = (0..80).map(|x| buffer[(x, 0)].symbol().to_string()).collect();
        assert!(!row0.contains('┌'), "{row0}");
        assert!(row0.contains("GRAPH"), "{row0}");
        let last: String = (0..80).map(|x| buffer[(x, 19)].symbol().to_string()).collect();
        assert!(!last.contains('└'), "{last}");
        assert_eq!(fixture.app.graph.graph_view.graph_area.y, 2);
        assert_eq!(fixture.app.graph.graph_view.graph_area.height, 20 - 1 - 1 - 2);
    }

    #[test]
    fn golden_main_view_flat_100x30() {
        let mut fixture = flat_fixture();
        assert_eq!(fixture.hash(100, 30), 9_558_837_033_891_136_885);
    }

    #[test]
    fn golden_edit_view_flat_80x24() {
        let mut fixture = flat_fixture();
        fixture.app.enter_edit_mode();
        assert_eq!(fixture.hash(80, 24), 4_518_287_944_179_168_489);
    }

    #[test]
    fn flat_style_drops_borders_and_marks_focus_with_a_bar() {
        let mut fixture = flat_fixture();
        let buffer = draw(&mut fixture, 100, 30);
        let sidebar = fixture.app.state.sidebar_area;
        let content = fixture.app.state.content_area;
        let row0: String = (0..100).map(|x| buffer[(x, 0)].symbol().to_string()).collect();
        assert!(!row0.contains('┌') && !row0.contains('─'), "{row0}");
        assert_eq!(buffer[(sidebar.x, sidebar.y)].symbol(), "▌");
        assert_eq!(buffer[(sidebar.x, sidebar.y + sidebar.height - 1)].symbol(), "▌");
        assert_ne!(buffer[(content.x.saturating_sub(1), content.y)].symbol(), "▌");
        assert_eq!(content.y, sidebar.y + 1);
        fixture.app.enter_edit_mode();
        let buffer = draw(&mut fixture, 100, 30);
        let editor = fixture.app.editor.editor_area;
        assert_eq!(buffer[(editor.x, editor.y)].symbol(), "▌");
        assert_eq!(buffer[(editor.x, editor.bottom() - 1)].symbol(), "▌");
        assert_ne!(buffer[(sidebar.x, sidebar.y)].symbol(), "▌");
    }

    fn task_view_fixture(content: &str) -> GoldenApp {
        let mut fixture = GoldenApp::with_content(content);
        if let Some(note) = fixture.app.vault.notes.first_mut() {
            note.file_path = Some(fixture.root.join("vault").join("fixture.md"));
        }
        fixture.app.open_task_view();
        let started = Instant::now();
        while (fixture.app.tasks_loading() || !fixture.app.tasks.scanned_once()) && started.elapsed() < Duration::from_secs(5) {
            fixture.app.poll_background();
            std::thread::yield_now();
        }
        fixture
    }

    fn draw(fixture: &mut GoldenApp, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn draw_changelog(fixture: &mut GoldenApp, version: &str, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let action = super::dialogs::render_changelog_dialog_for_version(frame, &fixture.app, version);
                fixture.app.state.changelog_scroll = action.scroll;
                fixture.app.state.changelog_links = action.links;
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn row_text(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..buffer.area.width).map(|x| buffer[(x, y)].symbol().to_string()).collect()
    }

    fn column_of(buffer: &ratatui::buffer::Buffer, y: u16, needle: &str) -> Option<u16> {
        (0..buffer.area.width).find(|&x| buffer[(x, y)].symbol() == needle)
    }

    #[test]
    fn task_view_aggregates_tasks_with_metadata() {
        let mut fixture = task_view_fixture("- [ ] #task alpha 📅 2026-06-01 ⏫\n- [ ] regular checklist\nplain line\n- [x] #task beta ✅ 2026-01-01\n");
        assert_eq!(fixture.app.tasks.tasks.len(), 2);
        assert_eq!(fixture.app.tasks.visible.len(), 1);
        fixture.app.tasks.status = crate::app::TaskStatusFilter::All;
        fixture.app.refilter_tasks();
        let buffer = draw(&mut fixture, 80, 24);
        let content: String = (0..24).map(|y| row_text(&buffer, y)).collect();
        assert!(content.contains("TASKS"), "{content}");
        assert!(content.contains("1 open · 2 total"), "{content}");
        assert!(content.contains("[ ] alpha"), "{content}");
        assert!(content.contains("[x] beta"), "{content}");
        assert!(!content.contains("regular checklist"), "{content}");
        assert!(!content.contains("#task"), "{content}");
        assert!(content.contains("⏫"), "{content}");
        assert!(content.contains("2026-06-01"), "{content}");
        assert!(content.contains("✅"), "{content}");
        assert!(!content.contains("alpha 📅"), "metadata tokens must not repeat in the text column: {content}");
        for y in 5..22 {
            let row = row_text(&buffer, y);
            assert_eq!(row.trim_matches('│').trim(), "", "the editor must not bleed through the task view at row {y}: {row}");
        }
        assert_eq!(fixture.app.tasks.row_hits.len(), 2);
        assert_eq!(fixture.app.tasks.filter_hits.len(), 4);
    }

    #[test]
    fn task_view_columns_align_across_glyph_widths_and_selection_fills_the_row() {
        let mut fixture = task_view_fixture("- [ ] #task wide 📅 2026-06-01 ⏫\n- [ ] #task plain\n- [ ] #task 日本語のタスク 🔼\n");
        fixture.app.tasks.selected = 1;
        let buffer = draw(&mut fixture, 80, 24);
        let rows: Vec<u16> = (0..24).filter(|&y| row_text(&buffer, y).contains("[ ]")).collect();
        assert_eq!(rows.len(), 3, "{}", (0..24).map(|y| row_text(&buffer, y)).collect::<String>());
        let note_columns: Vec<Option<u16>> = rows.iter().map(|&y| column_of(&buffer, y, "f")).collect();
        assert!(note_columns.iter().all(|column| column.is_some() && *column == note_columns[0]), "note column drifted: {note_columns:?}");
        let selected_row = rows[1];
        let selection = fixture.app.state.theme.selection;
        let mut x = 1;
        while x < buffer.area.width - 1 {
            let cell = &buffer[(x, selected_row)];
            assert_eq!(cell.bg, selection, "selection background missing at column {x}");
            x += cell.symbol().width().max(1) as u16;
        }
        assert_ne!(buffer[(2, rows[0])].bg, selection);
    }

    #[test]
    fn task_view_scrolls_to_the_selection_and_drops_columns_when_narrow() {
        let content: String = (0..40).map(|index| format!("- [ ] #task filler task number {index} 📅 2026-06-01\n")).collect();
        let mut fixture = task_view_fixture(&content);
        assert_eq!(fixture.app.tasks.visible.len(), 40);
        fixture.app.task_select_last();
        let buffer = draw(&mut fixture, 100, 14);
        let content: String = (0..14).map(|y| row_text(&buffer, y)).collect();
        assert!(content.contains("filler task number 39"), "{content}");
        assert!(content.contains("40/40"), "{content}");
        assert!(content.contains("┃"), "scrollbar thumb missing: {content}");
        assert_eq!(fixture.app.tasks.row_hits.len(), 9);
        assert_eq!(fixture.app.tasks.scroll_offset, 31);
        let narrow = draw(&mut fixture, 30, 6);
        let narrow_content: String = (0..6).map(|y| row_text(&narrow, y)).collect();
        assert!(narrow_content.contains("[ ] filler"), "{narrow_content}");
        assert!(!narrow_content.contains("2026-06-01"), "date column should be dropped when narrow: {narrow_content}");
        assert!(!narrow_content.contains("p prior"), "partial chips should be hidden: {narrow_content}");
    }

    #[test]
    fn task_view_survives_tiny_geometry_and_shows_empty_states() {
        let mut fixture = task_view_fixture("- [ ] #task only\n");
        for (width, height) in [(1, 1), (2, 2), (5, 3), (6, 3), (12, 4), (20, 3)] {
            let _ = draw(&mut fixture, width, height);
        }
        fixture.app.tasks.query = "zzz".into();
        fixture.app.refilter_tasks();
        let buffer = draw(&mut fixture, 60, 10);
        let content: String = (0..10).map(|y| row_text(&buffer, y)).collect();
        assert!(content.contains("No tasks match the current filters"), "{content}");
        assert!(content.contains("clear the filters"), "{content}");
        assert!(fixture.app.tasks.row_hits.is_empty());
        assert!(content.contains("0/0"), "{content}");
    }

    #[test]
    fn tiny_edit_geometry_and_mouse_auto_scroll_do_not_panic() {
        let mut fixture = GoldenApp::new();
        fixture.app.enter_edit_mode();
        fixture.app.editor.set_line_wrap(false);
        let backend = TestBackend::new(2, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let layout = editor::EditorLayout { area, inner_width: 0, inner_height: 0 };
                fixture.app.editor.editor_area = area;
                fixture.app.editor.set_view_size(0, 0);
                editor::render_editor(frame, editor::EditorView { theme: &fixture.app.state.theme, config: &fixture.app.state.config, editor: &fixture.app.editor, editing_mode: fixture.app.state.config.editor.mode, keymap: &fixture.app.state.keymap, zen_mode: false }, layout);
            })
            .unwrap();

        fixture.app.editor.editor_area = Rect::new(0, 0, 1, 1);
        assert_eq!(fixture.app.get_auto_scroll_direction(0), 0);
    }

    #[test]
    fn golden_document_snapshot_unicode_tables_links_and_inline_images() {
        let mut fixture = GoldenApp::with_content(
            "---\ntags: [golden, phase6]\ndate: 2026-08-21\n---\n# ASCII and\ttabs\n\nCombining e\u{301}, CJK 日本語, emoji 😀, and a [wide link 開く](https://example.test).\n\nA deliberately long wrapping line keeps ASCII, e\u{301}, 日本語, and 😀 coordinates stable across terminal rows.\n\n- [ ] task with [[fixture|wiki alias]]\n\n| left | centered 日本 | right 😀 |\n|:-----|:-------------:|---------:|\n| e\u{301} | [開く](https://example.test/table) | tabs\there |\n\nText before ![inline](missing.png) and after.\n",
        );
        assert_eq!(fixture.hash(100, 36), 1_420_924_652_973_427_897);
    }

    #[test]
    fn syntect_loads_only_for_a_visible_language_block_and_document_eviction_clears_results() {
        let mut content = String::from("# Lazy syntax\n\n");
        for line in 0..80 {
            content.push_str(&format!("plain line {line}\n"));
        }
        content.push_str("```rust\nfn main() { println!(\"visible\"); }\n```\n");
        let mut fixture = GoldenApp::with_content(&content);
        fixture.hash(70, 12);
        assert_eq!(fixture.app.syntax_service_status(), SyntaxServiceStatus::Unloaded);
        assert_eq!(fixture.app.memory_snapshot().syntax_definition_bytes, 0);
        fixture.app.document.content_cursor = fixture.app.document.content_items.iter().position(|item| matches!(item, crate::app::ContentItem::CodeLine { .. })).unwrap();
        fixture.app.state.focus = crate::app::Focus::Content;
        fixture.hash(70, 12);
        assert_eq!(fixture.app.syntax_service_status(), SyntaxServiceStatus::Loading);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !fixture.app.poll_highlighter() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(fixture.app.syntax_service_status(), SyntaxServiceStatus::Ready);
        fixture.hash(70, 12);
        let loaded = fixture.app.memory_snapshot();
        assert!(loaded.syntax_definition_bytes > 0);
        assert!(loaded.syntax_result_cache_bytes > 0);
        fixture.app.enter_edit_mode();
        let evicted = fixture.app.memory_snapshot();
        assert_eq!(evicted.syntax_definition_bytes, loaded.syntax_definition_bytes);
        assert_eq!(evicted.syntax_result_cache_bytes, 0);
    }

    #[test]
    fn image_protocols_are_viewport_scoped_and_missing_protocol_falls_back_safely() {
        let id = NEXT_GOLDEN_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("ekphos-image-lifecycle-{}-{id}", std::process::id()));
        let vault = root.join("vault");
        fs::create_dir_all(&vault).unwrap();
        let image_path = root.join("fixture.png");
        RgbaImage::from_pixel(32, 24, Rgba([20, 80, 160, 255])).save(&image_path).unwrap();
        let mut note = format!("# Images\n\n![fixture]({})\n", image_path.display());
        for line in 0..80 {
            note.push_str(&format!("plain line {line}\n"));
        }
        fs::write(vault.join("fixture.md"), note).unwrap();
        let config = Config { general: crate::config::GeneralConfig { welcome_shown: false, check_updates: false, ..Default::default() }, ..Default::default() };
        let dependencies = AppDependencies::headless(root.join("config"), root.join("cache"));
        let mut app = App::new_injected(config, vault, None, dependencies);
        app.images.picker = Some(Picker::halfblocks());
        app.state.focus = crate::app::Focus::Content;
        app.document.content_cursor = app.document.content_items.iter().position(|item| matches!(item, crate::app::ContentItem::Image { .. })).unwrap();
        let backend = TestBackend::new(80, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.image_has_background_work() && Instant::now() < deadline {
            app.poll_pending_images();
            std::thread::yield_now();
        }
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert_eq!(app.images.image_states.len(), 1);
        assert!(app.memory_snapshot().image_protocol_bytes > 0);
        app.document.content_cursor = app.document.content_items.len() - 1;
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(app.images.image_states.is_empty());
        assert_eq!(app.memory_snapshot().image_protocol_bytes, 0);
        app.images.picker = None;
        app.document.content_cursor = app.document.content_items.iter().position(|item| matches!(item, crate::app::ContentItem::Image { .. })).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(app.images.image_states.is_empty());
        assert!(app.memory_snapshot().image_decoded_bytes > 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn math_renders_without_content_focus_and_keeps_a_readable_terminal_fallback() {
        let mut fixture = GoldenApp::with_content("# Math\n\nInline $E = mc^2$ stays in the prose.\n\n$$\n\\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}\n$$\n");
        assert_eq!(fixture.app.state.focus, crate::app::Focus::Sidebar);
        fixture.app.document.content_cursor = fixture.app.document.content_items.iter().position(|item| matches!(item, crate::app::ContentItem::MathBlock { .. })).unwrap();

        let backend = TestBackend::new(80, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let fallback = terminal.backend().buffer();
        let fallback_symbols = (0..fallback.area.height).flat_map(|y| (0..fallback.area.width).map(move |x| fallback[(x, y)].symbol())).collect::<String>();
        assert!(fallback_symbols.contains('∑'), "{fallback_symbols}");
        assert!(fallback_symbols.contains("\\frac"), "{fallback_symbols}");

        fixture.app.images.picker = Some(Picker::halfblocks());
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while fixture.app.image_has_background_work() && Instant::now() < deadline {
            fixture.app.poll_pending_images();
            std::thread::yield_now();
        }
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        assert_eq!(fixture.app.images.image_states.len(), 2);
        assert!(fixture.app.images.image_states.keys().any(|key| key.starts_with("math:block:")));
        assert!(fixture.app.images.image_states.keys().any(|key| key.starts_with("math:inline:")));
        let memory = fixture.app.memory_snapshot();
        assert!(memory.image_decoded_bytes > 0);
        assert!(memory.image_protocol_bytes > 0);

        fixture.app.images.picker = None;
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        assert!(fixture.app.images.image_states.is_empty());
    }

    fn settle_images(fixture: &mut GoldenApp, terminal: &mut Terminal<TestBackend>) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
            if !fixture.app.image_has_background_work() || Instant::now() > deadline {
                break;
            }
            while fixture.app.image_has_background_work() && Instant::now() < deadline {
                fixture.app.poll_pending_images();
                std::thread::yield_now();
            }
        }
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
    }

    fn screen_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height).map(|y| row_text(buffer, y)).collect::<Vec<_>>().join("\n")
    }

    const DIAGRAM_NOTE: &str = "# Diagrams\n\n```mermaid\nflowchart LR\n  A[Write] --> B[Render]\n```\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n\n```mermaid\nnot a diagram\n```\n";

    #[test]
    fn mermaid_blocks_render_inline_and_keep_readable_fallbacks() {
        let mut fixture = GoldenApp::with_content(DIAGRAM_NOTE);
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let fallback = screen_text(&terminal);
        assert!(fallback.contains("◇ Flowchart"), "{fallback}");
        assert!(fallback.contains("A[Write] --> B[Render]"), "{fallback}");
        assert!(!fallback.contains("```"), "{fallback}");

        fixture.app.images.picker = Some(Picker::halfblocks());
        settle_images(&mut fixture, &mut terminal);
        let rendered = screen_text(&terminal);
        assert_eq!(fixture.app.images.image_states.keys().filter(|key| key.starts_with("diagram:block:")).count(), 2);
        assert!(rendered.contains("◇ Sequence diagram"), "{rendered}");
        assert!(rendered.contains("Couldn't render this diagram"), "{rendered}");
        assert!(rendered.contains("not a diagram"), "{rendered}");
        assert!(!rendered.contains("A[Write] --> B[Render]"), "{rendered}");

        fixture.app.state.focus = crate::app::Focus::Content;
        fixture.app.document.content_cursor = fixture.app.document.content_items.iter().position(|item| matches!(item, crate::app::ContentItem::Diagram { .. })).unwrap();
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        assert!(screen_text(&terminal).contains("Enter to explore"));

        fixture.app.images.picker = None;
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        assert!(fixture.app.images.image_states.is_empty());
    }

    #[test]
    fn toggling_a_task_keeps_rendered_blocks_and_folds_in_place() {
        let mut fixture = GoldenApp::with_content("# Tasks\n\n- [ ] ship it\n\n```mermaid\nflowchart LR\n  A[Write] --> B[Render]\n```\n\n$$\n\\frac{a}{b}\n$$\n\n> [!note]- Details\n> Hidden body\n");
        let note_path = fixture.root.join("vault").join("fixture.md");
        if let Some(note) = fixture.app.vault.notes.first_mut() {
            note.file_path = Some(note_path.clone());
        }
        fixture.app.images.picker = Some(Picker::halfblocks());
        let mut terminal = Terminal::new(TestBackend::new(100, 60)).unwrap();
        settle_images(&mut fixture, &mut terminal);
        let item_index = |app: &App, matches: fn(&crate::app::ContentItem) -> bool| app.document.content_items.iter().position(matches).unwrap();
        let task = item_index(&fixture.app, |item| matches!(item, crate::app::ContentItem::TaskItem { .. }));
        let callout = item_index(&fixture.app, |item| matches!(item, crate::app::ContentItem::Callout { .. }));
        fixture.app.toggle_callout_fold_at(callout);
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let mut placements: Vec<String> = fixture.app.images.image_states.keys().cloned().collect();
        placements.sort();
        assert!(placements.iter().any(|key| key.starts_with("diagram:block:")), "{placements:?}");
        assert!(placements.iter().any(|key| key.starts_with("math:block:")), "{placements:?}");
        let diagram_key = placements.iter().find_map(|key| key.strip_prefix("diagram:block:").and_then(|rest| rest.split_once(':')).map(|(_, image_key)| image_key.to_string())).unwrap();
        let scene = fixture.app.diagram_scene(&diagram_key).unwrap();

        fixture.app.toggle_task_at(task);
        assert!(fs::read_to_string(&note_path).unwrap().contains("- [x] ship it"));
        assert!(!fixture.app.image_has_background_work());
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();

        assert!(!fixture.app.image_has_background_work());
        let mut after: Vec<String> = fixture.app.images.image_states.keys().cloned().collect();
        after.sort();
        assert_eq!(after, placements);
        assert!(std::sync::Arc::ptr_eq(&scene, &fixture.app.diagram_scene(&diagram_key).unwrap()));
        assert!(!fixture.app.is_callout_folded(callout));
        let screen = screen_text(&terminal);
        assert!(screen.contains("[x] ship it"), "{screen}");
        assert!(screen.contains("Hidden body"), "{screen}");
        assert!(!screen.contains("Rendering"), "{screen}");
    }

    #[test]
    fn diagram_viewer_renders_frames_that_follow_zoom_and_closes_cleanly() {
        let mut fixture = GoldenApp::with_content(DIAGRAM_NOTE);
        fixture.app.images.picker = Some(Picker::halfblocks());
        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        settle_images(&mut fixture, &mut terminal);
        let first = fixture.app.diagram_item_indices()[0];
        assert!(fixture.app.open_diagram_viewer(first));
        assert_eq!(fixture.app.state.dialog, DialogState::DiagramViewer);
        settle_images(&mut fixture, &mut terminal);
        let screen = screen_text(&terminal);
        assert!(screen.contains("DIAGRAM"), "{screen}");
        assert!(screen.contains("Flowchart"), "{screen}");
        assert!(screen.contains("1 of 3"), "{screen}");
        assert!(screen.contains("Note theme"), "{screen}");
        let viewer = fixture.app.state.diagram_viewer.as_deref().unwrap();
        assert!(viewer.frame.is_some());
        assert!(!viewer.needs_fit);
        let fitted = viewer.frame.as_ref().unwrap().0;

        let viewer = fixture.app.state.diagram_viewer.as_deref_mut().unwrap();
        viewer.set_zoom(viewer.zoom * 3.0, None);
        viewer.pan_view_fraction(0.5, 0.0);
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let zoomed = fixture.app.state.diagram_viewer.as_deref().unwrap().frame.as_ref().unwrap().0;
        assert!(zoomed.zoom > fitted.zoom);
        assert_ne!(zoomed.center, fitted.center);
        assert!(screen_text(&terminal).contains('━'), "zoomed views show a horizontal scrollbar");

        fixture.app.state.diagram_viewer.as_deref_mut().unwrap().style = 1;
        settle_images(&mut fixture, &mut terminal);
        assert!(screen_text(&terminal).contains("Light"));
        assert!(fixture.app.state.diagram_viewer.as_deref().unwrap().frame.is_some());

        fixture.app.step_diagram_viewer(1);
        settle_images(&mut fixture, &mut terminal);
        let screen = screen_text(&terminal);
        assert!(screen.contains("Sequence diagram") && screen.contains("2 of 3"), "{screen}");
        assert_eq!(fixture.app.state.diagram_viewer.as_deref().unwrap().style, 1);

        fixture.app.step_diagram_viewer(1);
        settle_images(&mut fixture, &mut terminal);
        assert!(screen_text(&terminal).contains("Couldn't render this diagram"));

        fixture.app.close_diagram_viewer();
        assert_eq!(fixture.app.state.dialog, DialogState::None);
        assert!(fixture.app.state.diagram_viewer.is_none());
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
    }

    #[test]
    fn math_beyond_the_decoded_image_budget_stays_rendered_without_reload_churn() {
        let equation = r"Q_{m,INDEX} =\underbrace{\frac{C_{0,i}}{2}\sum_{n=1}^{N}\psi_{n,i}\,\omega_{n}}_{S_{sc,i}} +\underbrace{\frac{C_{f,i}}{2}\sum_{n=1}^{N}\psi_{n,i}\,\omega_{n}}_{S_{f,i}} +\frac{S_{m,i}}{\sigma_{t,i}} =S_{sc,i}+S_{f,i}+\frac{S_{m,i}}{\sigma_{t,i}}";
        let mut content = String::from("# Budget\n\n");
        for index in 0..8 {
            content.push_str(&format!("$$\n{}\n$$\n\n", equation.replace("INDEX", &index.to_string())));
        }
        let mut fixture = GoldenApp::with_content(&content);
        fixture.app.images.picker = Some(Picker::halfblocks());
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
            if !fixture.app.image_has_background_work() || Instant::now() > deadline {
                break;
            }
            while fixture.app.image_has_background_work() && Instant::now() < deadline {
                fixture.app.poll_pending_images();
                std::thread::yield_now();
            }
        }
        assert!(fixture.app.images.worker.stats().decoded_entries < 8, "fixture must exceed the decoded image budget");
        for _ in 0..3 {
            terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
            assert!(!fixture.app.image_has_background_work());
        }
        let buffer = terminal.backend().buffer();
        let symbols = (0..buffer.area.height).flat_map(|y| (0..buffer.area.width).map(move |x| buffer[(x, y)].symbol())).collect::<String>();
        assert!(!symbols.contains("Rendering equation"), "{symbols}");
        assert!(fixture.app.images.image_states.keys().any(|key| key.starts_with("math:block:")));
    }

    #[test]
    fn links_after_rendered_inline_math_keep_their_click_target() {
        let mut fixture = GoldenApp::with_content("# Math link\n\nBefore $\\frac{a}{b}$ [docs](https://example.test) after.\n");
        fixture.app.images.picker = Some(Picker::halfblocks());
        let backend = TestBackend::new(100, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while fixture.app.image_has_background_work() && Instant::now() < deadline {
            fixture.app.poll_pending_images();
            std::thread::yield_now();
        }
        terminal.draw(|frame| render(frame, &mut fixture.app)).unwrap();

        let item_index = fixture.app.document.content_items.iter().position(|item| item.source_line() == 2).unwrap();
        let item_area = fixture.app.state.content_item_rects.iter().find_map(|(index, rect)| (*index == item_index).then_some(*rect)).unwrap();
        let math_width = fixture.app.images.image_states.iter().find_map(|(key, state)| key.starts_with(&format!("math:inline:{item_index}:0:")).then_some(state.size.width)).unwrap();
        let link_x = item_area.x + 2 + "Before ".width() as u16 + math_width + 1;
        assert_eq!(content_item_click_col(&fixture.app, item_index, item_area, link_x, item_area.y), None);
        let text_row = item_area.y + item_area.height.saturating_sub(1);
        let rendered_col = content_item_click_col(&fixture.app, item_index, item_area, link_x, text_row).unwrap();
        assert_eq!(fixture.app.find_clicked_link_at_col(item_index, rendered_col).as_deref(), Some("https://example.test"));
    }

    #[test]
    fn unicode_and_tab_link_click_columns_use_terminal_cells() {
        let fixture = GoldenApp::with_content("# Clicks\n\nASCII e\u{301} 日本 😀\t[開く](https://example.test) tail\n");
        let item = fixture.app.document.content_items.iter().position(|item| item.source_line() == 2).unwrap();
        let links = fixture.app.item_links_at(item);
        assert_eq!(links.len(), 1);
        assert_eq!((links[0].2, links[0].3), (19, 23));
        assert_eq!(fixture.app.find_clicked_link_at_col(item, 21).as_deref(), Some("https://example.test"));
        assert_eq!(fixture.app.find_clicked_link_at_col(item, 24).as_deref(), Some("https://example.test"));
        assert_eq!(fixture.app.find_clicked_link_at_col(item, 25), None);
    }

    #[test]
    fn golden_onboarding_dialog_100x30() {
        let mut fixture = GoldenApp::new();
        fixture.app.state.dialog = DialogState::Onboarding;
        assert_eq!(fixture.hash(100, 30), 15_714_349_546_688_206_610);
    }

    #[test]
    fn golden_create_document_dialog_72x22() {
        let mut fixture = GoldenApp::new();
        fixture.app.state.dialog = DialogState::CreateDocument(crate::vault::VaultFileKind::Markdown);
        fixture.app.state.input_buffer = "deterministic-note".to_string();
        assert_eq!(fixture.hash(72, 22), 9_240_068_141_463_981_098);
    }

    #[test]
    fn changelog_modal_renders_announcement_before_summary() {
        let mut fixture = GoldenApp::new();
        fixture.app.open_changelog();
        let buffer = draw_changelog(&mut fixture, "0.50.10", 80, 24);
        let content = (0..24).map(|y| row_text(&buffer, y)).collect::<String>();
        assert!(content.contains("What's new in Ekphos"), "{content}");
        let announcement = content.find("Announcement").expect("announcement heading");
        let summary = content.find("Summary").expect("summary heading");
        assert!(announcement < summary, "{content}");

        let [(link_area, url)] = fixture.app.state.changelog_links.as_slice() else {
            panic!("expected one changelog link, got {:?}", fixture.app.state.changelog_links);
        };
        assert_eq!(url, "https://discord.gg/XBDstnqXVb");
        assert!(link_area.width > 0, "Discord link should be visible and clickable");
        let link_cell = &buffer[(link_area.x, link_area.y)];
        assert_eq!(link_cell.fg, fixture.app.state.theme.content.link);
        assert_eq!(link_cell.bg, fixture.app.state.theme.flat.surface_raised);
        assert!(link_cell.modifier.contains(ratatui::style::Modifier::UNDERLINED));
        let announcement_row = (0..24).find(|&y| row_text(&buffer, y).contains("Announcement")).expect("announcement row");
        let announcement_x = column_of(&buffer, announcement_row, "A").expect("announcement column");
        let card_x = (0..80).find(|&x| buffer[(x, announcement_row)].bg == fixture.app.state.theme.flat.surface_raised).expect("announcement card edge");
        assert_eq!(announcement_x, card_x + 2, "announcement copy should have two columns of internal padding");
        let card_top = (0..24).find(|&y| buffer[(card_x, y)].bg == fixture.app.state.theme.flat.surface_raised).expect("announcement card top");
        assert_eq!(announcement_row, card_top + 1, "announcement card should have one row of top padding");

        let footer_row = (0..24).find(|&y| row_text(&buffer, y).contains("Open Discord")).expect("footer row");
        let footer_gap = (3..77).map(|x| buffer[(x, footer_row - 1)].symbol()).collect::<String>();
        assert!(footer_gap.trim().is_empty(), "footer should have a blank row above it: {footer_gap:?}");

        let _ = draw(&mut fixture, 40, 12);
    }

    #[test]
    fn default_panel_layout_keeps_twenty_percent_sides_and_center_minimum() {
        let config = Config::default();
        assert_eq!(main_layout_constraints(false, false, false, config.effective_sidebar_width_percent(), config.effective_outline_width_percent(),), [Constraint::Percentage(20), Constraint::Min(20), Constraint::Percentage(20),]);
    }

    #[test]
    fn custom_panel_layout_uses_independent_effective_widths() {
        let config = Config { general: crate::config::GeneralConfig { sidebar_width_percent: 30, outline_width_percent: 140, ..Default::default() }, ..Default::default() };
        assert_eq!(main_layout_constraints(false, false, false, config.effective_sidebar_width_percent(), config.effective_outline_width_percent(),), [Constraint::Percentage(30), Constraint::Min(20), Constraint::Percentage(95),]);
    }

    #[test]
    fn collapsed_panels_override_configured_widths() {
        assert_eq!(main_layout_constraints(false, true, true, 35, 45), [Constraint::Length(5), Constraint::Min(20), Constraint::Length(5),]);
    }

    #[test]
    fn widths_below_ten_percent_use_minimized_constraints() {
        assert_eq!(main_layout_constraints(false, false, false, 9, 5), [Constraint::Length(5), Constraint::Min(20), Constraint::Length(5),]);
        assert_eq!(main_layout_constraints(false, false, false, 10, 10), [Constraint::Percentage(10), Constraint::Min(20), Constraint::Percentage(10),]);
    }

    #[test]
    fn zen_mode_overrides_configured_and_collapsed_widths() {
        assert_eq!(main_layout_constraints(true, true, false, 35, 45), [Constraint::Length(0), Constraint::Min(20), Constraint::Length(0),]);
    }

    #[test]
    fn wide_layout_applies_independent_panel_percentages() {
        let chunks = Layout::default().direction(Direction::Horizontal).constraints(main_layout_constraints(false, false, false, 25, 15)).split(Rect::new(0, 0, 200, 20));
        assert_eq!(chunks[0].width, 50);
        assert_eq!(chunks[1].width, 120);
        assert_eq!(chunks[2].width, 30);
    }

    #[test]
    fn narrow_layout_retains_center_panel_minimum() {
        let chunks = Layout::default().direction(Direction::Horizontal).constraints(main_layout_constraints(false, false, false, 95, 95)).split(Rect::new(0, 0, 40, 20));
        assert!(chunks[1].width >= 20);
        assert_eq!(chunks.iter().map(|chunk| chunk.width).sum::<u16>(), 40);
    }
}
