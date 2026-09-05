use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding},
};

use crate::config::{Config, StyleMode, Theme};

#[derive(Clone, Copy)]
pub enum SurfaceKind {
    Side,
    Raised,
    Content,
}

pub fn panel_surface(config: &Config, theme: &Theme, kind: SurfaceKind) -> Option<Color> {
    match kind {
        SurfaceKind::Side => Some(theme.flat.surface),
        SurfaceKind::Raised => Some(theme.flat.surface_raised),
        SurfaceKind::Content => (!config.transparent_bg).then_some(theme.flat.content_bg),
    }
}

pub struct PanelFrame<'a> {
    pub style: StyleMode,
    pub theme: &'a Theme,
    pub title: String,
    pub focused: bool,
    pub accent: Color,
    pub surface: Option<Color>,
}

impl PanelFrame<'_> {
    pub fn block(&self) -> Block<'static> {
        match self.style {
            StyleMode::Outlined => {
                let border = if self.focused { self.accent } else { self.theme.border };
                Block::default().title(self.title.clone()).borders(Borders::ALL).border_style(Style::default().fg(border))
            }
            StyleMode::Flat => {
                let mut title_style = Style::default().fg(if self.focused { self.accent } else { self.theme.muted });
                if self.focused {
                    title_style = title_style.add_modifier(Modifier::BOLD);
                }
                let mut block = Block::default().title(Line::from(Span::styled(self.title.clone(), title_style))).padding(Padding::horizontal(1));
                if let Some(surface) = self.surface {
                    block = block.style(Style::default().bg(surface));
                }
                block
            }
        }
    }

    pub fn bar_color(&self) -> Option<Color> {
        (self.style.is_flat() && self.focused).then_some(self.accent)
    }
}
