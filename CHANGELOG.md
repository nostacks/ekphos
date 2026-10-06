# Changelog

Notable Ekphos changes are summarized here. The in-app “What’s new” dialog shows the entry that matches the installed version.

## [Unreleased]

### Summary

- Code blocks now pick a syntax theme that matches the UI theme: light themes get a light palette and dark themes a dark one, choosing the highest-contrast option against the theme's code background. `syntax_theme` now defaults to `auto`.
- Added a Syntax row to the theme selector (`Ctrl+T`). Press `s` or `S` to cycle through the syntax themes with live preview. The choice is saved per UI theme, and cycling back to Auto clears it.
- Theme files can declare their own `syntax_theme`, and an unknown syntax theme name now shows an error instead of silently falling back.

## [0.60.10] - 2026-10-03

### Summary

- Added Catppuccin (Mocha, Macchiato, Frappé, Latte) and the full Gruvbox family (dark, dark soft, light hard, light, light soft) as official themes.

## [0.60.0] - 2026-10-01

### Summary

- Aligned LaTeX rendering with Obsidian: display equations inside list items, `$$` delimiters that share a line with the equation, `$$...$$` inside prose, unnumbered `align`, `equation`, and `gather`, note-wide `\newcommand` macros, and common MathJax commands such as `\label`, `\eqref`, `\DeclareMathOperator`, `multline`, and `eqnarray`.
- Rendered every equation at one consistent scale, so `\displaystyle` and other tall expressions appear larger than plain inline math and line up with the surrounding text.
- Added Mermaid diagrams. Fenced `mermaid` blocks render inline in the note's theme colors, and `Enter`, `Space`, or a click opens a full-screen viewer with zoom, pan, fit, actual size, light and dark looks, diagram-to-diagram navigation, and shortcuts to edit or copy the source. Diagrams that fail to render show the parse error and their source.
- Added Obsidian callouts. `> [!type] Title` blocks render with a colored bar, icon, and title for every built-in type and alias, and `+`/`-` callouts fold with `Space`, `za`, or a click.
- Added pasting images from the clipboard. Screenshots and copied image files are saved to a configurable `attachments_dir` and linked into the note.
- Fixed `cargo install ekphos` failing on Windows. Windows builds now use the system allocator instead of jemalloc.

## [0.50.20] - 2026-09-20

### Summary

- Added in-app creation for Markdown notes, JSON Canvas files, and Obsidian Bases through the New Document dialog.
- Made Canvas visually authorable with pointer and keyboard context menus, text/file/link/group creation, vault file search, duplication, card deletion, and empty-space double-click creation.
- Added a collapsible Canvas shortcut legend and dedicated, remappable Canvas navigation, zoom, and history keys so global panel focus, history, zen mode, and task view keep their original behavior.
- Expanded LaTeX rendering with `\(...\)` and `\[...\]` delimiters, working `\displaystyle` overrides, and configurable inline equation height.

## [0.50.10] - 2026-09-14

### Announcement

Ekphos is now part of nostacks, a software lab founded by Ekphos’s creator. Ekphos also has a Discord community where you can report bugs, request features, discuss ideas, or just hang out: https://discord.gg/XBDstnqXVb

### Summary

- Added Base and Canvas views, task management, heading folding, and frontmatter templates.
- Added Standard editing, flat interface styling, remappable keybindings, and an in-app “What’s new” dialog.
- Added graphical LaTeX rendering and expanded Markdown presentation.
- Reduced memory and compile costs while improving indexing, images, graphs, clipboard support, and links.

## [0.25.10] - 2026-06-03

### Summary

- Added Wayland clipboard support.
- Fixed graphics-protocol detection before terminal output is redirected.

## [0.25.0] - 2026-05-30

### Summary

- Added daily journals, theme selection, adjustable panels, and internal Markdown links.
- Improved tables, search highlighting, image sizing, and editor reliability.

## [0.20.10] - 2026-02-06

### Summary

- Improved terminal compatibility and packaging across supported platforms.

[Unreleased]: https://github.com/nostacks/ekphos/compare/v0.60.10...HEAD
[0.60.10]: https://github.com/nostacks/ekphos/releases/tag/v0.60.10
[0.60.0]: https://github.com/nostacks/ekphos/releases/tag/v0.60.0
[0.50.20]: https://github.com/nostacks/ekphos/releases/tag/v0.50.20
[0.50.10]: https://github.com/nostacks/ekphos/releases/tag/v0.50.10
[0.25.10]: https://github.com/nostacks/ekphos/releases/tag/v0.25.10
[0.25.0]: https://github.com/nostacks/ekphos/releases/tag/v0.25.0
[0.20.10]: https://github.com/nostacks/ekphos/releases/tag/v0.20.10
