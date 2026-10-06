use ratatui::style::{Color, Modifier, Style as RatatuiStyle};
use ratatui::text::Span;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, FontStyle, Style, Theme as SyntectTheme, ThemeSet};
use syntect::parsing::SyntaxSet;

pub const DEFAULT_SYNTAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
pub const FALLBACK_SYNTAX_THEME: &str = "base16-ocean.dark";
pub const AUTO_SYNTAX_THEME: &str = "auto";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxThemeRequest {
    Named(String),
    Auto { background: Color, preferred: Option<String> },
}

impl Default for SyntaxThemeRequest {
    fn default() -> Self {
        Self::Auto { background: Color::Reset, preferred: None }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    content_hash: u64,
    lang: String,
    theme: String,
}

struct CacheEntry {
    lines: Vec<Vec<Span<'static>>>,
    bytes: usize,
}

struct HighlightCache {
    entries: HashMap<CacheKey, CacheEntry>,
    lru: VecDeque<CacheKey>,
    bytes: usize,
    budget: usize,
}

impl HighlightCache {
    fn new(budget: usize) -> Self {
        Self { entries: HashMap::new(), lru: VecDeque::new(), bytes: 0, budget }
    }
    fn get(&mut self, key: &CacheKey) -> Option<Vec<Vec<Span<'static>>>> {
        let lines = self.entries.get(key)?.lines.clone();
        self.touch(key);
        Some(lines)
    }
    fn insert(&mut self, key: CacheKey, lines: Vec<Vec<Span<'static>>>) {
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(previous.bytes);
            self.lru.retain(|candidate| candidate != &key);
        }
        let bytes = cache_entry_bytes(&key, &lines);
        self.bytes = self.bytes.saturating_add(bytes);
        self.lru.push_back(key.clone());
        self.entries.insert(key, CacheEntry { lines, bytes });
        while self.bytes > self.budget && self.entries.len() > 1 {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
    }
    fn touch(&mut self, key: &CacheKey) {
        self.lru.retain(|candidate| candidate != key);
        self.lru.push_back(key.clone());
    }
    fn clear(&mut self) {
        self.entries.clear();
        self.lru.clear();
        self.bytes = 0;
    }
}
fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}
fn cache_entry_bytes(key: &CacheKey, lines: &[Vec<Span<'static>>]) -> usize {
    std::mem::size_of::<CacheKey>()
        + key.lang.capacity()
        + key.theme.capacity()
        + std::mem::size_of_val(lines)
        + lines
            .iter()
            .map(|spans| {
                spans.capacity() * std::mem::size_of::<Span<'static>>()
                    + spans
                        .iter()
                        .map(|span| match &span.content {
                            Cow::Borrowed(_) => 0,
                            Cow::Owned(text) => text.capacity(),
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
    definition_bytes: usize,
    cache: RefCell<HighlightCache>,
}

impl Highlighter {
    pub fn new(theme_name: &str) -> Self {
        Self::with_cache_budget(theme_name, DEFAULT_SYNTAX_CACHE_BYTES)
    }
    fn with_cache_budget(theme_name: &str, cache_budget: usize) -> Self {
        let theme_set = ThemeSet::load_defaults();
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let valid_theme = valid_theme_name(&theme_set, theme_name);
        let definition_bytes = syntect::dumps::dump_binary(&syntax_set).len() + syntect::dumps::dump_binary(&theme_set).len();
        Self { syntax_set, theme_set, theme_name: valid_theme, definition_bytes, cache: RefCell::new(HighlightCache::new(cache_budget)) }
    }

    pub fn highlight_block(&self, content: &str, lang: &str) -> Vec<Vec<Span<'static>>> {
        let key = CacheKey { content_hash: hash_content(content), lang: lang.to_string(), theme: self.theme_name.clone() };
        if let Some(cached) = self.cache.borrow_mut().get(&key) {
            return cached;
        }
        let syntax = self.syntax_set.find_syntax_by_token(lang).or_else(|| self.syntax_set.find_syntax_by_extension(lang)).unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes[&self.theme_name];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let result: Vec<Vec<Span<'static>>> = content
            .split('\n')
            .map(|line| {
                let line_with_newline = format!("{line}\n");
                match highlighter.highlight_line(&line_with_newline, &self.syntax_set) {
                    Ok(ranges) => ranges
                        .into_iter()
                        .map(|(style, text)| {
                            let cleaned = text.trim_end_matches('\n');
                            self.style_to_span(cleaned, style)
                        })
                        .filter(|span| !span.content.is_empty())
                        .collect(),
                    Err(_) => vec![Span::raw(line.to_string())],
                }
            })
            .collect();
        self.cache.borrow_mut().insert(key, result.clone());
        result
    }

    pub fn definition_bytes(&self) -> usize {
        self.definition_bytes
    }

    pub fn retained_cache_bytes(&self) -> usize {
        self.cache.borrow().bytes
    }

    pub fn cache_entries(&self) -> usize {
        self.cache.borrow().entries.len()
    }

    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    pub fn has_theme(&self, theme_name: &str) -> bool {
        self.theme_set.themes.contains_key(theme_name)
    }

    pub fn theme_name(&self) -> &str {
        &self.theme_name
    }

    pub fn theme_names(&self) -> impl Iterator<Item = &str> {
        self.theme_set.themes.keys().map(String::as_str)
    }

    pub fn resolve_theme(&self, request: &SyntaxThemeRequest) -> String {
        let (background, preferred) = match request {
            SyntaxThemeRequest::Named(name) => return valid_theme_name(&self.theme_set, name),
            SyntaxThemeRequest::Auto { background, preferred } => (*background, preferred.as_deref()),
        };
        let light = is_light_color(background);
        let matches = |theme: &SyntectTheme| syntect_theme_is_light(theme) == light;
        if let Some(preferred) = preferred.filter(|name| self.theme_set.themes.get(*name).is_some_and(matches)) {
            return preferred.to_string();
        }
        self.theme_set.themes.iter().filter(|(_, theme)| matches(theme)).max_by(|(_, left), (_, right)| contrast_score(left, background).total_cmp(&contrast_score(right, background))).map_or_else(|| FALLBACK_SYNTAX_THEME.to_string(), |(name, _)| name.clone())
    }

    pub fn apply_theme(&mut self, request: &SyntaxThemeRequest) {
        let name = self.resolve_theme(request);
        self.set_theme(&name);
    }

    pub fn set_theme(&mut self, theme_name: &str) {
        let valid_theme = valid_theme_name(&self.theme_set, theme_name);
        if self.theme_name != valid_theme {
            self.theme_name = valid_theme;
            self.clear_cache();
        }
    }
    fn style_to_span(&self, text: &str, style: Style) -> Span<'static> {
        let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
        let mut ratatui_style = RatatuiStyle::default().fg(fg);
        if style.font_style.contains(FontStyle::BOLD) {
            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
        }
        if style.font_style.contains(FontStyle::ITALIC) {
            ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
        }
        if style.font_style.contains(FontStyle::UNDERLINE) {
            ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
        }
        Span::styled(text.to_string(), ratatui_style)
    }
}
fn valid_theme_name(theme_set: &ThemeSet, requested: &str) -> String {
    if theme_set.themes.contains_key(requested) {
        requested.to_string()
    } else {
        FALLBACK_SYNTAX_THEME.to_string()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new(FALLBACK_SYNTAX_THEME)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast_ratio(left: f64, right: f64) -> f64 {
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn syntect_luminance(color: SyntectColor) -> f64 {
    relative_luminance(color.r, color.g, color.b)
}

pub fn is_light_color(color: Color) -> bool {
    match color {
        Color::Rgb(r, g, b) => relative_luminance(r, g, b) > 0.179,
        _ => false,
    }
}

fn syntect_theme_is_light(theme: &SyntectTheme) -> bool {
    theme.settings.background.is_some_and(|background| syntect_luminance(background) > 0.179)
}

fn contrast_score(theme: &SyntectTheme, background: Color) -> f64 {
    let Color::Rgb(r, g, b) = background else {
        return 0.0;
    };
    let background = relative_luminance(r, g, b);
    let mut colors: Vec<SyntectColor> = theme.settings.foreground.into_iter().collect();
    for color in theme.scopes.iter().filter_map(|item| item.style.foreground) {
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    if colors.is_empty() {
        return 0.0;
    }
    colors.iter().map(|color| contrast_ratio(syntect_luminance(*color), background)).sum::<f64>() / colors.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_block_trailing_empty_line() {
        let h = Highlighter::default();
        let content = "line1\nline2\n";
        let result = h.highlight_block(content, "txt");
        assert_eq!(result.len(), 3, "Should produce 3 lines including trailing empty");
    }

    #[test]
    fn test_highlight_block_cjk_no_panic() {
        let h = Highlighter::default();
        let content = "print(\"\u{4f60}\u{597d}\u{4e16}\u{754c}\")\nx = \"\u{6d4b}\u{8bd5}\"";
        let result = h.highlight_block(content, "python");
        assert_eq!(result.len(), 2);
        assert!(!result[0].is_empty());
        assert!(!result[1].is_empty());
    }

    #[test]
    fn test_highlight_block_c_with_cjk_comments() {
        let h = Highlighter::default();
        let content = "#include \"user/user.h\"\nint main(int argc, char *argv[]) {\n    // \u{9519}\u{8bef}\u{68c0}\u{67e5}\n    if (argc != 2) {\n        printf(\"hello\");\n    }\n}";
        let result = h.highlight_block(content, "c");
        assert_eq!(result.len(), 7);
        let line_after_cjk = &result[3];
        assert!(line_after_cjk.len() > 1, "Line after CJK comment should have multiple highlighted spans, got {} span(s): {:?}", line_after_cjk.len(), line_after_cjk.iter().map(|s| s.content.as_ref()).collect::<Vec<_>>());
    }

    #[test]
    fn syntax_cache_is_byte_weighted_and_keeps_only_one_oversized_result() {
        let h = Highlighter::with_cache_budget("base16-ocean.dark", 1);
        h.highlight_block("let first = 1;", "rust");
        h.highlight_block("let second = 2;", "rust");
        assert_eq!(h.cache_entries(), 1);
        assert!(h.retained_cache_bytes() > 1);
        assert!(h.definition_bytes() > 0);
    }

    #[test]
    fn theme_change_invalidates_results() {
        let mut h = Highlighter::default();
        h.highlight_block("let value = true;", "rust");
        assert_eq!(h.cache_entries(), 1);
        let alternative = h.theme_set.themes.keys().find(|name| *name != &h.theme_name).unwrap().clone();
        h.set_theme(&alternative);
        assert_eq!(h.cache_entries(), 0);
    }

    fn auto(background: Color, preferred: Option<&str>) -> SyntaxThemeRequest {
        SyntaxThemeRequest::Auto { background, preferred: preferred.map(str::to_string) }
    }

    #[test]
    fn auto_matches_lightness_and_picks_the_highest_contrast_theme() {
        let h = Highlighter::default();
        for name in crate::config::BUNDLED_THEMES {
            let background = crate::config::Theme::from_name_in(name, std::path::Path::new("/nonexistent-ekphos-themes")).content.code_background;
            let resolved = h.resolve_theme(&auto(background, None));
            let resolved_theme = &h.theme_set.themes[&resolved];
            assert_eq!(syntect_theme_is_light(resolved_theme), is_light_color(background), "{name} -> {resolved}");
            let best = contrast_score(resolved_theme, background);
            for (candidate, theme) in &h.theme_set.themes {
                if syntect_theme_is_light(theme) == is_light_color(background) {
                    assert!(contrast_score(theme, background) <= best, "{name}: {candidate} beats {resolved}");
                }
            }
        }
        assert_eq!(h.resolve_theme(&auto(Color::Rgb(0xeb, 0xdb, 0xb2), None)), "InspiredGitHub");
        assert_eq!(h.resolve_theme(&auto(Color::Rgb(0x24, 0x24, 0x3a), None)), "base16-eighties.dark");
    }

    #[test]
    fn auto_keeps_a_preferred_theme_only_when_its_lightness_matches() {
        let h = Highlighter::default();
        let light = Color::Rgb(0xeb, 0xdb, 0xb2);
        let dark = Color::Rgb(0x24, 0x24, 0x3a);
        assert_eq!(h.resolve_theme(&auto(light, Some("Solarized (light)"))), "Solarized (light)");
        assert_eq!(h.resolve_theme(&auto(light, Some("base16-ocean.dark"))), "InspiredGitHub");
        assert_eq!(h.resolve_theme(&auto(dark, Some("base16-ocean.dark"))), "base16-ocean.dark");
        assert_eq!(h.resolve_theme(&auto(dark, Some("Solarized (light)"))), "base16-eighties.dark");
        assert_eq!(h.resolve_theme(&auto(dark, Some("not-a-theme"))), "base16-eighties.dark");
    }

    #[test]
    fn named_request_is_exact_and_falls_back_when_unknown() {
        let h = Highlighter::default();
        assert_eq!(h.resolve_theme(&SyntaxThemeRequest::Named("Solarized (dark)".to_string())), "Solarized (dark)");
        assert_eq!(h.resolve_theme(&SyntaxThemeRequest::Named("not-a-theme".to_string())), FALLBACK_SYNTAX_THEME);
    }

    #[test]
    fn every_bundled_syntax_and_theme_fixture_loads() {
        let h = Highlighter::default();
        let default_theme = &h.theme_set.themes["base16-ocean.dark"];
        for syntax in h.syntax_set.syntaxes() {
            let mut lines = HighlightLines::new(syntax, default_theme);
            assert!(lines.highlight_line("fixture text\n", &h.syntax_set).is_ok(), "{}", syntax.name);
        }
        let rust = h.syntax_set.find_syntax_by_token("rust").unwrap();
        for (name, theme) in &h.theme_set.themes {
            let mut lines = HighlightLines::new(rust, theme);
            assert!(lines.highlight_line("fn fixture() {}\n", &h.syntax_set).is_ok(), "{name}");
        }
    }
}
