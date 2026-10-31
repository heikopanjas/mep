//! Screen rendering (IMPLEMENTATION_PLAN.md §6.2–§6.4; M guide p. 7, p. 13)

use crate::{
    cells::{self, GlyphKind},
    editor::{Editor, MessageKind, Pending},
    file::FileName,
    frame::{CursorShape, Frame},
    view::Pos
};

/// Returns the file type shown on the status line, from the extension
fn file_type(name: &FileName) -> &'static str
{
    let extension = name.path().and_then(|path| path.extension()).map(|extension| extension.to_string_lossy().to_lowercase());
    match extension.as_deref()
    {
        | Some("c" | "h") => "C",
        | Some("asm") => "asm",
        | Some("pas") => "pascal",
        | Some("bas") => "basic",
        | Some("for") => "fortran",
        | _ => "text"
    }
}

impl Editor
{
    /// Returns the status line (M guide p. 13: up to eight fields)
    // ORACLE: STATUS-1
    pub fn status(&self) -> String
    {
        let file = self.file();
        let view = self.view();
        let mut kind = file_type(&file.name).to_string();
        if file.modified == true
        {
            kind.push_str(" modified");
        }
        if file.crlf == false
        {
            kind.push_str(" NL");
        }
        let (label, pos) = match self.switches.displaycursor
        {
            | true => ("Cursor", view.cursor),
            | false => ("Window", Pos::new(view.top, view.left))
        };
        let mut status = format!("{} ({kind}) Length={}  {label}=({},{})", file.name.display(), self.length(), pos.line + 1, pos.col + 1);
        if self.insert == true
        {
            status.push_str(" insert");
        }
        if self.meta == true
        {
            status.push_str(" meta");
        }
        status
    }

    /// Returns `true` if a cell lies in the selection or the highlighted match
    fn highlighted(&self, pos: Pos) -> bool
    {
        if let Some((start, width)) = self.highlight &&
            pos.line == start.line &&
            (start.col..start.col + width).contains(&pos.col)
        {
            return true;
        }
        let Some(arg) = self.arg.as_ref().filter(|arg| arg.text.is_none())
        else
        {
            return false;
        };
        let (anchor, cursor) = (arg.anchor, self.cursor());
        let (top, bottom) = (anchor.line.min(cursor.line), anchor.line.max(cursor.line));
        let (left, right) = (anchor.col.min(cursor.col), anchor.col.max(cursor.col));
        if anchor == cursor
        {
            pos == anchor
        }
        else if anchor.col == cursor.col
        {
            (top..=bottom).contains(&pos.line)
        }
        else
        {
            (top..=bottom).contains(&pos.line) && (left..right).contains(&pos.col)
        }
    }

    /// Draws one file line into a window row
    fn draw_line(&self, frame: &mut Frame, row: u16, line: usize)
    {
        let view = self.view();
        let width = self.width();
        let raw = self.file().text.line(line);
        let glyphs = cells::glyphs(&raw);
        let trailing = glyphs.iter().rposition(|glyph| glyph.kind != GlyphKind::Char(' ')).map_or(0, |last| last + 1);
        let show_trailing = self.switches.trailspace == true && self.switches.traildisp != 0;
        let fill = |code: usize| char::from_u32(code as u32).unwrap_or(' ').to_string();
        for (index, glyph) in glyphs.iter().enumerate()
        {
            let (text, attr) = match glyph.kind
            {
                | GlyphKind::Tab => (fill(self.switches.tabdisp), self.switches.fgcolor),
                | GlyphKind::Char(' ') if index >= trailing && show_trailing => (fill(self.switches.traildisp), self.switches.fgcolor),
                | GlyphKind::Char(_) => (String::from_utf8_lossy(&raw[glyph.bytes.clone()]).into_owned(), self.switches.fgcolor),
                | GlyphKind::Control(_) | GlyphKind::Invalid(_) => (char::REPLACEMENT_CHARACTER.to_string(), self.switches.errcolor)
            };
            // A tab is drawn cell by cell; any other glyph as a whole when it fits in the window.
            let pieces = match glyph.kind
            {
                | GlyphKind::Tab => glyph.width,
                | _ => 1
            };
            let piece_width = glyph.width / pieces;
            for piece in 0..pieces
            {
                let col = glyph.col + piece * piece_width;
                if col >= view.left && col + piece_width <= view.left + width
                {
                    let attr = if self.highlighted(Pos::new(line, col))
                    {
                        self.switches.hgcolor
                    }
                    else
                    {
                        attr
                    };
                    frame.put((col - view.left) as u16, row, &text, piece_width as u16, attr);
                }
            }
        }
        // Highlight virtual space of a selection beyond the end of the line.
        let end = glyphs.last().map_or(0, |glyph| glyph.col + glyph.width);
        for col in end.max(view.left)..view.left + width
        {
            if self.highlighted(Pos::new(line, col))
            {
                frame.put((col - view.left) as u16, row, " ", 1, self.switches.hgcolor);
            }
        }
    }

    /// Renders the screen
    pub fn render(&self) -> Frame
    {
        let mut frame = Frame::new(self.cols, self.rows, self.switches.fgcolor);
        let view = self.view();
        let height = self.height();
        for row in 0..height
        {
            self.draw_line(&mut frame, row as u16, view.top + row);
        }
        let dialog = self.rows.saturating_sub(2);
        let mut cursor = ((view.cursor.col.saturating_sub(view.left)) as u16, (view.cursor.line.saturating_sub(view.top)) as u16);
        match (&self.pending, self.arg.as_ref().and_then(|arg| arg.text.as_ref()))
        {
            | (Some(Pending::Confirm { prompt, .. }), _) =>
            {
                cursor = (frame.put_str(0, dialog, prompt, self.switches.infcolor), dialog);
            }
            | (Some(Pending::Name { prompt, text, .. }), _) =>
            {
                let after = frame.put_str(0, dialog, prompt, self.switches.infcolor);
                cursor = (frame.put_str(after, dialog, text, self.switches.fgcolor), dialog);
            }
            | (_, Some(text)) =>
            {
                let after = frame.put_str(0, dialog, "Arg: ", self.switches.fgcolor);
                let typed: String = text.chars.iter().collect();
                frame.put_str(after, dialog, &typed, self.switches.fgcolor);
                let before: String = text.chars[..text.cursor].iter().collect();
                cursor = (after + cells::width(before.as_bytes()) as u16, dialog);
            }
            | _ =>
            {
                if let Some((message, kind)) = &self.message
                {
                    let attr = match kind
                    {
                        | MessageKind::Info => self.switches.infcolor,
                        | MessageKind::Error => self.switches.errcolor
                    };
                    frame.put_str(0, dialog, message, attr);
                }
            }
        }
        frame.put_str(0, self.rows.saturating_sub(1), &self.status(), self.switches.stacolor);
        frame.cursor = cursor;
        frame.shape = match self.insert
        {
            | true => CursorShape::Bar,
            | false => CursorShape::Block
        };
        frame
    }
}
