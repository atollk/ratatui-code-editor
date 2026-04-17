use crate::code::EditKind;
use crate::editor::Editor;
use crate::selection::Selection;

pub trait Action {
    fn apply(&mut self, editor: &mut Editor);
}

pub enum DefaultAction {
    /// Moves the cursor one character to the right.
    ///
    /// If `shift` is true, the selection is extended to the new cursor position.
    /// If `shift` is false and there is an active selection, the cursor jumps
    /// to the end of the selection and the selection is cleared.
    /// Otherwise, the cursor moves one position to the right.
    MoveRight { shift: bool },

    /// Moves the cursor one character to the left.
    ///
    /// If `shift` is true, the selection is extended to the new cursor position.
    /// If `shift` is false and there is an active selection, the cursor jumps
    /// to the start of the selection and the selection is cleared.
    /// Otherwise, the cursor moves one position to the left.
    MoveLeft { shift: bool },

    /// Moves the cursor one line up.
    ///
    /// If the previous line is shorter, the cursor is placed at the end of that line.
    /// If `shift` is true, the selection is extended to the new cursor position.
    /// If `shift` is false, the selection is cleared.
    MoveUp { shift: bool },

    /// Moves the cursor one line down.
    ///
    /// If the next line is shorter, the cursor is placed at the end of that line.
    /// If `shift` is true, the selection is extended to the new cursor position.
    /// If `shift` is false, the selection is cleared.
    MoveDown { shift: bool },

    /// Inserts arbitrary text at the cursor, replacing the selection if any.
    InsertText { text: String },

    /// Inserts a newline at the cursor with automatic indentation.
    ///
    /// The indentation is computed based on the current line and column.
    InsertNewline,

    /// Deletes the selected text or the character before the cursor.
    ///
    /// - If there is a non-empty selection, deletes the selection.
    /// - If there is no selection, deletes the previous character.
    /// - If the cursor is after indentation only, deletes the entire indentation.
    Delete,

    /// Toggles line comments at the start of the selected lines.
    ///
    /// If all lines in the selection already start with the language's comment string,
    /// removes it. Otherwise prepends it. Applies to the line under the cursor if no
    /// selection exists.
    ToggleComment,

    /// Inserts indentation at the beginning of the current line or selected lines.
    Indent,

    /// Removes one indentation level from the start of the current line or selected lines.
    UnIndent,

    /// Selects the entire text in the editor.
    SelectAll,

    /// Duplicates the selected text or the current line if no selection exists.
    Duplicate,

    /// Deletes the entire line under the cursor.
    DeleteLine,

    /// Cuts the current selection: copies it to the clipboard and removes it from the editor.
    Cut,

    /// Copies the selected text to the clipboard.
    ///
    /// Does nothing if there is no active selection.
    Copy,

    /// Pastes text from the clipboard at the current cursor position.
    ///
    /// If a selection exists, it will be replaced by the pasted text.
    /// The pasted text is adjusted using language-specific indentation rules.
    Paste,

    /// Undoes the last edit in the code buffer.
    ///
    /// Restores both the cursor position and selection state
    /// from the saved editor snapshot if available.
    Undo,

    /// Redoes the last undone edit in the code buffer.
    ///
    /// Restores both the cursor position and selection state
    /// from the saved editor snapshot if available.
    Redo,
}

impl Action for DefaultAction {
    fn apply(&mut self, editor: &mut Editor) {
        match self {
            DefaultAction::MoveRight { shift } => {
                let shift = *shift;
                let cursor = editor.get_cursor();

                if !shift {
                    if let Some(sel) = editor.get_selection() {
                        if !sel.is_empty() {
                            let (_, end) = sel.sorted();
                            editor.set_cursor(end);
                            editor.clear_selection();
                            return;
                        }
                    }
                }

                if cursor < editor.code_mut().len() {
                    let new_cursor = cursor.saturating_add(1);
                    if shift {
                        editor.extend_selection(new_cursor);
                    } else {
                        editor.clear_selection();
                    }
                    editor.set_cursor(new_cursor);
                }
            }

            DefaultAction::MoveLeft { shift } => {
                let shift = *shift;
                let cursor = editor.get_cursor();

                if !shift {
                    if let Some(sel) = editor.get_selection() {
                        if !sel.is_empty() {
                            let (start, _) = sel.sorted();
                            editor.set_cursor(start);
                            editor.clear_selection();
                            return;
                        }
                    }
                }

                if cursor > 0 {
                    let new_cursor = cursor.saturating_sub(1);
                    if shift {
                        editor.extend_selection(new_cursor);
                    } else {
                        editor.clear_selection();
                    }
                    editor.set_cursor(new_cursor);
                }
            }

            DefaultAction::MoveUp { shift } => {
                let shift = *shift;
                let cursor = editor.get_cursor();
                let code = editor.code_mut();
                let (row, col) = code.point(cursor);

                if row == 0 {
                    return;
                }

                let prev_start = code.line_to_char(row - 1);
                let prev_len = code.line_len(row - 1);
                let new_col = col.min(prev_len);
                let new_cursor = prev_start + new_col;

                if shift {
                    editor.extend_selection(new_cursor);
                } else {
                    editor.clear_selection();
                }
                editor.set_cursor(new_cursor);
            }

            DefaultAction::MoveDown { shift } => {
                let shift = *shift;
                let cursor = editor.get_cursor();
                let code = editor.code_mut();
                let (row, col) = code.point(cursor);
                let is_last_line = row + 1 >= code.len_lines();
                if is_last_line {
                    return;
                }

                let next_start = code.line_to_char(row + 1);
                let next_len = code.line_len(row + 1);
                let new_col = col.min(next_len);
                let new_cursor = next_start + new_col;

                if shift {
                    editor.extend_selection(new_cursor);
                } else {
                    editor.clear_selection();
                }
                editor.set_cursor(new_cursor);
            }

            DefaultAction::InsertText { text } => {
                let text = text.clone();
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);

                if let Some(sel) = &selection {
                    if !sel.is_empty() {
                        let (start, end) = sel.sorted();
                        code.remove(start, end);
                        cursor = start;
                    }
                }
                selection = None;

                code.insert(cursor, &text);
                cursor += text.chars().count();

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::InsertNewline => {
                let cursor = editor.get_cursor();
                let code = editor.code_mut();
                let (row, col) = code.point(cursor);
                let indent_level = code.indentation_level(row, col);
                let indent_text = code.indent().repeat(indent_level);
                let text_to_insert = format!("\n{}", indent_text);
                DefaultAction::InsertText { text: text_to_insert }.apply(editor);
            }

            DefaultAction::Delete => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);

                if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (start, end) = sel.sorted();
                    code.remove(start, end);
                    cursor = start;
                    selection = None;
                } else if cursor > 0 {
                    let (row, col) = code.point(cursor);
                    if code.is_only_indentation_before(row, col) {
                        let from = cursor - col;
                        code.remove(from, cursor);
                        cursor = from;
                    } else {
                        code.remove(cursor - 1, cursor);
                        cursor -= 1;
                    }
                }

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::ToggleComment => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let selection_anchor = editor.selection_anchor();

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);

                let comment_text = code.comment();
                let comment_len = comment_text.chars().count();

                let lines_to_handle = if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (start, end) = sel.sorted();
                    let (start_row, _) = code.point(start);
                    let (end_row, _) = code.point(end);
                    (start_row..=end_row).collect::<Vec<_>>()
                } else {
                    let (row, _) = code.point(cursor);
                    vec![row]
                };

                let all_have_comment = lines_to_handle.iter().all(|&line_idx| {
                    let line_start = code.line_to_char(line_idx);
                    let line_len = code.line_len(line_idx);
                    line_start + comment_len <= line_start + line_len
                        && code.slice(line_start, line_start + comment_len) == comment_text
                });

                let mut comments_added = 0usize;
                let mut comments_removed = 0usize;

                for &line_idx in lines_to_handle.iter().rev() {
                    let start = code.line_to_char(line_idx);
                    if all_have_comment {
                        let slice = code.slice(start, start + comment_len);
                        if slice == comment_text {
                            code.remove(start, start + comment_len);
                            comments_removed += 1;
                        }
                    } else {
                        code.insert(start, &comment_text);
                        comments_added += 1;
                    }
                }

                if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (smin, _) = sel.sorted();
                    let mut anchor = selection_anchor;
                    let is_forward = anchor == smin;

                    if is_forward {
                        if !all_have_comment {
                            cursor += comment_len * comments_added;
                            anchor += comment_len;
                        } else {
                            cursor = cursor.saturating_sub(comment_len * comments_removed);
                            anchor = anchor.saturating_sub(comment_len);
                        }
                    } else {
                        if !all_have_comment {
                            cursor += comment_len;
                            anchor += comment_len * comments_added;
                        } else {
                            cursor = cursor.saturating_sub(comment_len);
                            anchor = anchor.saturating_sub(comment_len * comments_removed);
                        }
                    }

                    selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
                } else {
                    if !all_have_comment {
                        cursor += comment_len;
                    } else {
                        cursor = cursor.saturating_sub(comment_len);
                    }
                }

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::Indent => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let selection_anchor = editor.selection_anchor();

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);

                let indent_text = code.indent();

                let lines_to_handle = if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (start, end) = sel.sorted();
                    let (start_row, _) = code.point(start);
                    let (end_row, _) = code.point(end);
                    (start_row..=end_row).collect::<Vec<_>>()
                } else {
                    let (row, _) = code.point(cursor);
                    vec![row]
                };

                let mut indents_added = 0;
                for &line_idx in lines_to_handle.iter().rev() {
                    let line_start = code.line_to_char(line_idx);
                    code.insert(line_start, &indent_text);
                    indents_added += 1;
                }

                if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (smin, _) = sel.sorted();
                    let mut anchor = selection_anchor;
                    let is_forward = anchor == smin;

                    if is_forward {
                        cursor += indent_text.len() * indents_added;
                        anchor += indent_text.len();
                    } else {
                        cursor += indent_text.len();
                        anchor += indent_text.len() * indents_added;
                    }

                    selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
                } else {
                    cursor += indent_text.len();
                }

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::UnIndent => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let selection_anchor = editor.selection_anchor();

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);

                let indent_text = code.indent();
                let indent_len = indent_text.chars().count();

                let lines_to_handle = if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (start, end) = sel.sorted();
                    let (start_row, _) = code.point(start);
                    let (end_row, _) = code.point(end);
                    (start_row..=end_row).collect::<Vec<_>>()
                } else {
                    let (row, _) = code.point(cursor);
                    vec![row]
                };

                let mut lines_untabbed = 0;
                for &line_idx in lines_to_handle.iter().rev() {
                    if let Some(indent_cols) = code.find_indent_at_line_start(line_idx) {
                        let remove_count = indent_cols.min(indent_len);
                        if remove_count > 0 {
                            let line_start = code.line_to_char(line_idx);
                            code.remove(line_start, line_start + remove_count);
                            lines_untabbed += 1;
                        }
                    }
                }

                if let Some(sel) = &selection
                    && !sel.is_empty()
                {
                    let (smin, _) = sel.sorted();
                    let mut anchor = selection_anchor;
                    let is_forward = anchor == smin;

                    if is_forward {
                        cursor = cursor.saturating_sub(indent_len * lines_untabbed);
                        anchor = anchor.saturating_sub(indent_len);
                    } else {
                        cursor = cursor.saturating_sub(indent_len);
                        anchor = anchor.saturating_sub(indent_len * lines_untabbed);
                    }

                    selection = Some(Selection::from_anchor_and_cursor(anchor, cursor));
                } else {
                    cursor = cursor.saturating_sub(indent_len * lines_untabbed);
                }

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::SelectAll => {
                let from = 0;
                let code = editor.code_mut();
                let to = code.len_chars();
                let sel = Selection::new(from, to);
                editor.set_selection(Some(sel));
            }

            DefaultAction::Duplicate => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let code = editor.code_mut();

                code.tx();
                code.set_state_before(cursor, selection);

                if let Some(sel) = &selection {
                    let text = code.slice(sel.start, sel.end);
                    let insert_pos = sel.end;
                    code.insert(insert_pos, &text);
                    cursor = insert_pos + text.chars().count();
                    selection = None;
                } else {
                    let (line_start, line_end) = code.line_boundaries(cursor);
                    let line_text = code.slice(line_start, line_end);
                    let column = cursor - line_start;

                    let insert_pos = line_end;
                    let to_insert = if line_text.ends_with('\n') {
                        line_text.clone()
                    } else {
                        format!("{}\n", line_text)
                    };
                    code.insert(insert_pos, &to_insert);

                    let new_line_len = to_insert.trim_end_matches('\n').chars().count();
                    let new_column = column.min(new_line_len);
                    cursor = insert_pos + new_column;
                }

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::DeleteLine => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let code = editor.code_mut();

                let (start, end) = code.line_boundaries(cursor);

                if start == end && start == code.len() {
                    return;
                }

                code.tx();
                code.set_state_before(cursor, selection);
                code.remove(start, end);
                code.set_state_after(start, None);
                code.commit();

                cursor = start;
                selection = None;
                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::Cut => {
                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();

                let sel = match &selection {
                    Some(sel) if !sel.is_empty() => sel.clone(),
                    _ => return,
                };

                let text = editor.code_ref().slice(sel.start, sel.end);
                let _ = editor.set_clipboard(&text);

                let code = editor.code_mut();
                code.tx();
                code.set_state_before(cursor, selection);
                code.remove(sel.start, sel.end);
                code.set_state_after(sel.start, None);
                code.commit();

                cursor = sel.start;
                selection = None;
                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::Copy => {
                let selection = editor.get_selection();
                let Some(sel) = selection else { return };
                if sel.is_empty() {
                    return;
                }
                let text = editor.code_ref().slice(sel.start, sel.end);
                let _ = editor.set_clipboard(&text);
            }

            DefaultAction::Paste => {
                let Ok(text) = editor.get_clipboard() else {
                    return;
                };
                if text.is_empty() {
                    return;
                }

                let mut cursor = editor.get_cursor();
                let mut selection = editor.get_selection();
                let code = editor.code_mut();

                code.tx();
                code.set_state_before(cursor, selection);

                if let Some(sel) = &selection {
                    if !sel.is_empty() {
                        let (start, end) = sel.sorted();
                        code.remove(start, end);
                        cursor = start;
                        selection = None;
                    }
                }

                let inserted = code.smart_paste(cursor, &text);
                cursor += inserted;

                code.set_state_after(cursor, selection);
                code.commit();

                editor.set_cursor(cursor);
                editor.set_selection(selection);
                editor.reset_highlight_cache();
            }

            DefaultAction::Undo => {
                let code = editor.code_mut();
                let edits = code.undo();
                editor.reset_highlight_cache();

                let Some(batch) = edits else { return };

                if let Some(before) = batch.state_before {
                    editor.set_cursor(before.offset);
                    editor.set_selection(before.selection);
                    return;
                }

                for edit in batch.edits.iter().rev() {
                    match &edit.kind {
                        EditKind::Insert { offset, .. } => {
                            editor.set_cursor(*offset);
                        }
                        EditKind::Remove { offset, text } => {
                            editor.set_cursor(*offset + text.chars().count());
                        }
                    }
                }
            }

            DefaultAction::Redo => {
                let code = editor.code_mut();
                let edits = code.redo();
                editor.reset_highlight_cache();

                let Some(batch) = edits else { return };

                if let Some(after) = batch.state_after {
                    editor.set_cursor(after.offset);
                    editor.set_selection(after.selection);
                    return;
                }

                for edit in batch.edits {
                    match &edit.kind {
                        EditKind::Insert { offset, text } => {
                            editor.set_cursor(*offset + text.chars().count());
                        }
                        EditKind::Remove { offset, .. } => {
                            editor.set_cursor(*offset);
                        }
                    }
                }
            }
        }
    }
}