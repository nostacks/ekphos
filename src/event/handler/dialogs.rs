use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogCommand {
    Submit,
    Cancel,
    Edited,
    Ignore,
}
fn apply_text_dialog_key(input: &mut String, error: &mut Option<String>, key: crossterm::event::KeyEvent, clear_error_on_edit: bool) -> DialogCommand {
    match key.code {
        KeyCode::Enter => DialogCommand::Submit,
        KeyCode::Esc => DialogCommand::Cancel,
        KeyCode::Char(ch) => {
            input.push(ch);
            if clear_error_on_edit {
                *error = None;
            }
            DialogCommand::Edited
        }
        KeyCode::Backspace => {
            input.pop();
            if clear_error_on_edit {
                *error = None;
            }
            DialogCommand::Edited
        }
        _ => DialogCommand::Ignore,
    }
}

/// Returns true if the app should quit.
pub(super) fn handle_key_event(app: &mut App, key: crossterm::event::KeyEvent) -> io::Result<bool> {
    if app.state.keybinding_warning.is_some() {
        handle_keybinding_warning(app, key);
        return Ok(false);
    }
    match app.state.dialog {
        DialogState::Onboarding => {
            app.state.keymap.reset_pending();
            handle_onboarding_dialog(app, key);
            return Ok(false);
        }
        DialogState::CreateNote => {
            app.state.keymap.reset_pending();
            handle_create_note_dialog(app, key);
            return Ok(false);
        }
        DialogState::CreateFolder => {
            app.state.keymap.reset_pending();
            handle_create_folder_dialog(app, key);
            return Ok(false);
        }
        DialogState::CreateNoteInFolder => {
            app.state.keymap.reset_pending();
            handle_create_note_in_folder_dialog(app, key);
            return Ok(false);
        }
        DialogState::DeleteConfirm => {
            app.state.keymap.reset_pending();
            handle_delete_confirm_dialog(app, key);
            return Ok(false);
        }
        DialogState::DeleteFolderConfirm => {
            app.state.keymap.reset_pending();
            handle_delete_folder_confirm_dialog(app, key);
            return Ok(false);
        }
        DialogState::RenameNote => {
            app.state.keymap.reset_pending();
            handle_rename_note_dialog(app, key);
            return Ok(false);
        }
        DialogState::RenameFolder => {
            app.state.keymap.reset_pending();
            handle_rename_folder_dialog(app, key);
            return Ok(false);
        }
        DialogState::Help => {
            app.state.keymap.reset_pending();
            handle_help_dialog(app, key);
            return Ok(false);
        }
        DialogState::EmptyDirectory => {
            app.state.keymap.reset_pending();
            handle_empty_directory_dialog(app, key);
            return Ok(false);
        }
        DialogState::DirectoryNotFound => {
            app.state.keymap.reset_pending();
            return Ok(handle_directory_not_found_dialog(app, key));
        }
        DialogState::UnsavedChanges => {
            app.state.keymap.reset_pending();
            handle_unsaved_changes_dialog(app, key);
            return Ok(false);
        }
        DialogState::CreateWikiNote => {
            app.state.keymap.reset_pending();
            handle_create_wiki_note_dialog(app, key);
            return Ok(false);
        }
        DialogState::GraphView => {
            app.state.keymap.reset_pending();
            handle_graph_view_dialog(app, key);
            return Ok(false);
        }
        DialogState::TaskView => {
            app.state.keymap.reset_pending();
            handle_task_view_dialog(app, key);
            return Ok(false);
        }
        DialogState::ThemeSelector => {
            app.state.keymap.reset_pending();
            handle_theme_selector_dialog(app, key);
            return Ok(false);
        }
        DialogState::None => {}
    }
    if app.state.show_welcome {
        app.state.keymap.reset_pending();
        handle_welcome_dialog(app, key);
        return Ok(false);
    }
    if !matches!(app.search.search_picker, SearchPickerState::Closed) {
        app.state.keymap.reset_pending();
        handle_search_picker_input(app, key);
        return Ok(false);
    }
    if app.search.search_active {
        app.state.keymap.reset_pending();
        handle_search_input(app, key);
        return Ok(false);
    }
    if app.search.buffer_search.active {
        app.state.keymap.reset_pending();
        handle_buffer_search_input(app, key);
        return Ok(false);
    }
    match app.editor.mode {
        Mode::Normal => {
            if handle_normal_mode(app, key) {
                return Ok(true);
            }
        }
        Mode::Edit => {
            handle_edit_mode(app, key);
        }
    }
    Ok(false)
}

pub(super) fn handle_keybinding_warning(app: &mut App, key: crossterm::event::KeyEvent) {
    app.state.keymap.reset_pending();
    if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
        app.state.keybinding_warning = None;
        return;
    }
    let Some(warning) = app.state.keybinding_warning.as_mut() else {
        return;
    };
    let max_scroll = warning.issues.len().saturating_mul(8).saturating_sub(1);
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            warning.scroll = warning.scroll.saturating_add(1).min(max_scroll);
        }
        KeyCode::Up | KeyCode::Char('k') => warning.scroll = warning.scroll.saturating_sub(1),
        KeyCode::PageDown => {
            warning.scroll = warning.scroll.saturating_add(5).min(max_scroll);
        }
        KeyCode::PageUp => warning.scroll = warning.scroll.saturating_sub(5),
        _ => {}
    }
}

pub(super) fn handle_onboarding_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.complete_onboarding();
        }
        KeyCode::Char(c) => {
            app.state.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.state.input_buffer.pop();
        }
        _ => {}
    }
}

pub(super) fn handle_create_note_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match apply_text_dialog_key(&mut app.state.input_buffer, &mut app.state.dialog_error, key, true) {
        DialogCommand::Submit => {
            let name = app.state.input_buffer.trim().to_string();
            if name.is_empty() {
                app.state.dialog_error = Some("Note name cannot be empty".to_string());
                return;
            }
            if app.create_note(&name) {
                app.state.input_buffer.clear();
                app.state.dialog_error = None;
                app.state.dialog = DialogState::None;
            }
        }
        DialogCommand::Cancel => {
            app.state.input_buffer.clear();
            app.vault.target_folder = None;
            app.state.dialog_error = None;
            app.state.dialog = DialogState::None;
        }
        DialogCommand::Edited | DialogCommand::Ignore => {}
    }
}

pub(super) fn handle_create_folder_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match apply_text_dialog_key(&mut app.state.input_buffer, &mut app.state.dialog_error, key, true) {
        DialogCommand::Submit => {
            let name = app.state.input_buffer.trim().to_string();
            if name.is_empty() {
                app.state.dialog_error = Some("Folder name cannot be empty".to_string());
                return;
            }
            if app.create_folder(&name) {
                app.state.input_buffer.clear();
                app.state.dialog_error = None;
                app.state.dialog = DialogState::CreateNoteInFolder;
            }
        }
        DialogCommand::Cancel => {
            app.state.input_buffer.clear();
            app.state.dialog_error = None;
            app.vault.target_folder = None;
            app.state.dialog = DialogState::None;
        }
        DialogCommand::Edited | DialogCommand::Ignore => {}
    }
}

pub(super) fn handle_create_note_in_folder_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match apply_text_dialog_key(&mut app.state.input_buffer, &mut app.state.dialog_error, key, true) {
        DialogCommand::Submit => {
            let name = app.state.input_buffer.trim().to_string();
            if name.is_empty() {
                app.state.dialog_error = Some("Note name cannot be empty".to_string());
                return;
            }
            if app.create_note(&name) {
                app.state.input_buffer.clear();
                app.state.dialog_error = None;
                app.state.dialog = DialogState::None;
            }
        }
        DialogCommand::Cancel => {
            app.state.input_buffer.clear();
            app.vault.target_folder = None;
            app.state.dialog_error = None;
            app.state.dialog = DialogState::None;
            app.load_notes_from_dir();
        }
        DialogCommand::Edited | DialogCommand::Ignore => {}
    }
}

pub(super) fn handle_delete_confirm_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_current_note();
            app.state.dialog = DialogState::None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state.dialog = DialogState::None;
        }
        _ => {}
    }
}

pub(super) fn handle_delete_folder_confirm_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_current_folder();
            app.state.dialog = DialogState::None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state.dialog = DialogState::None;
        }
        _ => {}
    }
}

pub(super) fn handle_unsaved_changes_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('y') | KeyCode::Char('Y') => {
            if app.save_edit() {
                app.editor.vim.mode = VimMode::Normal;
                update_cursor_style(app);
                app.state.dialog = DialogState::None;
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.cancel_edit();
            app.editor.vim.mode = VimMode::Normal;
            update_cursor_style(app);
            app.state.dialog = DialogState::None;
        }
        KeyCode::Esc => {
            app.state.dialog = DialogState::None;
        }
        _ => {}
    }
}

pub(super) fn handle_create_wiki_note_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(target) = app.editor.pending_wiki_target.take() {
                app.create_note_from_wiki_target(&target);
            }
            app.state.dialog = DialogState::None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.editor.pending_wiki_target = None;
            app.state.dialog = DialogState::None;
        }
        _ => {}
    }
}

pub(super) fn handle_wiki_autocomplete(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    let is_open = matches!(app.editor.wiki_autocomplete, WikiAutocompleteState::Open { .. });
    if !is_open {
        return false;
    }
    let (query, suggestions_len, mode, target_note) = if let WikiAutocompleteState::Open { ref query, ref suggestions, ref mode, ref target_note, .. } = app.editor.wiki_autocomplete {
        (query.clone(), suggestions.len(), mode.clone(), target_note.clone())
    } else {
        return false;
    };
    match key.code {
        KeyCode::Esc => {
            app.editor.wiki_autocomplete = WikiAutocompleteState::None;
            true
        }
        KeyCode::Enter | KeyCode::Tab => {
            if mode == WikiAutocompleteMode::Alias {
                let (row, col) = app.editor.cursor();
                let already_closed = app.editor.line(row).is_some_and(|line| line.chars().nth(col) == Some(']') && line.chars().nth(col + 1) == Some(']'));
                if !already_closed {
                    app.editor.insert_str("]]");
                }
                app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                app.update_editor_highlights();
                return true;
            }
            let suggestion = if let WikiAutocompleteState::Open { ref suggestions, selected_index, .. } = app.editor.wiki_autocomplete { suggestions.get(selected_index).cloned() } else { None };
            if let Some(suggestion) = suggestion {
                let chars_to_delete = match mode {
                    WikiAutocompleteMode::Note => query.chars().count(),
                    WikiAutocompleteMode::Heading => query.chars().count(),
                    WikiAutocompleteMode::Alias => 0,
                };
                for _ in 0..chars_to_delete {
                    app.editor.delete_newline();
                }
                if mode == WikiAutocompleteMode::Heading {
                    app.editor.insert_str(&suggestion.insert_text);
                    let already_closed = {
                        let (row, col) = app.editor.cursor();
                        app.editor.line(row).is_some_and(|line| line.chars().nth(col) == Some(']') && line.chars().nth(col + 1) == Some(']'))
                    };
                    if !already_closed {
                        app.editor.insert_str("]]");
                    }
                    app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                    app.update_editor_highlights();
                } else if suggestion.is_folder {
                    app.editor.insert_str(&suggestion.insert_text);
                    let new_query = suggestion.insert_text.clone();
                    let new_suggestions = app.build_wiki_suggestions(&new_query);
                    app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: new_query, suggestions: new_suggestions, selected_index: 0, mode: WikiAutocompleteMode::Note, target_note: None };
                } else {
                    app.editor.insert_str(&suggestion.insert_text);
                    let already_closed = {
                        let (row, col) = app.editor.cursor();
                        app.editor.line(row).is_some_and(|line| line.chars().nth(col) == Some(']') && line.chars().nth(col + 1) == Some(']'))
                    };
                    if !already_closed {
                        app.editor.insert_str("]]");
                    }
                    app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                    app.update_editor_highlights();
                }
            }
            true
        }
        KeyCode::Down => {
            if mode != WikiAutocompleteMode::Alias && suggestions_len > 0 {
                if let WikiAutocompleteState::Open { ref mut selected_index, .. } = app.editor.wiki_autocomplete {
                    *selected_index = (*selected_index + 1) % suggestions_len;
                }
            }
            true
        }
        KeyCode::Up => {
            if mode != WikiAutocompleteMode::Alias && suggestions_len > 0 {
                if let WikiAutocompleteState::Open { ref mut selected_index, .. } = app.editor.wiki_autocomplete {
                    *selected_index = if *selected_index == 0 { suggestions_len - 1 } else { *selected_index - 1 };
                }
            }
            true
        }
        KeyCode::Backspace => {
            if query.is_empty() {
                match mode {
                    WikiAutocompleteMode::Note => {
                        app.editor.delete_newline(); // Delete first [
                        app.editor.delete_newline(); // Delete second [
                        app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                    }
                    WikiAutocompleteMode::Heading => {
                        app.editor.delete_newline();
                        if let Some(ref target) = target_note {
                            let new_suggestions = app.build_wiki_suggestions(target);
                            app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: target.clone(), suggestions: new_suggestions, selected_index: 0, mode: WikiAutocompleteMode::Note, target_note: None };
                        } else {
                            app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                        }
                    }
                    WikiAutocompleteMode::Alias => {
                        app.editor.delete_newline();
                        if let Some(ref target) = target_note {
                            if target.contains('#') {
                                let (note_part, heading_part) = target.split_once('#').unwrap_or((target, ""));
                                let heading_suggestions = app.build_heading_suggestions(note_part, heading_part);
                                app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: heading_part.to_string(), suggestions: heading_suggestions, selected_index: 0, mode: WikiAutocompleteMode::Heading, target_note: Some(note_part.to_string()) };
                            } else {
                                let new_suggestions = app.build_wiki_suggestions(target);
                                app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: target.clone(), suggestions: new_suggestions, selected_index: 0, mode: WikiAutocompleteMode::Note, target_note: None };
                            }
                        } else {
                            app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                        }
                    }
                }
            } else {
                let mut new_query = query.clone();
                new_query.pop();
                app.editor.delete_newline();
                let new_suggestions = match mode {
                    WikiAutocompleteMode::Note => app.build_wiki_suggestions(&new_query),
                    WikiAutocompleteMode::Heading => {
                        if let Some(ref target) = target_note {
                            app.build_heading_suggestions(target, &new_query)
                        } else {
                            Vec::new()
                        }
                    }
                    WikiAutocompleteMode::Alias => Vec::new(), // No suggestions in alias mode
                };
                app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: new_query, suggestions: new_suggestions, selected_index: 0, mode: mode.clone(), target_note: target_note.clone() };
            }
            true
        }
        KeyCode::Char(']') => {
            app.editor.insert_char(']');
            let (row, col) = app.editor.cursor();
            if let Some(line) = app.editor.line(row) {
                if col >= 2 && line.chars().nth(col.saturating_sub(2)) == Some(']') && line.chars().nth(col.saturating_sub(1)) == Some(']') {
                    app.editor.wiki_autocomplete = WikiAutocompleteState::None;
                    app.update_editor_highlights();
                }
            }
            true
        }
        KeyCode::Char('#') if mode == WikiAutocompleteMode::Note => {
            let note_target = query.clone();
            app.editor.insert_char('#');
            let heading_suggestions = app.build_heading_suggestions(&note_target, "");
            app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: String::new(), suggestions: heading_suggestions, selected_index: 0, mode: WikiAutocompleteMode::Heading, target_note: Some(note_target) };
            true
        }
        KeyCode::Char('|') if mode == WikiAutocompleteMode::Note || mode == WikiAutocompleteMode::Heading => {
            app.editor.insert_char('|');
            let full_target = if mode == WikiAutocompleteMode::Heading {
                if let Some(ref target) = target_note {
                    format!("{}#{}", target, query)
                } else {
                    query.clone()
                }
            } else {
                query.clone()
            };
            app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: String::new(), suggestions: Vec::new(), selected_index: 0, mode: WikiAutocompleteMode::Alias, target_note: Some(full_target) };
            true
        }
        KeyCode::Char(c) => {
            let mut new_query = query.clone();
            new_query.push(c);
            app.editor.insert_char(c);
            let new_suggestions = match mode {
                WikiAutocompleteMode::Note => app.build_wiki_suggestions(&new_query),
                WikiAutocompleteMode::Heading => {
                    if let Some(ref target) = target_note {
                        app.build_heading_suggestions(target, &new_query)
                    } else {
                        Vec::new()
                    }
                }
                WikiAutocompleteMode::Alias => Vec::new(),
            };
            app.editor.wiki_autocomplete = WikiAutocompleteState::Open { trigger_pos: (0, 0), query: new_query, suggestions: new_suggestions, selected_index: 0, mode: mode.clone(), target_note: target_note.clone() };
            true
        }
        _ => {
            app.editor.wiki_autocomplete = WikiAutocompleteState::None;
            false
        }
    }
}

pub(super) fn handle_rename_note_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match apply_text_dialog_key(&mut app.state.input_buffer, &mut app.state.dialog_error, key, false) {
        DialogCommand::Submit => {
            let new_name = app.state.input_buffer.clone();
            if app.rename_note(&new_name) {
                app.state.input_buffer.clear();
                app.state.dialog_error = None;
                app.state.dialog = DialogState::None;
            }
        }
        DialogCommand::Cancel => {
            app.state.input_buffer.clear();
            app.state.dialog = DialogState::None;
        }
        DialogCommand::Edited | DialogCommand::Ignore => {}
    }
}

pub(super) fn handle_rename_folder_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match apply_text_dialog_key(&mut app.state.input_buffer, &mut app.state.dialog_error, key, true) {
        DialogCommand::Submit => {
            let new_name = app.state.input_buffer.clone();
            if app.rename_folder(&new_name) {
                app.state.input_buffer.clear();
                app.state.dialog_error = None;
                app.state.dialog = DialogState::None;
            }
        }
        DialogCommand::Cancel => {
            app.state.input_buffer.clear();
            app.state.dialog_error = None;
            app.state.dialog = DialogState::None;
        }
        DialogCommand::Edited | DialogCommand::Ignore => {}
    }
}

pub(super) fn handle_help_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    const MAX_HELP_LINES: usize = 90;
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.state.help_scroll = 0;
            app.state.dialog = DialogState::None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.state.help_scroll = app.state.help_scroll.saturating_add(1).min(MAX_HELP_LINES);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.state.help_scroll = app.state.help_scroll.saturating_sub(1);
        }
        KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            app.state.help_scroll = app.state.help_scroll.saturating_add(10).min(MAX_HELP_LINES);
        }
        KeyCode::Char('u') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            app.state.help_scroll = app.state.help_scroll.saturating_sub(10);
        }
        KeyCode::Char('g') => {
            app.state.help_scroll = 0;
        }
        KeyCode::Char('G') => {
            app.state.help_scroll = MAX_HELP_LINES;
        }
        _ => {}
    }
}

/// Zoom the graph view, anchoring on the selected node or graph center
pub(super) fn handle_empty_directory_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Esc => {
            app.state.dialog = DialogState::None;
        }
        KeyCode::Char('n') => {
            app.state.dialog = DialogState::None;
            app.state.input_buffer.clear();
            app.state.dialog = DialogState::CreateNote;
        }
        _ => {}
    }
}

pub(super) fn handle_directory_not_found_dialog(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.create_notes_directory();
            false
        }
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => true,
        _ => false,
    }
}

pub(super) fn handle_welcome_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
            app.dismiss_welcome();
        }
        _ => {}
    }
}

pub(super) fn handle_task_view_dialog(app: &mut App, key: crossterm::event::KeyEvent) {
    if app.tasks.text_input_active {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => app.tasks.text_input_active = false,
            KeyCode::Backspace => {
                app.tasks.query.pop();
                app.refilter_tasks();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.tasks.query.clear();
                app.refilter_tasks();
            }
            KeyCode::Char(ch) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) => {
                app.tasks.query.push(ch);
                app.refilter_tasks();
            }
            _ => {}
        }
        return;
    }
    if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) {
        return;
    }
    match key.code {
        KeyCode::Esc => {
            if app.tasks.query.is_empty() {
                app.close_task_view();
            } else {
                app.tasks.query.clear();
                app.refilter_tasks();
            }
        }
        KeyCode::Char('q') => app.close_task_view(),
        KeyCode::Down | KeyCode::Char('j') => app.task_move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.task_move_selection(-1),
        KeyCode::PageDown | KeyCode::Char('J') => app.task_move_selection(page_step(app)),
        KeyCode::PageUp | KeyCode::Char('K') => app.task_move_selection(-page_step(app)),
        KeyCode::Home | KeyCode::Char('g') => app.task_select_first(),
        KeyCode::End | KeyCode::Char('G') => app.task_select_last(),
        KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_task_from_view(),
        KeyCode::Enter => app.open_task_source(),
        KeyCode::Char('f') => app.cycle_task_filter(TaskFilterKind::Status),
        KeyCode::Char('d') => app.cycle_task_filter(TaskFilterKind::Due),
        KeyCode::Char('p') => app.cycle_task_filter(TaskFilterKind::Priority),
        KeyCode::Char('/') => app.cycle_task_filter(TaskFilterKind::Search),
        KeyCode::Char('c') => app.clear_task_filters(),
        KeyCode::Char('r') => app.mark_tasks_dirty(),
        _ => {}
    }
}

fn page_step(app: &App) -> isize {
    isize::try_from(app.tasks.list_area.height).unwrap_or(10).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn text_dialog_commands_preserve_existing_key_behavior() {
        let mut input = String::from("ab");
        let mut error = Some(String::from("invalid"));
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Char('c')), true), DialogCommand::Edited);
        assert_eq!(input, "abc");
        assert_eq!(error, None);
        error = Some(String::from("invalid"));
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Backspace), true), DialogCommand::Edited);
        assert_eq!(input, "ab");
        assert_eq!(error, None);
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Enter), true), DialogCommand::Submit);
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Esc), true), DialogCommand::Cancel);
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Left), true), DialogCommand::Ignore);
    }

    fn task_view_app() -> (App, std::path::PathBuf) {
        use crate::app::AppDependencies;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("ekphos-tasks-{}-{id}", std::process::id()));
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(vault.join("fixture.md"), "# Fixture\n\n- [ ] alpha 📅 2026-06-01 ⏫\n").unwrap();
        std::fs::write(vault.join("other.md"), "# Other\n\n- [ ] beta\n- [x] gamma ✅ 2026-01-01\n").unwrap();
        let config = Config { general: crate::config::GeneralConfig { welcome_shown: false, check_updates: false, ..Default::default() }, ..Default::default() };
        let dependencies = AppDependencies::headless(root.join("config"), root.join("cache"));
        let mut app = App::new_injected(config, vault.clone(), None, dependencies);
        app.state.show_welcome = false;
        app.state.dialog = DialogState::None;
        app.open_task_view();
        let started = std::time::Instant::now();
        while (app.tasks_loading() || !app.tasks.scanned_once()) && started.elapsed() < std::time::Duration::from_secs(5) {
            app.poll_background();
            std::thread::yield_now();
        }
        (app, vault)
    }

    fn visible_texts(app: &App) -> Vec<String> {
        app.tasks.visible.iter().map(|&index| app.tasks.tasks[index].text.clone()).collect()
    }

    #[test]
    fn task_view_keys_filter_search_and_close() {
        let (mut app, _vault) = task_view_app();
        assert_eq!(visible_texts(&app), ["alpha", "beta"]);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.tasks.status, crate::app::TaskStatusFilter::Done);
        assert_eq!(visible_texts(&app), ["gamma"]);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.tasks.status, crate::app::TaskStatusFilter::All);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('p')));
        assert_eq!(visible_texts(&app), ["alpha"]);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('c')));
        assert_eq!(visible_texts(&app), ["alpha", "beta"]);
        assert!(!app.tasks.has_active_filters());

        handle_task_view_dialog(&mut app, key(KeyCode::Char('/')));
        assert!(app.tasks.text_input_active);
        for ch in "be".chars() {
            handle_task_view_dialog(&mut app, key(KeyCode::Char(ch)));
        }
        assert_eq!(visible_texts(&app), ["beta"]);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tasks.query, "bej", "typing must go to the query, not navigation");
        handle_task_view_dialog(&mut app, key(KeyCode::Backspace));
        handle_task_view_dialog(&mut app, key(KeyCode::Enter));
        assert!(!app.tasks.text_input_active);
        assert_eq!(visible_texts(&app), ["beta"]);
        handle_task_view_dialog(&mut app, key(KeyCode::Esc));
        assert!(app.tasks.query.is_empty());
        assert_eq!(app.state.dialog, DialogState::TaskView);
        handle_task_view_dialog(&mut app, key(KeyCode::Esc));
        assert_eq!(app.state.dialog, DialogState::None);
    }

    #[test]
    fn task_view_navigation_and_toggle_write_through_to_disk() {
        let (mut app, vault) = task_view_app();
        handle_task_view_dialog(&mut app, key(KeyCode::Char('G')));
        assert_eq!(app.tasks.selected, 1);
        handle_task_view_dialog(&mut app, key(KeyCode::Char('g')));
        assert_eq!(app.tasks.selected, 0);
        handle_task_view_dialog(&mut app, key(KeyCode::Down));
        assert_eq!(app.tasks.selected_task().map(|task| task.text.as_str()), Some("beta"));
        handle_task_view_dialog(&mut app, key(KeyCode::Down));
        assert_eq!(app.tasks.selected, 1, "selection must clamp at the last row");
        let today = app.today();
        handle_task_view_dialog(&mut app, key(KeyCode::Char(' ')));
        let body = std::fs::read_to_string(vault.join("other.md")).unwrap();
        assert_eq!(body, format!("# Other\n\n- [x] beta ✅ {today}\n- [x] gamma ✅ 2026-01-01\n"));
        let started = std::time::Instant::now();
        while app.tasks.visible.len() != 1 && started.elapsed() < std::time::Duration::from_secs(5) {
            app.poll_background();
            std::thread::yield_now();
        }
        assert_eq!(visible_texts(&app), ["alpha"]);
    }

    #[test]
    fn task_view_toggle_refuses_stale_lines() {
        let (mut app, vault) = task_view_app();
        handle_task_view_dialog(&mut app, key(KeyCode::Char('G')));
        let edited = "# Other\n\nintro\n- [ ] beta\n";
        std::fs::write(vault.join("other.md"), edited).unwrap();
        handle_task_view_dialog(&mut app, key(KeyCode::Char(' ')));
        assert_eq!(std::fs::read_to_string(vault.join("other.md")).unwrap(), edited, "a moved task must not rewrite the wrong line");
        assert!(app.state.toast.is_some());
    }

    #[test]
    fn rename_note_editing_keeps_its_existing_error_policy() {
        let mut input = String::from("old");
        let mut error = Some(String::from("unchanged"));
        assert_eq!(apply_text_dialog_key(&mut input, &mut error, key(KeyCode::Char('x')), false), DialogCommand::Edited);
        assert_eq!(input, "oldx");
        assert_eq!(error.as_deref(), Some("unchanged"));
    }
}
