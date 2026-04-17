use crate::history::History;
use crate::selection::Selection;
use crate::utils::{calculate_end_position, count_indent_units};
use ratatui_core::style::Style;
use ropey::{Rope, RopeSlice};
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub enum EditKind {
    Insert { offset: usize, text: String },
    Remove { offset: usize, text: String },
}

#[derive(Clone)]
pub struct Edit {
    pub kind: EditKind,
}

#[derive(Clone)]
pub struct EditBatch {
    pub edits: Vec<Edit>,
    pub state_before: Option<EditState>,
    pub state_after: Option<EditState>,
}

impl EditBatch {
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            state_before: None,
            state_after: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EditState {
    pub offset: usize,
    pub selection: Option<Selection>,
}

pub(crate) trait CodeLanguage<'a> {
    fn get_indent(&self) -> &'a str;
    fn get_comment_prefix(&self) -> &'a str;
    fn highlight(&self, text: &'a str) -> Vec<(Range<usize>, Style)>;
}

pub use crate::code_logos::PLAIN_TEXT;

pub struct Code<'a> {
    content: Rope,
    language: &'a dyn CodeLanguage<'a>,
    applying_history: bool,
    history: History,
    current_batch: EditBatch,
    change_callback: Option<Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>>,
}

impl<'a> Code<'a> {
    pub fn new(text: &str, language: &'a dyn CodeLanguage<'a>) -> Self {
        Self {
            content: Rope::from_str(text),
            language,
            applying_history: true,
            history: History::new(1000),
            current_batch: EditBatch::new(),
            change_callback: None,
        }
    }

    fn on_change(&mut self) {
        todo!()
    }

    pub fn point(&self, offset: usize) -> (usize, usize) {
        let row = self.content.char_to_line(offset);
        let line_start = self.content.line_to_char(row);
        let col = offset - line_start;
        (row, col)
    }

    pub fn offset(&self, row: usize, col: usize) -> usize {
        let line_start = self.content.line_to_char(row);
        line_start + col
    }

    pub fn get_content(&self) -> String {
        self.content.to_string()
    }

    pub fn slice(&self, start: usize, end: usize) -> String {
        self.content.slice(start..end).to_string()
    }

    pub fn len(&self) -> usize {
        self.content.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.content.len_lines()
    }

    pub fn len_chars(&self) -> usize {
        self.content.len_chars()
    }

    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.content.line_to_char(line_idx)
    }
    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        self.content.char_to_byte(char_idx)
    }

    pub fn line_len(&self, idx: usize) -> usize {
        let line = self.content.line(idx);
        let len = line.len_chars();
        if idx == self.content.len_lines() - 1 {
            len
        } else {
            len.saturating_sub(1)
        }
    }

    pub fn line(&self, line_idx: usize) -> RopeSlice<'_> {
        self.content.line(line_idx)
    }

    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.content.char_to_line(char_idx)
    }

    pub fn char_slice(&self, start: usize, end: usize) -> RopeSlice<'_> {
        self.content.slice(start..end)
    }

    pub fn byte_slice(&self, start: usize, end: usize) -> RopeSlice<'_> {
        self.content.byte_slice(start..end)
    }

    pub fn byte_to_line(&self, byte_idx: usize) -> usize {
        self.content.byte_to_line(byte_idx)
    }

    pub fn byte_to_char(&self, byte_idx: usize) -> usize {
        self.content.byte_to_char(byte_idx)
    }

    pub fn tx(&mut self) {
        self.current_batch = EditBatch::new();
    }

    pub fn set_state_before(&mut self, offset: usize, selection: Option<Selection>) {
        self.current_batch.state_before = Some(EditState { offset, selection });
    }

    pub fn set_state_after(&mut self, offset: usize, selection: Option<Selection>) {
        self.current_batch.state_after = Some(EditState { offset, selection });
    }

    pub fn commit(&mut self) {
        if !self.current_batch.edits.is_empty() {
            self.notify_changes(&self.current_batch.edits);
            self.history.push(self.current_batch.clone());
            self.current_batch = EditBatch::new();
        }
    }

    pub fn insert(&mut self, from: usize, text: &str) {
        self.content.insert(from, text);

        if self.applying_history {
            self.current_batch.edits.push(Edit {
                kind: EditKind::Insert {
                    offset: from,
                    text: text.to_string(),
                },
            });
        }

        self.on_change();
    }

    pub fn remove(&mut self, from: usize, to: usize) {
        let removed_text = self.content.slice(from..to).to_string();

        self.content.remove(from..to);

        if self.applying_history {
            self.current_batch.edits.push(Edit {
                kind: EditKind::Remove {
                    offset: from,
                    text: removed_text,
                },
            });
        }

        self.on_change();
    }

    pub fn is_highlight(&self) -> bool {
        true
    }

    /// Highlights the interval between `start` and `end` char indices.
    /// Returns a list of (start byte, end byte, token_name) for highlighting.
    pub fn highlight_interval(&self, start: usize, end: usize) -> Vec<(usize, usize, Style)> {
        if start > end {
            panic!("Invalid range")
        }

        // TODO: allow slice instead of String
        let text = self.content.to_string();
        let mut results = self.language.highlight(&text[start..=end]);

        results.sort_by(|a, b| {
            let len_a = a.0.end - a.0.start + 1;
            let len_b = b.0.end - b.0.start + 1;
            match len_b.cmp(&len_a) {
                std::cmp::Ordering::Equal => b.1.cmp(&a.1),
                other => other,
            }
        });

        results
            .into_iter()
            .map(|(range, value)| (range.start, range.end - 1, value))
            .collect()
    }

    pub fn undo(&mut self) -> Option<EditBatch> {
        let batch = self.history.undo()?;
        self.applying_history = false;

        for edit in batch.edits.iter().rev() {
            match edit.kind {
                EditKind::Insert { offset, ref text } => {
                    self.remove(offset, offset + text.chars().count());
                }
                EditKind::Remove { offset, ref text } => {
                    self.insert(offset, text);
                }
            }
        }

        self.applying_history = true;
        Some(batch)
    }

    pub fn redo(&mut self) -> Option<EditBatch> {
        let batch = self.history.redo()?;
        self.applying_history = false;

        for edit in &batch.edits {
            match edit.kind {
                EditKind::Insert { offset, ref text } => {
                    self.insert(offset, text);
                }
                EditKind::Remove { offset, ref text } => {
                    self.remove(offset, offset + text.chars().count());
                }
            }
        }

        self.applying_history = true;
        Some(batch)
    }

    pub fn word_boundaries(&self, pos: usize) -> (usize, usize) {
        let len = self.content.len_chars();
        if pos >= len {
            return (pos, pos);
        }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        let mut start = pos;
        while start > 0 {
            let c = self.content.char(start - 1);
            if !is_word_char(c) {
                break;
            }
            start -= 1;
        }

        let mut end = pos;
        while end < len {
            let c = self.content.char(end);
            if !is_word_char(c) {
                break;
            }
            end += 1;
        }

        (start, end)
    }

    pub fn line_boundaries(&self, pos: usize) -> (usize, usize) {
        let total_chars = self.content.len_chars();
        if pos >= total_chars {
            return (pos, pos);
        }

        let line = self.content.char_to_line(pos);
        let start = self.content.line_to_char(line);
        let end = start + self.content.line(line).len_chars();

        (start, end)
    }

    pub fn indent(&self) -> &'a str {
        self.language.get_indent()
    }

    pub fn comment(&self) -> &'a str {
        self.language.get_comment_prefix()
    }

    pub fn indentation_level(&self, line: usize, col: usize) -> usize {
        if self.indent().is_empty() {
            return 0;
        }
        let line_str = self.line(line);
        count_indent_units(line_str, &self.indent(), Some(col))
    }

    pub fn is_only_indentation_before(&self, r: usize, c: usize) -> bool {
        if r >= self.len_lines() || c == 0 {
            return false;
        }

        let line = self.line(r);
        let indent_unit = self.indent();

        if indent_unit.is_empty() {
            return line.chars().take(c).all(|ch| ch.is_whitespace());
        }

        let count_units = count_indent_units(line, &indent_unit, Some(c));
        let only_indent = count_units * indent_unit.chars().count() >= c;
        only_indent
    }

    pub fn find_indent_at_line_start(&self, line_idx: usize) -> Option<usize> {
        if line_idx >= self.len_lines() {
            return None;
        }

        let line = self.line(line_idx);
        let indent_unit = self.indent();
        if indent_unit.is_empty() {
            return None;
        }

        let count_units = count_indent_units(line, &indent_unit, None);
        let col = count_units * indent_unit.chars().count();
        if col > 0 { Some(col) } else { None }
    }

    /// Paste text with **indentation awareness**.
    ///
    /// 1. Determine the indentation level at the cursor (`base_level`).
    /// 2. The first line of the pasted block is inserted at the cursor level (trimmed).
    /// 3. Subsequent lines adjust their indentation **relative to the previous non-empty line in the pasted block**:
    ///    - Compute `diff` = change in indentation from the previous non-empty line in the source block (clamped ±1).
    ///    - Apply `diff` to `prev_nonempty_level` to calculate the new insertion level.
    /// 4. Empty lines are inserted as-is and do not affect subsequent indentation.
    ///
    /// This ensures that pasted blocks keep their relative structure while aligning to the cursor.

    /// Inserts `text` with indentation-awareness at `offset`.
    /// Returns number of characters inserted.
    pub fn smart_paste(&mut self, offset: usize, text: &str) -> usize {
        let (row, col) = self.point(offset);
        let base_level = self.indentation_level(row, col);
        let indent_unit = self.indent();

        if indent_unit.is_empty() {
            self.insert(offset, text);
            return text.chars().count();
        }

        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return 0;
        }

        // Compute indentation levels of all lines in the source block
        let mut line_levels = Vec::with_capacity(lines.len());
        for line in &lines {
            let mut lvl = 0;
            let mut rest = *line;
            while rest.starts_with(&indent_unit) {
                lvl += 1;
                rest = &rest[indent_unit.len()..];
            }
            line_levels.push(lvl);
        }

        let mut result = Vec::with_capacity(lines.len());

        let first_line_trimmed = lines[0].trim_start();
        result.push(first_line_trimmed.to_string());

        let mut prev_nonempty_level = base_level;
        let mut prev_line_level_in_block = line_levels[0];

        for i in 1..lines.len() {
            let line = lines[i];

            if line.trim().is_empty() {
                result.push(line.to_string());
                continue;
            }

            // diff relative to previous non-empty line in the source block
            let diff = (line_levels[i] as isize - prev_line_level_in_block as isize).clamp(-1, 1);
            let new_level = (prev_nonempty_level as isize + diff).max(0) as usize;
            let indents = indent_unit.repeat(new_level);
            let result_line = format!("{}{}", indents, line.trim_start());
            result.push(result_line);

            // update levels only for non-empty line
            prev_nonempty_level = new_level;
            prev_line_level_in_block = line_levels[i];
        }

        let to_insert = result.join("\n");
        self.insert(offset, &to_insert);
        to_insert.chars().count()
    }

    /// Set the change callback function for handling document changes
    pub fn set_change_callback(
        &mut self,
        callback: Box<dyn Fn(Vec<(usize, usize, usize, usize, String)>)>,
    ) {
        self.change_callback = Some(callback);
    }

    /// Notify about document changes
    fn notify_changes(&self, edits: &[Edit]) {
        if let Some(callback) = &self.change_callback {
            let mut changes = Vec::new();

            for edit in edits {
                match &edit.kind {
                    EditKind::Insert { offset, text } => {
                        let (start_row, start_col) = self.point(*offset);
                        changes.push((start_row, start_col, start_row, start_col, text.clone()));
                    }
                    EditKind::Remove { offset, text } => {
                        let (start_row, start_col) = self.point(*offset);
                        let (end_row, end_col) = calculate_end_position(start_row, start_col, text);
                        changes.push((start_row, start_col, end_row, end_col, String::new()));
                    }
                }
            }

            if !changes.is_empty() {
                callback(changes);
            }
        }
    }
}

/// An iterator over byte slices of Rope chunks.
/// This is used to feed `tree-sitter` without allocating a full `String`.
pub struct ChunksBytes<'a> {
    chunks: ropey::iter::Chunks<'a>,
}

impl<'a> Iterator for ChunksBytes<'a> {
    type Item = &'a [u8];

    /// Returns the next chunk as a byte slice.
    /// Internally converts a `&str` to a `&[u8]` without allocation.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.chunks.next().map(str::as_bytes)
    }
}

/// An implementation of a graphemes iterator, for iterating over the graphemes of a RopeSlice.
pub struct RopeGraphemes<'a> {
    text: ropey::RopeSlice<'a>,
    chunks: ropey::iter::Chunks<'a>,
    cur_chunk: &'a str,
    cur_chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    pub fn new<'b>(slice: &RopeSlice<'b>) -> RopeGraphemes<'b> {
        let mut chunks = slice.chunks();
        let first_chunk = chunks.next().unwrap_or("");
        RopeGraphemes {
            text: *slice,
            chunks: chunks,
            cur_chunk: first_chunk,
            cur_chunk_start: 0,
            cursor: GraphemeCursor::new(0, slice.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<RopeSlice<'a>> {
        let a = self.cursor.cur_cursor();
        let b;
        loop {
            match self
                .cursor
                .next_boundary(self.cur_chunk, self.cur_chunk_start)
            {
                Ok(None) => {
                    return None;
                }
                Ok(Some(n)) => {
                    b = n;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.cur_chunk_start += self.cur_chunk.len();
                    self.cur_chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(idx)) => {
                    let (chunk, byte_idx, _, _) = self.text.chunk_at_byte(idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, byte_idx);
                }
                _ => unreachable!(),
            }
        }

        if a < self.cur_chunk_start {
            let a_char = self.text.byte_to_char(a);
            let b_char = self.text.byte_to_char(b);

            Some(self.text.slice(a_char..b_char))
        } else {
            let a2 = a - self.cur_chunk_start;
            let b2 = b - self.cur_chunk_start;
            Some((&self.cur_chunk[a2..b2]).into())
        }
    }
}

pub fn grapheme_width_and_chars_len(g: RopeSlice) -> (usize, usize) {
    if let Some(g_str) = g.as_str() {
        (UnicodeWidthStr::width(g_str), g_str.chars().count())
    } else {
        let g_string = g.to_string();
        let g_str = g_string.as_str();
        (UnicodeWidthStr::width(g_str), g_str.chars().count())
    }
}

pub fn grapheme_width_and_bytes_len(g: RopeSlice) -> (usize, usize) {
    if let Some(g_str) = g.as_str() {
        (UnicodeWidthStr::width(g_str), g_str.len())
    } else {
        let g_string = g.to_string();
        let g_str = g_string.as_str();
        (UnicodeWidthStr::width(g_str), g_str.len())
    }
}

pub fn grapheme_width(g: RopeSlice) -> usize {
    if let Some(s) = g.as_str() {
        UnicodeWidthStr::width(s)
    } else {
        let s = g.to_string();
        UnicodeWidthStr::width(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let x: &dyn CodeLanguage = PLAIN_TEXT;
        let mut code = Code::new("", PLAIN_TEXT);
        code.insert(0, "Hello ");
        code.insert(6, "World");
        assert_eq!(code.content.to_string(), "Hello World");
    }

    #[test]
    fn test_remove() {
        let mut code = Code::new("Hello World", PLAIN_TEXT.clone());
        code.remove(5, 11);
        assert_eq!(code.content.to_string(), "Hello");
    }

    #[test]
    fn test_undo() {
        let mut code = Code::new("", PLAIN_TEXT.clone());

        code.tx();
        code.insert(0, "Hello ");
        code.commit();

        code.tx();
        code.insert(6, "World");
        code.commit();

        code.undo();
        assert_eq!(code.content.to_string(), "Hello ");

        code.undo();
        assert_eq!(code.content.to_string(), "");
    }

    #[test]
    fn test_redo() {
        let mut code = Code::new("", PLAIN_TEXT.clone());

        code.tx();
        code.insert(0, "Hello");
        code.commit();

        code.undo();
        assert_eq!(code.content.to_string(), "");

        code.redo();
        assert_eq!(code.content.to_string(), "Hello");
    }

    #[test]
    fn test_indentation_level0() {
        let mut code = Code::new("", PLAIN_TEXT.clone());
        code.insert(0, "    hello world");
        assert_eq!(code.indentation_level(0, 10), 0);
    }

    static INDENT_LANG: CodeLanguage<PlainTextToken> = CodeLanguage::new("    ", "#");

    #[test]
    fn test_indentation_level() {
        let mut code = Code::new("", INDENT_LANG.clone());
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 1);
    }

    #[test]
    fn test_indentation_level2() {
        let mut code = Code::new("", INDENT_LANG.clone());
        code.insert(0, "        print('Hello, World!')");
        assert_eq!(code.indentation_level(0, 10), 2);
    }

    #[test]
    fn test_is_only_indentation_before() {
        let mut code = Code::new("", INDENT_LANG.clone());
        code.insert(0, "    print('Hello, World!')");
        assert_eq!(code.is_only_indentation_before(0, 4), true);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }

    #[test]
    fn test_is_only_indentation_before2() {
        let mut code = Code::new("", PLAIN_TEXT.clone());
        code.insert(0, "    Hello, World");
        assert_eq!(code.is_only_indentation_before(0, 4), false);
        assert_eq!(code.is_only_indentation_before(0, 10), false);
    }

    #[test]
    fn test_smart_paste_1() {
        let initial = "fn foo() {\n    let x = 1;\n    \n}";
        let mut code = Code::new(initial, INDENT_LANG.clone());

        let offset = 30;
        let paste = "if start == end && start == self.code.len() {\n    return;\n}";
        code.smart_paste(offset, paste);

        let expected = "fn foo() {\n    let x = 1;\n    if start == end && start == self.code.len() {\n        return;\n    }\n}";
        assert_eq!(code.get_content(), expected);
    }

    #[test]
    fn test_smart_paste_2() {
        let initial = "fn foo() {\n    let x = 1;\n    \n}";
        let mut code = Code::new(initial, INDENT_LANG.clone());

        let offset = 30;
        let paste = "    if start == end && start == self.code.len() {\n        return;\n    }";
        code.smart_paste(offset, paste);

        let expected = "fn foo() {\n    let x = 1;\n    if start == end && start == self.code.len() {\n        return;\n    }\n}";
        assert_eq!(code.get_content(), expected);
    }
}
