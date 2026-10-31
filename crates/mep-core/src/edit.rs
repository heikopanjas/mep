//! Editing primitives and undo (IMPLEMENTATION_PLAN.md §5.1–§5.4, §5.7, §6.9)
//!
//! Every change of a file goes through [`Editor::replace`], which records the
//! undo delta and marks the file modified.

use std::ops::Range;

use crate::{
    cells,
    clip::Clip,
    editor::{Edit, Editor, Group},
    env::Os,
    file::FileName,
    io::RequestKind,
    text::Text,
    view::Pos
};

impl Editor
{
    /// Returns the number of lines in the current file's index
    pub(crate) fn line_count(&self) -> usize
    {
        self.file().text.line_count()
    }

    /// Returns the length in lines shown on the status line: the empty line after a final line break does not count
    pub(crate) fn length(&self) -> usize
    {
        let text = &self.file().text;
        let count = text.line_count();
        match count > 1 && text.line(count - 1).is_empty()
        {
            | true => count - 1,
            | false => count
        }
    }

    /// Returns the last line of the file, at least 0
    pub(crate) fn last_line(&self) -> usize
    {
        self.length().saturating_sub(1)
    }

    /// Returns a line as displayed: tabs expanded; empty beyond the end of the file
    pub(crate) fn shown_line(&self, line: usize) -> Vec<u8>
    {
        cells::detab(&self.file().text.line(line))
    }

    /// Returns the width of a line in cells
    pub(crate) fn line_width(&self, line: usize) -> usize
    {
        cells::width(&self.shown_line(line))
    }

    /// Returns `true` if a line is empty or holds only blanks
    pub(crate) fn is_blank(&self, line: usize) -> bool
    {
        self.shown_line(line).iter().all(|byte| *byte == b' ')
    }

    /// Returns the column of the first nonblank character of a line, or `None` for a blank line
    pub(crate) fn first_nonblank(&self, line: usize) -> Option<usize>
    {
        let shown = self.shown_line(line);
        cells::glyphs(&shown).into_iter().find(|glyph| glyph.kind != cells::GlyphKind::Char(' ')).map(|glyph| glyph.col)
    }

    /// Returns the terminator for new lines in the current file
    pub(crate) fn newline(&self) -> &'static [u8]
    {
        match self.file().crlf
        {
            | true => b"\r\n",
            | false => b"\n"
        }
    }

    /// Returns the newline of the platform, for clipboard text (plan §3.3)
    pub(crate) fn platform_newline(&self) -> &'static str
    {
        match self.env.os()
        {
            | Os::Windows => "\r\n",
            | _ => "\n"
        }
    }

    /// Returns the byte offset where line `line` starts; `line` must exist
    pub(crate) fn line_start(&self, line: usize) -> usize
    {
        self.file().text.line_range(line).start
    }

    /// Replaces a byte range of the current file, recording the change for undo
    pub(crate) fn replace(&mut self, range: Range<usize>, bytes: &[u8])
    {
        if range.is_empty() == true && bytes.is_empty() == true
        {
            return;
        }
        let cursor = self.cursor();
        let undocount = self.switches.undocount;
        let current = self.current;
        let file = self.file_mut();
        let removed = file.text.delete(range.clone()).unwrap_or_default();
        file.text.insert(range.start, bytes).unwrap_or_default();
        file.modified = true;
        let open = self.undo_file == Some(current);
        let undo = &mut self.state().undo;
        if open == false
        {
            undo.push(Group { cursor, edits: Vec::new() });
        }
        if let Some(group) = undo.last_mut()
        {
            group.edits.push(Edit { offset: range.start, removed, inserted: bytes.to_vec() });
        }
        let excess = undo.len().saturating_sub(undocount);
        undo.drain(..excess);
        self.undo_file = Some(current);
    }

    /// Appends line breaks so that `line` exists (plan §5.4)
    ///
    /// A file that ends with a line break keeps it: the new lines go before the
    /// empty line that follows the final break, so `line` is not that empty line.
    pub(crate) fn ensure_line(&mut self, line: usize)
    {
        let count = self.line_count();
        let text = &self.file().text;
        let ends_with_break = text.line(count - 1).is_empty() && count > 1;
        let needed = line + 1 + usize::from(ends_with_break);
        if needed > count
        {
            let end = text.len();
            let breaks = self.newline().repeat(needed - count);
            self.replace(end..end, &breaks);
        }
    }

    /// Appends line breaks so that line `line` starts somewhere, for inserting whole lines before it
    pub(crate) fn ensure_start(&mut self, line: usize)
    {
        let count = self.line_count();
        if line >= count
        {
            let end = self.file().text.len();
            let breaks = self.newline().repeat(line + 1 - count);
            self.replace(end..end, &breaks);
        }
    }

    /// Replaces the tabs of a line with spaces before it is edited (plan §5.2)
    pub(crate) fn detab_line(&mut self, line: usize)
    {
        if line < self.line_count()
        {
            let raw = self.file().text.line(line);
            if raw.contains(&b'\t') == true
            {
                let range = self.file().text.line_range(line);
                self.replace(range, &cells::detab(&raw));
            }
        }
    }

    /// Prepares `pos` for an edit and returns its byte offset: the line exists, has no tabs and reaches the column
    pub(crate) fn prepare(&mut self, pos: Pos) -> usize
    {
        self.ensure_line(pos.line);
        self.detab_line(pos.line);
        let line = self.shown_line(pos.line);
        let end = self.file().text.line_range(pos.line).end;
        let width = cells::width(&line);
        if pos.col > width
        {
            self.replace(end..end, &b" ".repeat(pos.col - width));
            return end + pos.col - width;
        }
        self.line_start(pos.line) + cells::byte_at(&line, pos.col)
    }

    /// Returns the byte offset of `pos` without padding: columns beyond the line end map to the end of
    /// its content, lines beyond the file to the end of the file
    pub(crate) fn clamped_offset(&mut self, pos: Pos) -> usize
    {
        if pos.line >= self.line_count()
        {
            return self.file().text.len();
        }
        self.detab_line(pos.line);
        self.line_start(pos.line) + cells::byte_at(&self.shown_line(pos.line), pos.col)
    }

    /// Returns the bytes of a range of the current file
    pub(crate) fn bytes(&self, range: Range<usize>) -> Vec<u8>
    {
        self.file().text.slice(range).unwrap_or_default()
    }

    /// Replaces the content of a line, keeping its terminator; the line must exist
    pub(crate) fn set_line(&mut self, line: usize, content: &[u8])
    {
        let range = self.file().text.line_range(line);
        if self.bytes(range.clone()) != content
        {
            self.replace(range, content);
        }
    }

    /// Puts a typed character at the cursor in insert or overtype mode (graphic.md)
    pub(crate) fn put_char(&mut self, ch: char) -> bool
    {
        let cursor = self.cursor();
        let at = self.prepare(cursor);
        let mut encoded = [0; 4];
        let bytes = ch.encode_utf8(&mut encoded).as_bytes();
        let line = self.shown_line(cursor.line);
        let glyph = cells::glyphs(&line).into_iter().find(|glyph| glyph.col == cursor.col);
        let replaced = match (self.insert, glyph)
        {
            | (false, Some(glyph)) => glyph.bytes.len(),
            | _ => 0
        };
        self.replace(at..at + replaced, bytes);
        let width = cells::glyphs(bytes).first().map_or(1, |glyph| glyph.width);
        self.set_cursor(Pos::new(cursor.line, cursor.col + width));
        if self.switches.wordwrap == true && cursor.col + width > self.switches.rmargin
        {
            self.wrap(cursor.line);
        }
        true
    }

    /// Breaks a line at the last blank before the word holding the cursor (CMD-GRAPHIC-1)
    fn wrap(&mut self, line: usize)
    {
        let cursor = self.cursor();
        let shown = self.shown_line(line);
        let glyphs = cells::glyphs(&shown);
        let blank = glyphs.iter().rev().find(|glyph| glyph.col < cursor.col && glyph.kind == cells::GlyphKind::Char(' ')).cloned();
        let indent = self.first_nonblank(line).unwrap_or(0);
        if let Some(blank) = blank &&
            blank.col >= indent
        {
            let start = self.line_start(line);
            let mut moved = self.newline().to_vec();
            moved.extend(b" ".repeat(indent));
            self.replace(start + blank.bytes.start..start + blank.bytes.end, &moved);
            self.set_cursor(Pos::new(line + 1, indent + cursor.col - blank.col - 1));
        }
    }

    /// Undoes the last change group of the current file (plan §5.7)
    pub(crate) fn undo_step(&mut self)
    {
        let Some(group) = self.state().undo.pop()
        else
        {
            return;
        };
        for edit in group.edits.iter().rev()
        {
            let text = &mut self.file_mut().text;
            text.delete(edit.offset..edit.offset + edit.inserted.len()).unwrap_or_default();
            text.insert(edit.offset, &edit.removed).unwrap_or_default();
        }
        self.set_cursor(group.cursor);
    }

    /// Puts text into the clipboard and the system clipboard; rewrites an open `<clipboard>` (plan §6.9)
    pub(crate) fn set_clip(&mut self, clip: Clip)
    {
        let payload = clip.to_payload(self.platform_newline());
        self.written = Some(payload.clone());
        self.request(RequestKind::ClipboardWrite(payload));
        let pseudo = self.files.find(&FileName::Pseudo("clipboard".into()));
        if let Some(file) = pseudo.and_then(|id| self.files.get_mut(id))
        {
            file.text = Text::from(clip_text(&clip));
            file.modified = false;
        }
        self.clip = clip;
    }
}

/// Returns the clipboard contents as file text: lines joined by LF, line shape with a final LF
pub(crate) fn clip_text(clip: &Clip) -> Vec<u8>
{
    let mut text = clip.lines.join(&b'\n');
    if clip.shape == crate::clip::Shape::Line
    {
        text.push(b'\n');
    }
    text
}
