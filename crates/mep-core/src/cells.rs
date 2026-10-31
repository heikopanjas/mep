//! Display cells of a line (IMPLEMENTATION_PLAN.md §3.2, §5.2)
//!
//! A column is a display cell. Wide characters take two cells, zero-width
//! characters attach to the preceding character, tabs expand to the next
//! multiple of [`TAB_WIDTH`], and every invalid UTF-8 byte or control
//! character takes one cell.

use std::ops::Range;

use unicode_width::UnicodeWidthChar;

/// Width of a tab character in the file
// ORACLE: TAB-1
pub const TAB_WIDTH: usize = 8;

/// What a glyph shows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind
{
    /// A printable character, possibly followed by zero-width characters
    Char(char),
    /// A tab, expanded to the next tab stop
    Tab,
    /// A control character other than tab
    Control(char),
    /// A byte that is not valid UTF-8
    Invalid(u8)
}

/// One displayed unit of a line
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph
{
    /// Bytes of the line that make up the glyph
    pub bytes: Range<usize>,
    /// First cell of the glyph
    pub col:   usize,
    /// Number of cells
    pub width: usize,
    /// What the glyph shows
    pub kind:  GlyphKind
}

/// How the `entab` switch converts spaces to tabs on save
// ORACLE: TAB-1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entab
{
    /// `entab:0`: no tabs
    Never,
    /// `entab:1`: runs of spaces outside quoted strings
    OutsideQuotes,
    /// `entab:2`: all runs of spaces
    Everywhere
}

/// Splits `line` into glyphs
pub fn glyphs(line: &[u8]) -> Vec<Glyph>
{
    let mut glyphs: Vec<Glyph> = Vec::new();
    let mut byte = 0;
    let mut col = 0;
    for chunk in line.utf8_chunks()
    {
        for ch in chunk.valid().chars()
        {
            let len = ch.len_utf8();
            let (kind, width) = match ch
            {
                | '\t' => (GlyphKind::Tab, TAB_WIDTH - col % TAB_WIDTH),
                | ch if ch.is_control() => (GlyphKind::Control(ch), 1),
                | ch => (GlyphKind::Char(ch), ch.width().unwrap_or(1))
            };
            match (width, glyphs.last_mut())
            {
                | (0, Some(previous)) => previous.bytes.end += len,
                | (width, _) =>
                {
                    let width = width.max(1);
                    glyphs.push(Glyph { bytes: byte..byte + len, col, width, kind });
                    col += width;
                }
            }
            byte += len;
        }
        for &invalid in chunk.invalid()
        {
            glyphs.push(Glyph { bytes: byte..byte + 1, col, width: 1, kind: GlyphKind::Invalid(invalid) });
            byte += 1;
            col += 1;
        }
    }
    glyphs
}

/// Returns the width of `line` in cells
pub fn width(line: &[u8]) -> usize
{
    glyphs(line).last().map_or(0, |glyph| glyph.col + glyph.width)
}

/// Returns the first byte of the glyph covering cell `col`, or the line length beyond its end
pub fn byte_at(line: &[u8], col: usize) -> usize
{
    glyphs(line).into_iter().find(|glyph| col < glyph.col + glyph.width).map_or(line.len(), |glyph| glyph.bytes.start)
}

/// Returns the first cell of the glyph containing byte `byte`, or the line width at or beyond its end
pub fn col_at(line: &[u8], byte: usize) -> usize
{
    let glyphs = glyphs(line);
    let end = glyphs.last().map_or(0, |glyph| glyph.col + glyph.width);
    glyphs.into_iter().find(|glyph| byte < glyph.bytes.end).map_or(end, |glyph| glyph.col)
}

/// Returns the bytes of the glyphs whose first cell lies in the cells `left..right`
pub fn box_range(line: &[u8], left: usize, right: usize) -> Range<usize>
{
    let glyphs = glyphs(line);
    // A glyph that starts left of the box stays outside, so the box begins after it.
    let start = glyphs.iter().find(|glyph| glyph.col >= left).map_or(line.len(), |glyph| glyph.bytes.start);
    let end = glyphs.iter().take_while(|glyph| glyph.col < right).last().map_or(start, |glyph| glyph.bytes.end);
    start..end.max(start)
}

/// Replaces tabs with the spaces they display as
pub fn detab(line: &[u8]) -> Vec<u8>
{
    let mut out = Vec::with_capacity(line.len());
    for glyph in glyphs(line)
    {
        match glyph.kind
        {
            | GlyphKind::Tab => out.resize(out.len() + glyph.width, b' '),
            | _ => out.extend_from_slice(&line[glyph.bytes])
        }
    }
    out
}

/// Replaces runs of two or more spaces that reach a tab stop with tabs
///
/// The displayed line does not change. With [`Entab::OutsideQuotes`], spaces
/// between `"` or `'` quotes are kept.
pub fn entab(line: &[u8], mode: Entab) -> Vec<u8>
{
    require!(mode != Entab::Never, line.to_vec());
    let mut out = Vec::with_capacity(line.len());
    let mut quote = None;
    let mut spaces = 0;
    for glyph in glyphs(line)
    {
        let kind = glyph.kind;
        // A space carrying zero-width characters is content, not white space.
        if kind == GlyphKind::Char(' ') && glyph.bytes.len() == 1 && (mode == Entab::Everywhere || quote.is_none())
        {
            spaces += 1;
            if (glyph.col + 1) % TAB_WIDTH == 0
            {
                out.push(
                    if spaces > 1
                    {
                        b'\t'
                    }
                    else
                    {
                        b' '
                    }
                );
                spaces = 0;
            }
        }
        else
        {
            out.resize(out.len() + spaces, b' ');
            spaces = 0;
            quote = match (quote, kind)
            {
                | (None, GlyphKind::Char(ch @ ('"' | '\''))) => Some(ch),
                | (Some(open), GlyphKind::Char(ch)) if ch == open => None,
                | (quote, _) => quote
            };
            out.extend_from_slice(&line[glyph.bytes]);
        }
    }
    out.resize(out.len() + spaces, b' ');
    out
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn layout(line: &[u8]) -> Vec<(Range<usize>, usize, usize)>
    {
        glyphs(line).into_iter().map(|glyph| (glyph.bytes, glyph.col, glyph.width)).collect()
    }

    #[test]
    fn test_ascii_one_cell_per_byte()
    {
        assert_eq!(layout(b"ab"), vec![(0..1, 0, 1), (1..2, 1, 1)]);
        assert_eq!(glyphs(b"a")[0].kind, GlyphKind::Char('a'));
        assert_eq!(width(b"abc"), 3);
        assert_eq!(width(b""), 0);
    }

    #[test]
    fn test_wide_characters_take_two_cells()
    {
        let line = "日本".as_bytes();
        assert_eq!(layout(line), vec![(0..3, 0, 2), (3..6, 2, 2)]);
        assert_eq!(width(line), 4);
        assert_eq!(byte_at(line, 1), 0);
        assert_eq!(byte_at(line, 2), 3);
        assert_eq!(col_at(line, 3), 2);
        assert_eq!(col_at(line, 1), 0);
    }

    #[test]
    fn test_combining_marks_attach_to_previous_character()
    {
        let line = "e\u{301}x".as_bytes();
        assert_eq!(layout(line), vec![(0..3, 0, 1), (3..4, 1, 1)]);
        assert_eq!(col_at(line, 2), 0);
    }

    #[test]
    fn test_zero_width_character_at_line_start_takes_one_cell()
    {
        assert_eq!(layout("\u{301}a".as_bytes()), vec![(0..2, 0, 1), (2..3, 1, 1)]);
    }

    #[test]
    fn test_invalid_bytes_take_one_cell_each()
    {
        let line = b"a\xff\xe2\x82b";
        assert_eq!(layout(line), vec![(0..1, 0, 1), (1..2, 1, 1), (2..3, 2, 1), (3..4, 3, 1), (4..5, 4, 1)]);
        assert_eq!(glyphs(line)[1].kind, GlyphKind::Invalid(0xff));
    }

    #[test]
    fn test_control_characters_take_one_cell()
    {
        let line = b"a\x0cb\x7f";
        assert_eq!(layout(line), vec![(0..1, 0, 1), (1..2, 1, 1), (2..3, 2, 1), (3..4, 3, 1)]);
        assert_eq!(glyphs(line)[1].kind, GlyphKind::Control('\x0c'));
        assert_eq!(glyphs(line)[3].kind, GlyphKind::Control('\x7f'));
    }

    #[test]
    fn test_tabs_expand_to_next_multiple_of_eight()
    {
        let line = b"a\tb\t\tc";
        assert_eq!(layout(line), vec![(0..1, 0, 1), (1..2, 1, 7), (2..3, 8, 1), (3..4, 9, 7), (4..5, 16, 8), (5..6, 24, 1)]);
        assert_eq!(glyphs(line)[1].kind, GlyphKind::Tab);
        assert_eq!(byte_at(line, 5), 1);
        assert_eq!(col_at(line, 2), 8);
    }

    #[test]
    fn test_mapping_beyond_end_of_line()
    {
        assert_eq!(byte_at(b"ab", 2), 2);
        assert_eq!(byte_at(b"ab", 10), 2);
        assert_eq!(col_at(b"ab", 2), 2);
        assert_eq!(col_at("日".as_bytes(), 3), 2);
    }

    #[test]
    fn test_box_takes_wide_character_when_its_first_cell_is_inside()
    {
        let line = "a日b".as_bytes();
        assert_eq!(box_range(line, 0, 2), 0..4);
        assert_eq!(box_range(line, 1, 2), 1..4);
        assert_eq!(box_range(line, 2, 4), 4..5);
        assert_eq!(box_range(line, 2, 3), 4..4);
        assert_eq!(box_range(line, 5, 9), 5..5);
    }

    #[test]
    fn test_detab_replaces_tabs_with_spaces()
    {
        assert_eq!(detab(b"a\tb"), b"a       b");
        assert_eq!(detab(b"\xff\t"), b"\xff       ");
    }

    #[test]
    fn test_entab_never_keeps_spaces()
    {
        assert_eq!(entab(b"        x", Entab::Never), b"        x");
    }

    #[test]
    fn test_entab_converts_runs_reaching_a_tab_stop()
    {
        assert_eq!(entab(b"        x", Entab::OutsideQuotes), b"\tx");
        assert_eq!(entab(b"a       b", Entab::OutsideQuotes), b"a\tb");
        assert_eq!(entab(b"                 x", Entab::OutsideQuotes), b"\t\t x");
        assert_eq!(entab(b"x   ", Entab::OutsideQuotes), b"x   ");
    }

    #[test]
    fn test_entab_keeps_single_space_before_tab_stop()
    {
        assert_eq!(entab(b"abcdefg h", Entab::OutsideQuotes), b"abcdefg h");
    }

    #[test]
    fn test_entab_outside_quotes_keeps_quoted_spaces()
    {
        assert_eq!(entab(b"\"        \"        x", Entab::OutsideQuotes), b"\"        \"\t  x");
        assert_eq!(entab(b"'\"        '", Entab::OutsideQuotes), b"'\"        '");
        assert_eq!(entab(b"\"        \"", Entab::Everywhere), b"\"\t \"");
    }

    #[test]
    fn test_entab_keeps_space_with_combining_mark()
    {
        assert_eq!(entab(" \u{301}       x".as_bytes(), Entab::Everywhere), " \u{301}\tx".as_bytes());
    }

    #[test]
    fn test_entab_counts_cells_of_wide_characters()
    {
        assert_eq!(entab("日本      x".as_bytes(), Entab::OutsideQuotes), "日本\t  x".as_bytes());
    }

    fn line() -> impl Strategy<Value = Vec<u8>>
    {
        let piece = prop_oneof![
            Just(b" ".to_vec()),
            Just(b"\t".to_vec()),
            Just(b"\"".to_vec()),
            Just(b"'".to_vec()),
            Just("日".as_bytes().to_vec()),
            Just("\u{301}".as_bytes().to_vec()),
            any::<u8>().prop_map(|byte| vec![byte])
        ];
        proptest::collection::vec(piece, 0..40).prop_map(|pieces| pieces.concat())
    }

    proptest! {
        #[test]
        fn prop_glyphs_cover_the_line_without_gaps(line in line())
        {
            let mut byte = 0;
            let mut col = 0;
            for glyph in glyphs(&line)
            {
                prop_assert_eq!(glyph.bytes.start, byte);
                prop_assert_eq!(glyph.col, col);
                prop_assert!(glyph.width > 0);
                byte = glyph.bytes.end;
                col += glyph.width;
            }
            prop_assert_eq!(byte, line.len());
            prop_assert_eq!(col, width(&line));
        }

        #[test]
        fn prop_cell_and_byte_mapping_round_trip(line in line(), probe in 0usize..100)
        {
            for glyph in glyphs(&line)
            {
                prop_assert_eq!(byte_at(&line, glyph.col), glyph.bytes.start);
                prop_assert_eq!(col_at(&line, glyph.bytes.start), glyph.col);
                prop_assert_eq!(byte_at(&line, glyph.col + glyph.width - 1), glyph.bytes.start);
            }
            prop_assert!(col_at(&line, byte_at(&line, probe)) <= probe.max(width(&line)));
        }

        #[test]
        fn prop_entab_and_detab_keep_the_display(line in line())
        {
            let shown = detab(&line);
            prop_assert!(shown.contains(&b'\t') == false);
            prop_assert_eq!(width(&shown), width(&line));
            for mode in [Entab::Never, Entab::OutsideQuotes, Entab::Everywhere]
            {
                prop_assert_eq!(detab(&entab(&shown, mode)), shown.clone());
            }
        }
    }
}
