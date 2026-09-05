use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::panel::{panel_surface, render_accent_bar, render_panel, PanelFrame, SurfaceKind};
use crate::app::{CutItem, Focus, Mode, SearchState, SidebarItemKind, VaultState};
use crate::config::{Config, Theme};

pub struct SidebarView<'a> {
    pub theme: &'a Theme,
    pub config: &'a Config,
    pub vault: &'a VaultState,
    pub search: &'a SearchState,
    pub focus: Focus,
    pub mode: Mode,
    pub minimized: bool,
}

pub fn render_sidebar(f: &mut Frame, view: SidebarView<'_>, area: Rect) -> Rect {
    let theme = view.theme;
    let sidebar_theme = &theme.sidebar;
    if view.minimized {
        render_collapsed_sidebar(f, &view, area);
        return Rect::default();
    }
    let style = view.config.style;
    let (search_area, list_area) = if view.search.search_active {
        let search_height = if style.is_flat() { 1 } else { 3 };
        let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(search_height), Constraint::Min(0)]).split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };
    if let Some(search_area) = search_area {
        let has_query = !view.search.search_query.is_empty();
        let has_results = !view.search.search_matched_notes.is_empty();
        let border_color = if has_query && !has_results {
            theme.error
        } else if has_query && has_results {
            theme.success
        } else {
            theme.warning
        };
        let search_line = Line::from(vec![Span::styled("/", Style::default().fg(theme.foreground)), Span::styled(&view.search.search_query, Style::default().fg(theme.foreground)), Span::styled("_", Style::default().fg(border_color))]);
        if style.is_flat() {
            let raised = panel_surface(view.config, theme, SurfaceKind::Raised);
            let mut search_style = Style::default();
            if let Some(bg) = raised {
                search_style = search_style.bg(bg);
            }
            let input_area = Rect { x: search_area.x + 1, y: search_area.y, width: search_area.width.saturating_sub(1), height: search_area.height };
            f.render_widget(Paragraph::new(search_line).style(search_style), input_area);
            render_accent_bar(f, search_area, border_color, raised);
        } else {
            let search_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title(" Search ");
            f.render_widget(Paragraph::new(search_line).block(search_block), search_area);
        }
    }
    let is_searching = view.search.search_active && !view.search.search_query.is_empty();
    let items: Vec<ListItem> = view
        .vault
        .sidebar_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == view.vault.selected_sidebar_index;
            let indent = "  ".repeat(item.depth);
            let is_cut = match (&view.vault.cut_buffer, &item.kind) {
                (Some(CutItem::Note { source_path, .. }), SidebarItemKind::Note { note_id }) => view.vault.notes.iter().find(|note| note.id == *note_id).and_then(|note| note.file_path.as_ref()).map(|path| path == source_path).unwrap_or(false),
                (Some(CutItem::Folder { source_path, .. }), SidebarItemKind::Folder(folder)) => &folder.path == source_path,
                _ => false,
            };
            let (icon, mut style) = match &item.kind {
                SidebarItemKind::Folder(folder) => {
                    let icon = if folder.expanded { "▼ " } else { "▶ " };
                    let folder_color = if folder.expanded { sidebar_theme.folder_expanded } else { sidebar_theme.folder };
                    let style = if is_selected { Style::default().fg(folder_color).add_modifier(Modifier::BOLD) } else { Style::default().fg(folder_color) };
                    (icon, style)
                }
                SidebarItemKind::Note { note_id } => {
                    let note = view.vault.notes.iter().find(|note| note.id == *note_id);
                    let icon = match note.map(|note| note.kind) {
                        Some(ekphos_vault::VaultFileKind::Base) => "▦ ",
                        Some(ekphos_vault::VaultFileKind::Canvas) => "◇ ",
                        Some(ekphos_vault::VaultFileKind::Markdown) | None => "  ",
                    };
                    let is_match = is_searching && view.vault.notes.iter().position(|note| note.id == *note_id).is_some_and(|index| view.search.search_matched_notes.contains(&index));
                    let style = if is_selected {
                        Style::default().fg(sidebar_theme.item_selected).add_modifier(Modifier::BOLD)
                    } else if is_match {
                        Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(sidebar_theme.item)
                    };
                    (icon, style)
                }
            };
            if is_cut {
                style = style.add_modifier(Modifier::DIM | Modifier::ITALIC);
            }
            let display_name = match item.kind {
                SidebarItemKind::Note { note_id } => view.vault.notes.iter().find(|note| note.id == note_id).map(|note| note.title.as_str()).unwrap_or(""),
                SidebarItemKind::Folder(_) => item.display_name.as_str(),
            };
            let display = format!("{}{}{}", indent, icon, display_name);
            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();
    let focused = view.focus == Focus::Sidebar && view.mode == Mode::Normal;
    let title = if is_searching {
        let match_count = view.search.search_matched_notes.len();
        let total_count = view.vault.notes.len();
        format!(" Found {}/{} ", match_count, total_count)
    } else {
        let note_count = view.vault.sidebar_items.iter().filter(|item| matches!(item.kind, SidebarItemKind::Note { .. })).count();
        format!(" Notes ({}) [{}] ", note_count, view.vault.sort_mode.label())
    };
    let frame = PanelFrame { style, theme, title, focused, accent: theme.primary, surface: panel_surface(view.config, theme, SurfaceKind::Side) };
    let inner = render_panel(f, &frame, list_area);
    let sidebar = List::new(items).highlight_style(Style::default().bg(theme.selection).add_modifier(Modifier::BOLD)).highlight_symbol("");
    let mut list_state = ListState::default();
    list_state.select(Some(view.vault.selected_sidebar_index));
    f.render_stateful_widget(sidebar, inner, &mut list_state);
    list_area
}
fn render_collapsed_sidebar(f: &mut Frame, view: &SidebarView<'_>, area: Rect) {
    let theme = view.theme;
    let style = view.config.style;
    let focused = view.focus == Focus::Sidebar && view.mode == Mode::Normal;
    let note_count = view.vault.sidebar_items.iter().filter(|item| matches!(item.kind, SidebarItemKind::Note { .. })).count();
    let mut lines: Vec<Line> = Vec::new();
    let available_height = area.height.saturating_sub(style.vertical_inset()) as usize;
    let padding_top = available_height / 2;
    for _ in 0..padding_top {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(" ≡", Style::default().fg(theme.info))));
    lines.push(Line::from(Span::styled(format!(" {}", note_count), Style::default().fg(theme.foreground))));
    let frame = PanelFrame { style, theme, title: String::new(), focused, accent: theme.primary, surface: panel_surface(view.config, theme, SurfaceKind::Side) };
    let inner = render_panel(f, &frame, area);
    f.render_widget(Paragraph::new(lines), inner);
}
