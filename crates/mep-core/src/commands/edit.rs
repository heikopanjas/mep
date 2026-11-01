//! Editing functions (docs/spec/commands: graphic, insertmode, quote, cdelete, emacscdel,
//! emacsnewl, sdelete, sinsert, ldelete, linsert, pbal, undo, redo)

use crate::{
    cells,
    clip::{Clip, Shape},
    editor::{Arg, Editor, Pending, Step},
    view::Pos
};

/// Splits deleted stream text into clipboard pieces
pub(crate) fn stream_clip(bytes: &[u8]) -> Clip
{
    let lines = bytes.split(|byte| *byte == b'\n').map(|piece| piece.strip_suffix(b"\r").unwrap_or(piece).to_vec()).collect();
    Clip { shape: Shape::Stream, lines }
}

/// Returns the byte range of displayed line `line` that the stream `start..end` covers: from
/// `start.col` on the first line, up to `end.col` on the last, whole in between (argcol.md)
pub(crate) fn stream_window(shown: &[u8], line: usize, start: Pos, end: Pos) -> std::ops::Range<usize>
{
    let from = match line == start.line
    {
        | true => cells::byte_at(shown, start.col),
        | false => 0
    };
    let to = match line == end.line
    {
        | true => cells::byte_at(shown, end.col),
        | false => shown.len()
    };
    from..to.max(from)
}

/// Returns the displayed rows of the stream `start..end`, each trimmed by `stream_window`; rows
/// beyond the end of the file are capped to one, matching the beyond-EOF rule other shapes use
pub(crate) fn stream_rows(ed: &Editor, start: Pos, end: Pos) -> Vec<Vec<u8>>
{
    let cap = ed.line_count();
    (start.line.min(cap)..=end.line.min(cap))
        .map(|line| {
            let shown = ed.shown_line(line);
            shown[stream_window(&shown, line, start, end)].to_vec()
        })
        .collect()
}

/// Inserts blank space for the stream `start..end`: a line break per line crossed, then
/// enough spaces to cover the last line's columns, at `start`
fn insert_stream(ed: &mut Editor, start: Pos, end: Pos)
{
    let at = ed.prepare(start);
    let mut blanks = ed.newline().repeat(end.line - start.line);
    let columns = match end.line == start.line
    {
        | true => end.col - start.col,
        | false => end.col
    };
    blanks.extend(b" ".repeat(columns));
    ed.replace(at..at, &blanks);
    ed.set_cursor(start);
}

/// `graphic`: puts the typed character at the cursor (graphic.md)
// ORACLE: CMD-GRAPHIC-1
pub(crate) fn graphic(ed: &mut Editor, _: Arg) -> Step
{
    match ed.typed
    {
        | Some(ch) => Step::Done(ed.put_char(ch)),
        | None => Step::Done(false)
    }
}

/// `insertmode`: toggles insert mode and returns the new state (insertmode.md)
pub(crate) fn insertmode(ed: &mut Editor, _: Arg) -> Step
{
    ed.insert = ed.insert == false;
    Step::Done(ed.insert)
}

/// `quote`: the next key is put into the file literally (quote.md)
pub(crate) fn quote(_: &mut Editor, _: Arg) -> Step
{
    Step::Wait(Pending::Quote)
}

/// `undo`: reverses the last change of the current file (undo.md)
// ORACLE: UNDO-1, CMD-UNDO-1
pub(crate) fn undo(ed: &mut Editor, _: Arg) -> Step
{
    ed.undo_step();
    Step::Done(true)
}

/// `redo`: reapplies the last change `undo` undid (redo.md)
// ORACLE: REDO-1
pub(crate) fn redo(ed: &mut Editor, _: Arg) -> Step
{
    ed.redo_step();
    Step::Done(true)
}

/// Deletes or blanks the character left of the cursor; shared by `cdelete` and `emacscdel` (cdelete.md)
fn delete_left(ed: &mut Editor) -> Step
{
    let cursor = ed.cursor();
    let width = ed.line_width(cursor.line);
    if cursor.col > width
    {
        ed.set_cursor(Pos::new(cursor.line, width));
        return Step::Done(true);
    }
    if cursor.col == 0
    {
        return match cursor.line
        {
            | 0 => Step::Done(false),
            | line =>
            {
                let end = ed.line_width(line - 1);
                ed.set_cursor(Pos::new(line - 1, end));
                Step::Done(true)
            }
        };
    }
    ed.detab_line(cursor.line);
    let shown = ed.shown_line(cursor.line);
    let start = ed.line_start(cursor.line);
    let byte = cells::byte_at(&shown, cursor.col - 1);
    let glyph = cells::glyphs(&shown).into_iter().find(|glyph| glyph.bytes.start == byte).unwrap_or(cells::Glyph {
        bytes: byte..byte + 1,
        col:   cursor.col - 1,
        width: 1,
        kind:  cells::GlyphKind::Char(' ')
    });
    let blanks = match ed.insert
    {
        | true => Vec::new(),
        | false => b" ".repeat(glyph.width)
    };
    ed.replace(start + glyph.bytes.start..start + glyph.bytes.end, &blanks);
    ed.set_cursor(Pos::new(cursor.line, glyph.col));
    Step::Done(true)
}

/// `cdelete` (cdelete.md)
// ORACLE: CMD-CDELETE-1
pub(crate) fn cdelete(ed: &mut Editor, _: Arg) -> Step
{
    delete_left(ed)
}

/// `emacscdel`: like `cdelete`, but joins lines in insert mode (emacscdel.md)
pub(crate) fn emacscdel(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    if ed.insert == false || cursor.col > 0 || cursor.line == 0
    {
        return delete_left(ed);
    }
    let end = ed.line_width(cursor.line - 1);
    if cursor.line < ed.line_count()
    {
        let join = ed.file().text.line_range(cursor.line - 1).end;
        let start = ed.line_start(cursor.line);
        ed.replace(join..start, b"");
    }
    ed.set_cursor(Pos::new(cursor.line - 1, end));
    Step::Done(true)
}

/// `emacsnewl`: `newline` in overtype mode; splits the line in insert mode (emacsnewl.md)
// ORACLE: CMD-EMACSNEWL-1
pub(crate) fn emacsnewl(ed: &mut Editor, arg: Arg) -> Step
{
    if ed.insert == false
    {
        return super::movement::newline(ed, arg);
    }
    let cursor = ed.cursor();
    ed.ensure_line(cursor.line);
    let at = ed.clamped_offset(cursor);
    let end = ed.file().text.line_range(cursor.line).end;
    let moved = ed.bytes(at..end);
    let indent = ed.indent_for(cursor.line, cursor.line).min(cursor.col);
    let mut inserted = ed.newline().to_vec();
    inserted.extend(b" ".repeat(indent));
    inserted.extend(moved.iter().skip_while(|byte| **byte == b' '));
    ed.replace(at..end, &inserted);
    ed.set_cursor(Pos::new(cursor.line + 1, indent));
    Step::Done(true)
}

/// `sdelete` (sdelete.md)
pub(crate) fn sdelete(ed: &mut Editor, arg: Arg) -> Step
{
    match arg
    {
        | Arg::Null { pos, .. } =>
        {
            let at = ed.prepare(pos);
            let end = match pos.line + 1 < ed.line_count()
            {
                | true => ed.line_start(pos.line + 1),
                | false => ed.file().text.line_range(pos.line).end
            };
            let removed = ed.bytes(at..end);
            ed.replace(at..end, b"");
            ed.set_clip(stream_clip(&removed));
        }
        | Arg::Stream { start, end, .. } =>
        {
            let from = ed.prepare(start);
            let to = ed.prepare(end);
            let removed = ed.bytes(from..to);
            ed.replace(from..to, b"");
            ed.set_clip(stream_clip(&removed));
            ed.set_cursor(start);
        }
        | _ =>
        {
            let cursor = ed.cursor();
            if cursor.line < ed.line_count() && cursor.col < ed.line_width(cursor.line)
            {
                ed.detab_line(cursor.line);
                let shown = ed.shown_line(cursor.line);
                let start = ed.line_start(cursor.line);
                let byte = cells::byte_at(&shown, cursor.col);
                let end = cells::glyphs(&shown).into_iter().find(|glyph| glyph.bytes.start == byte).map_or(byte + 1, |glyph| glyph.bytes.end);
                ed.replace(start + byte..start + end, b"");
            }
        }
    }
    Step::Done(true)
}

/// `sinsert` (sinsert.md)
// ORACLE: CMD-BLOCK-1
pub(crate) fn sinsert(ed: &mut Editor, arg: Arg) -> Step
{
    match arg
    {
        | Arg::Null { pos, .. } =>
        {
            ed.ensure_line(pos.line);
            let at = ed.clamped_offset(pos);
            let newline = ed.newline();
            ed.replace(at..at, newline);
            ed.set_cursor(pos);
        }
        | Arg::Stream { start, end, .. } => insert_stream(ed, start, end),
        | _ =>
        {
            let cursor = ed.cursor();
            if cursor.col < ed.line_width(cursor.line)
            {
                let at = ed.prepare(cursor);
                ed.replace(at..at, b" ");
            }
        }
    }
    Step::Done(true)
}

/// Returns the byte range of whole lines `first..=last`, including the last line's terminator
pub(crate) fn lines_range(ed: &Editor, first: usize, last: usize) -> std::ops::Range<usize>
{
    let end = match last + 1 < ed.line_count()
    {
        | true => ed.line_start(last + 1),
        | false => ed.file().text.len()
    };
    ed.line_start(first)..end
}

/// Deletes box cells on lines `top..=bottom` and returns the rows padded to the box width
pub(crate) fn delete_box(ed: &mut Editor, top: usize, bottom: usize, left: usize, right: usize) -> Vec<Vec<u8>>
{
    let mut rows = Vec::new();
    for line in top..=bottom
    {
        ed.detab_line(line);
        let shown = ed.shown_line(line);
        let range = cells::box_range(&shown, left, right);
        let mut row = shown[range.clone()].to_vec();
        let width = cells::width(&row);
        row.extend(b" ".repeat((right - left).saturating_sub(width)));
        rows.push(row);
        if line < ed.line_count()
        {
            let start = ed.line_start(line);
            ed.replace(start + range.start..start + range.end, b"");
        }
    }
    rows
}

/// `ldelete` (ldelete.md)
// ORACLE: CMD-LDELETE-1, CMD-BLOCK-1
pub(crate) fn ldelete(ed: &mut Editor, arg: Arg) -> Step
{
    let cursor = ed.cursor();
    let clip = match arg
    {
        | Arg::Null { pos, .. } =>
        {
            let removed = match pos.line < ed.line_count()
            {
                | true =>
                {
                    let at = ed.clamped_offset(pos);
                    let end = ed.file().text.line_range(pos.line).end;
                    let removed = ed.bytes(at..end);
                    ed.replace(at..end, b"");
                    removed
                }
                | false => Vec::new()
            };
            Clip { shape: Shape::Stream, lines: vec![removed] }
        }
        | Arg::Line { first, last, .. } => match first < ed.length()
        {
            | true =>
            {
                let last = last.min(ed.last_line());
                let lines = (first..=last).map(|line| ed.file().text.line(line)).collect();
                let range = lines_range(ed, first, last);
                ed.replace(range, b"");
                ed.set_cursor(Pos::new(first, cursor.col));
                Clip { shape: Shape::Line, lines }
            }
            | false => Clip { shape: Shape::Line, lines: vec![Vec::new()] }
        },
        | Arg::Box { top, bottom, left, right, .. } =>
        {
            let rows = delete_box(ed, top, bottom, left, right);
            ed.set_cursor(Pos::new(top, left));
            Clip { shape: Shape::Box { width: right - left }, lines: rows }
        }
        | Arg::Stream { start, end, .. } if start.line == end.line =>
        {
            let rows = delete_box(ed, start.line, start.line, start.col, end.col);
            ed.set_cursor(start);
            Clip {
                shape: Shape::Stream,
                lines: rows.into_iter().map(|row| row[..row.iter().rposition(|byte| *byte != b' ').map_or(0, |last| last + 1)].to_vec()).collect()
            }
        }
        | Arg::Stream { start, end, .. } =>
        {
            let from = ed.prepare(start);
            let to = ed.clamped_offset(end);
            let removed = ed.bytes(from..to);
            ed.replace(from..to, b"");
            ed.set_cursor(start);
            stream_clip(&removed)
        }
        | _ => match cursor.line < ed.length()
        {
            | true =>
            {
                let line = ed.file().text.line(cursor.line);
                let range = lines_range(ed, cursor.line, cursor.line);
                ed.replace(range, b"");
                Clip { shape: Shape::Line, lines: vec![line] }
            }
            | false => Clip { shape: Shape::Line, lines: vec![Vec::new()] }
        }
    };
    ed.set_clip(clip);
    Step::Done(true)
}

/// Inserts `width` blanks at column `left` on lines `top..=bottom`, padding short lines first
fn insert_box(ed: &mut Editor, top: usize, bottom: usize, left: usize, width: usize)
{
    for line in top..=bottom
    {
        let at = ed.prepare(Pos::new(line, left));
        ed.replace(at..at, &b" ".repeat(width));
    }
}

/// `linsert` (linsert.md)
// ORACLE: CMD-BLOCK-1
pub(crate) fn linsert(ed: &mut Editor, arg: Arg) -> Step
{
    let cursor = ed.cursor();
    match arg
    {
        | Arg::Null { pos, .. } =>
        {
            if let Some(first) = ed.first_nonblank(pos.line)
            {
                ed.detab_line(pos.line);
                let shown = ed.shown_line(pos.line);
                let mut content = b" ".repeat(pos.col);
                content.extend_from_slice(&shown[cells::byte_at(&shown, first)..]);
                ed.set_line(pos.line, &content);
            }
        }
        | Arg::Line { first, last, .. } =>
        {
            ed.ensure_start(first);
            let at = ed.line_start(first);
            let newlines = ed.newline().repeat(last - first + 1);
            ed.replace(at..at, &newlines);
            ed.set_cursor(Pos::new(first, cursor.col));
        }
        | Arg::Box { top, bottom, left, right, .. } =>
        {
            insert_box(ed, top, bottom, left, right - left);
            ed.set_cursor(Pos::new(top, left));
        }
        | Arg::Stream { start, end, .. } => insert_stream(ed, start, end),
        | _ =>
        {
            if cursor.line < ed.length()
            {
                let at = ed.line_start(cursor.line);
                let newline = ed.newline();
                ed.replace(at..at, newline);
            }
        }
    }
    Step::Done(true)
}

/// Returns the partner of a bracket and whether it opens
fn bracket(ch: char) -> Option<(char, bool)>
{
    match ch
    {
        | '(' => Some((')', true)),
        | '[' => Some((']', true)),
        | '{' => Some(('}', true)),
        | ')' => Some(('(', false)),
        | ']' => Some(('[', false)),
        | '}' => Some(('{', false)),
        | _ => None
    }
}

/// Finds the first unbalanced bracket opening (`opening`) in `chars`, scanned in the given order
fn unbalanced(chars: impl Iterator<Item = (usize, char)>, opening: bool) -> Option<(usize, char)>
{
    let mut stack = Vec::new();
    for (at, ch) in chars
    {
        match bracket(ch)
        {
            | Some((partner, opens)) if opens != opening => stack.push(partner),
            | Some(_) => match stack.pop()
            {
                | None => return Some((at, ch)),
                | Some(expected) if expected == ch =>
                {}
                | Some(_) => return None
            },
            | None =>
            {}
        }
    }
    None
}

/// `pbal`: balances brackets (pbal.md)
// ORACLE: CMD-PBAL-1
pub(crate) fn pbal(ed: &mut Editor, arg: Arg) -> Step
{
    let cursor = ed.cursor();
    let at = ed.clamped_offset(cursor);
    let text = ed.file().text.to_bytes();
    let forward = matches!(arg, Arg::Null { .. });
    // Brackets are ASCII, so a byte scan is exact even with invalid UTF-8 around them.
    let found = match forward
    {
        | true => unbalanced(text[at..].iter().enumerate().skip(1).map(|(index, byte)| (at + index, char::from(*byte))), false),
        | false => unbalanced(text[..at].iter().enumerate().rev().map(|(index, byte)| (index, char::from(*byte))), true)
    };
    let Some((offset, ch)) = found
    else
    {
        return ed.fail("No unbalanced bracket");
    };
    let line = ed.file().text.line_of(offset);
    let raw = ed.file().text.line(line);
    let tab = ed.switches.tab_width();
    let col = cells::width(&cells::detab(&raw[..offset - ed.line_start(line)], tab));
    ed.highlight = Some((Pos::new(line, col), 1));
    let view = ed.view();
    if line < view.top || line >= view.top + ed.height()
    {
        ed.info(&String::from_utf8_lossy(&cells::detab(&raw, tab)));
    }
    if ed.meta == false
    {
        ed.put_char(bracket(ch).map_or(ch, |(partner, _)| partner));
    }
    Step::Done(true)
}
