use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Padding},
    Frame,
};

use crate::config::{StyleMode, Theme};
pub use crate::panel::{panel_surface, PanelFrame, SurfaceKind};

pub fn full_view_block(style: StyleMode, theme: &Theme, title: Line<'static>) -> Block<'static> {
    let block = Block::default().title(title).style(Style::default().bg(theme.dialog.background));
    match style {
        StyleMode::Outlined => block.borders(Borders::ALL).border_style(Style::default().fg(theme.dialog.border)),
        StyleMode::Flat => block.padding(Padding::horizontal(1)),
    }
}

pub fn full_view_inner(style: StyleMode, area: Rect) -> Rect {
    Rect::new(area.x.saturating_add(1), area.y.saturating_add(1), area.width.saturating_sub(2), area.height.saturating_sub(style.vertical_inset()))
}

pub fn render_panel(f: &mut Frame, frame: &PanelFrame<'_>, area: Rect) -> Rect {
    let block = frame.block();
    let inner = block.inner(area);
    f.render_widget(block, area);
    if let Some(color) = frame.bar_color() {
        render_accent_bar(f, area, color, frame.surface);
    }
    inner
}

pub fn render_accent_bar(f: &mut Frame, area: Rect, color: Color, surface: Option<Color>) {
    let buf = f.buffer_mut();
    let mut style = Style::default().fg(color);
    if let Some(bg) = surface {
        style = style.bg(bg);
    }
    for y in area.y..area.bottom() {
        if let Some(cell) = buf.cell_mut((area.x, y)) {
            cell.set_symbol("▌");
            cell.set_style(style);
        }
    }
}
