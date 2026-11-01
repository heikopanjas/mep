//! Cursor and window movement (docs/spec/commands: left … setwindow)
//!
//! Cursor functions never take an argument; in arg mode they extend the selection.

use crate::{
    cells::{self, GlyphKind},
    editor::{Arg, Editor, Step},
    view::Pos
};

/// Moves the cursor and returns whether it moved
fn move_to(ed: &mut Editor, pos: Pos) -> Step
{
    let moved = pos != ed.cursor();
    ed.set_cursor(pos);
    Step::Done(moved)
}

/// Returns the first cell of the glyph covering `col`, or `col` itself beyond the line end
fn snap(line: &[u8], col: usize) -> usize
{
    match col < cells::width(line)
    {
        | true => cells::col_at(line, cells::byte_at(line, col)),
        | false => col
    }
}

/// `left` (left.md)
pub(crate) fn left(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let col = match ed.meta
    {
        | true => ed.view().left,
        | false => snap(&ed.shown_line(cursor.line), cursor.col.saturating_sub(1))
    };
    move_to(ed, Pos::new(cursor.line, col))
}

/// `right`: returns whether the cursor is over text afterwards (right.md)
pub(crate) fn right(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let line = ed.shown_line(cursor.line);
    let col = match ed.meta
    {
        | true => ed.view().left + ed.width().saturating_sub(1),
        | false => cells::glyphs(&line).into_iter().find(|glyph| glyph.col == cursor.col).map_or(cursor.col + 1, |glyph| glyph.col + glyph.width)
    };
    ed.set_cursor(Pos::new(cursor.line, col));
    Step::Done(col < cells::width(&line))
}

/// `up` (up.md)
pub(crate) fn up(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let line = match ed.meta
    {
        | true => ed.view().top,
        | false => cursor.line.saturating_sub(1)
    };
    move_to(ed, Pos::new(line, cursor.col))
}

/// `down` (down.md)
pub(crate) fn down(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let line = match ed.meta
    {
        | true => ed.view().top + ed.height().saturating_sub(1),
        | false => cursor.line + 1
    };
    move_to(ed, Pos::new(line, cursor.col))
}

/// `begline`: first nonblank character; `meta begline`: column 0 (begline.md)
pub(crate) fn begline(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let col = match ed.meta
    {
        | true => 0,
        | false => ed.first_nonblank(cursor.line).unwrap_or(0)
    };
    move_to(ed, Pos::new(cursor.line, col))
}

/// `endline`: right of the last nonblank character; `meta endline`: one past the window edge (endline.md)
// ORACLE: CMD-ENDLINE-1
pub(crate) fn endline(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let col = match ed.meta
    {
        | true => ed.view().left + ed.width(),
        | false =>
        {
            let line = ed.shown_line(cursor.line);
            cells::glyphs(&line).into_iter().rev().find(|glyph| glyph.kind != GlyphKind::Char(' ')).map_or(0, |glyph| glyph.col + glyph.width)
        }
    };
    move_to(ed, Pos::new(cursor.line, col))
}

/// `home`: upper-left corner of the window (home.md)
// ORACLE: CMD-HOME-1
pub(crate) fn home(ed: &mut Editor, _: Arg) -> Step
{
    let view = ed.view();
    move_to(ed, Pos::new(view.top, view.left))
}

/// Returns `true` for a word character: letter, digit or underscore
// ORACLE: CMD-PWORD-1
fn is_word(kind: GlyphKind) -> bool
{
    matches!(kind, GlyphKind::Char(ch) if ch.is_alphanumeric() || ch == '_')
}

/// Character positions of a line for word movement: each glyph, then the line end as a non-word slot
fn slots(ed: &Editor, line: usize) -> Vec<(usize, bool)>
{
    let shown = ed.shown_line(line);
    let glyphs = cells::glyphs(&shown);
    let end = glyphs.last().map_or(0, |glyph| glyph.col + glyph.width);
    glyphs.into_iter().map(|glyph| (glyph.col, is_word(glyph.kind))).chain(std::iter::once((end, false))).collect()
}

/// Returns all slots from the cursor on, in file order, lazily line by line
fn forward(ed: &Editor, from: Pos) -> impl Iterator<Item = (Pos, bool)> + '_
{
    let count = ed.line_count();
    (from.line..count)
        .flat_map(move |line| slots(ed, line).into_iter().map(move |(col, word)| (Pos::new(line, col), word)))
        .skip_while(move |(pos, _)| pos.col < snap(&ed.shown_line(from.line), from.col).min(ed.line_width(from.line)))
}

/// Returns all slots before the cursor, nearest first
fn backward(ed: &Editor, from: Pos) -> impl Iterator<Item = (Pos, bool)> + '_
{
    let last = from.line.min(ed.line_count().saturating_sub(1));
    (0..=last).rev().flat_map(move |line| slots(ed, line).into_iter().rev().map(move |(col, word)| (Pos::new(line, col), word))).filter(move |(pos, _)| *pos < from)
}

/// `pword`: start of the next word; `meta pword`: right of the current or next word (pword.md)
pub(crate) fn pword(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let mut slots = forward(ed, cursor).peekable();
    let in_word = slots.peek().is_some_and(|(pos, word)| *word && *pos == cursor);
    let target = if ed.meta == true
    {
        // Skip to a word unless already in one, then to its end.
        slots.skip_while(|(_, word)| *word == false).find(|(_, word)| *word == false).map(|(pos, _)| pos)
    }
    else
    {
        slots.skip_while(|(_, word)| *word == in_word && in_word).find(|(_, word)| *word).map(|(pos, _)| pos)
    };
    match target
    {
        | Some(pos) => move_to(ed, pos),
        | None => Step::Done(false)
    }
}

/// `mword`: start of the current or previous word; `meta mword`: right of the previous word (mword.md)
pub(crate) fn mword(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let target = if ed.meta == true
    {
        // A word end is a word character whose following slot is not a word character.
        let mut following_is_word = forward(ed, cursor).next().is_some_and(|(pos, word)| word && pos == cursor);
        let mut end = None;
        for (pos, word) in backward(ed, cursor)
        {
            let width = cells::glyphs(&ed.shown_line(pos.line)).into_iter().find(|glyph| glyph.col == pos.col).map_or(1, |glyph| glyph.width);
            let after = Pos::new(pos.line, pos.col + width);
            if word == true && following_is_word == false && after < cursor
            {
                end = Some(after);
                break;
            }
            following_is_word = word;
        }
        end
    }
    else
    {
        let mut start = None;
        for (pos, word) in backward(ed, cursor).skip_while(|(_, word)| *word == false)
        {
            if word == false
            {
                break;
            }
            start = Some(pos);
        }
        start
    };
    match target
    {
        | Some(pos) => move_to(ed, pos),
        | None => Step::Done(false)
    }
}

/// `tab`: next multiple of `tabstops` (tab.md)
pub(crate) fn tab(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let stops = ed.switches.tabstops;
    move_to(ed, Pos::new(cursor.line, (cursor.col / stops + 1) * stops))
}

/// `backtab`: previous multiple of `tabstops` (backtab.md)
pub(crate) fn backtab(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let stops = ed.switches.tabstops;
    move_to(ed, Pos::new(cursor.line, cursor.col.saturating_sub(1) / stops * stops))
}

/// `newline`: next line, column per `softcr`; `meta newline`: column 0 (newline.md)
// ORACLE: CMD-NEWLINE-1
pub(crate) fn newline(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let col = ed.indent_for(cursor.line, cursor.line + 1);
    move_to(ed, Pos::new(cursor.line + 1, col));
    Step::Done(true)
}

impl Editor
{
    /// Returns the `softcr` column for line `next` after line `from` (newline.md)
    pub(crate) fn indent_for(&self, from: usize, next: usize) -> usize
    {
        if self.meta == true || self.switches.softcr == false
        {
            return 0;
        }
        self.first_nonblank(next).or_else(|| self.first_nonblank(from)).unwrap_or(0)
    }
}

/// `savecur` (savecur.md)
pub(crate) fn savecur(ed: &mut Editor, _: Arg) -> Step
{
    ed.saved_cursor = Some((ed.current, ed.cursor()));
    Step::Done(true)
}

/// `restcur` (restcur.md)
// ORACLE: CMD-RESTCUR-1
pub(crate) fn restcur(ed: &mut Editor, _: Arg) -> Step
{
    match ed.saved_cursor
    {
        | Some((file, pos)) if file == ed.current =>
        {
            ed.set_cursor(pos);
            Step::Done(true)
        }
        | _ => Step::Done(false)
    }
}

/// Moves the window by `delta` lines, the cursor keeping its row; returns whether it moved
// ORACLE: CMD-SCROLL-1
fn scroll(ed: &mut Editor, delta: isize) -> Step
{
    let view = ed.view();
    let top = view.top.saturating_add_signed(delta).min(ed.last_line().max(view.top));
    let moved = top as isize - view.top as isize;
    let window = ed.view_mut();
    window.top = top;
    window.cursor.line = view.cursor.line.saturating_add_signed(moved);
    Step::Done(moved != 0)
}

/// Returns the line count of a window function's argument: 1 for none, n for a number
fn amount(arg: &Arg) -> Option<usize>
{
    match arg
    {
        | Arg::Num { n, .. } => Some(*n),
        | Arg::Text { text, .. } => text.parse().ok(),
        | _ => Some(1)
    }
}

/// `plines` (plines.md)
pub(crate) fn plines(ed: &mut Editor, arg: Arg) -> Step
{
    let vscroll = ed.switches.vscroll.min(ed.height().saturating_sub(1)).max(1);
    match arg
    {
        | Arg::Null { .. } =>
        {
            let cursor = ed.cursor();
            let moved = ed.view().top != cursor.line;
            ed.view_mut().top = cursor.line;
            Step::Done(moved)
        }
        | Arg::Num { n, .. } => scroll(ed, n as isize),
        | _ => scroll(ed, vscroll as isize)
    }
}

/// `mlines` (mlines.md)
pub(crate) fn mlines(ed: &mut Editor, arg: Arg) -> Step
{
    let vscroll = ed.switches.vscroll.min(ed.height().saturating_sub(1)).max(1);
    match arg
    {
        | Arg::Null { .. } =>
        {
            let cursor = ed.cursor();
            let top = (cursor.line + 1).saturating_sub(ed.height());
            let moved = ed.view().top != top;
            ed.view_mut().top = top;
            Step::Done(moved)
        }
        | Arg::Num { n, .. } => scroll(ed, -(n as isize)),
        | _ => scroll(ed, -(vscroll as isize))
    }
}

/// `begfile`: start of the file, as `arg mpage` (begfile.md)
// ORACLE: CMD-BEGFILE-1
pub(crate) fn begfile(ed: &mut Editor, _: Arg) -> Step
{
    // The window always shows the cursor, so with the cursor at the start it is already there.
    let moved = ed.cursor() != Pos::default();
    let view = ed.view_mut();
    view.top = 0;
    view.left = 0;
    view.cursor = Pos::default();
    Step::Done(moved)
}

/// `endfile`: last line of the file, as `arg ppage` (endfile.md)
// ORACLE: CMD-BEGFILE-1
pub(crate) fn endfile(ed: &mut Editor, _: Arg) -> Step
{
    let last = ed.last_line();
    move_to(ed, Pos::new(last, 0))
}

/// `ppage`: pages forward; `arg ppage`: end of the file (ppage.md)
// ORACLE: CMD-PPAGE-1
pub(crate) fn ppage(ed: &mut Editor, arg: Arg) -> Step
{
    if let Arg::Null { .. } = arg
    {
        return endfile(ed, arg);
    }
    match amount(&arg)
    {
        | Some(pages) => scroll(ed, (pages * ed.height()) as isize),
        | None => ed.fail("Invalid argument")
    }
}

/// `mpage`: pages back; `arg mpage`: start of the file (mpage.md)
pub(crate) fn mpage(ed: &mut Editor, arg: Arg) -> Step
{
    if let Arg::Null { .. } = arg
    {
        return begfile(ed, arg);
    }
    match amount(&arg)
    {
        | Some(pages) => scroll(ed, -((pages * ed.height()) as isize)),
        | None => ed.fail("Invalid argument")
    }
}

/// `ppara`: first line of the next paragraph; `meta ppara`: blank line after the paragraph (ppara.md)
// ORACLE: CMD-PPARA-1
pub(crate) fn ppara(ed: &mut Editor, _: Arg) -> Step
{
    let start = ed.cursor().line;
    let end = ed.length();
    let mut line = start;
    if ed.meta == true
    {
        while line < end && ed.is_blank(line) == true
        {
            line += 1;
        }
        while line < end && ed.is_blank(line) == false
        {
            line += 1;
        }
    }
    else
    {
        while line < end && ed.is_blank(line) == false
        {
            line += 1;
        }
        while line < end && ed.is_blank(line) == true
        {
            line += 1;
        }
    }
    move_to(ed, Pos::new(line, 0))
}

/// `mpara`: blank line above the paragraph; `meta mpara`: previous line with text (mpara.md)
// ORACLE: S4
pub(crate) fn mpara(ed: &mut Editor, _: Arg) -> Step
{
    let start = ed.cursor().line;
    if ed.meta == true
    {
        let found = (0..start).rev().find(|line| ed.is_blank(*line) == false);
        return match found
        {
            | Some(line) => move_to(ed, Pos::new(line, 0)),
            | None => Step::Done(false)
        };
    }
    let mut line = start;
    while line > 0 && ed.is_blank(line) == true
    {
        line -= 1;
    }
    while line > 0 && ed.is_blank(line) == false
    {
        line -= 1;
    }
    move_to(ed, Pos::new(line, 0))
}

/// `setwindow`: redisplay; `arg setwindow`: the anchor becomes the window's upper-left corner (setwindow.md)
pub(crate) fn setwindow(ed: &mut Editor, arg: Arg) -> Step
{
    if let Arg::Null { pos, .. } = arg
    {
        let view = ed.view_mut();
        view.top = pos.line;
        view.left = pos.col;
    }
    ed.redraw = true;
    Step::Done(true)
}
