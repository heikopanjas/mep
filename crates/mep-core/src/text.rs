//! Text of one file: bytes plus line index (IMPLEMENTATION_PLAN.md §5.1)
//!
//! Bytes are kept exactly as loaded, so unedited lines round-trip byte-exact.

use std::ops::Range;

use crate::{
    GapBuffer, Result,
    cells::{Entab, entab},
    lines::LineIndex
};

/// How a line ends in the file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator
{
    /// Last line without a line break
    None,
    /// `\n`
    Lf,
    /// `\r\n`
    CrLf
}

impl Terminator
{
    /// Returns the bytes of this terminator
    pub fn bytes(self) -> &'static [u8]
    {
        match self
        {
            | Self::None => b"",
            | Self::Lf => b"\n",
            | Self::CrLf => b"\r\n"
        }
    }
}

/// Bytes of a file with an incrementally maintained line index
#[derive(Debug)]
pub struct Text
{
    buf:   GapBuffer,
    index: LineIndex
}

impl Text
{
    /// Deletes `range` and returns the removed bytes
    ///
    /// # Errors
    ///
    /// Returns an error and leaves the text unchanged if `range` is reversed or out of bounds.
    pub fn delete(&mut self, range: Range<usize>) -> Result<Vec<u8>>
    {
        let removed = self.buf.delete(range.clone())?;
        self.index.delete(range);
        Ok(removed)
    }

    /// Inserts `bytes` at `offset`
    ///
    /// # Errors
    ///
    /// Returns an error and leaves the text unchanged if `offset` is beyond the end.
    pub fn insert(&mut self, offset: usize, bytes: &[u8]) -> Result<()>
    {
        self.buf.insert(offset, bytes)?;
        self.index.insert(offset, bytes);
        Ok(())
    }

    /// Marks all lines as unedited, after saving
    pub fn clear_edited(&mut self)
    {
        self.index.clear_edited();
    }

    /// Returns `true` if line `line` was edited since loading or saving; `line` must exist
    pub fn is_edited(&self, line: usize) -> bool
    {
        self.index.edited(line)
    }

    /// Returns the bytes to save
    ///
    /// Edited lines lose their trailing spaces unless `trailspace` is set and are
    /// re-tabbed with `mode` (plan §5.2, §5.3). Unedited lines are copied byte-exact.
    pub fn save_bytes(&self, mode: Entab, trailspace: bool) -> Vec<u8>
    {
        let mut bytes = Vec::with_capacity(self.len());
        for line in 0..self.line_count()
        {
            let content = self.line(line);
            if self.is_edited(line) == true
            {
                let kept = match trailspace
                {
                    | true => content.len(),
                    | false => content.iter().rposition(|byte| *byte != b' ').map_or(0, |last| last + 1)
                };
                bytes.extend(entab(&content[..kept], mode));
            }
            else
            {
                bytes.extend(content);
            }
            bytes.extend_from_slice(self.terminator(line).bytes());
        }
        bytes
    }

    /// Returns `true` if the text holds no bytes
    pub fn is_empty(&self) -> bool
    {
        self.buf.is_empty()
    }

    /// Returns the number of bytes
    pub fn len(&self) -> usize
    {
        self.buf.len()
    }

    /// Returns the content of line `line` without its terminator; empty beyond the last line
    pub fn line(&self, line: usize) -> Vec<u8>
    {
        match line < self.line_count()
        {
            | true => self.bytes(self.line_range(line)),
            | false => Vec::new()
        }
    }

    /// Returns the number of lines, counting an empty line after a final line break
    pub fn line_count(&self) -> usize
    {
        self.index.count()
    }

    /// Returns the byte range of line `line` without its terminator; `line` must exist
    pub fn line_range(&self, line: usize) -> Range<usize>
    {
        let range = self.index.range(line);
        range.start..range.end - self.terminator(line).bytes().len()
    }

    /// Returns the line that contains byte `offset`
    pub fn line_of(&self, offset: usize) -> usize
    {
        self.index.line_of(offset)
    }

    /// Returns the bytes in `range`
    ///
    /// # Errors
    ///
    /// Returns an error if `range` is reversed or out of bounds.
    pub fn slice(&self, range: Range<usize>) -> Result<Vec<u8>>
    {
        let (first, second) = self.buf.slices(range)?;
        Ok([first, second].concat())
    }

    /// Returns the terminator of line `line`; `line` must exist
    pub fn terminator(&self, line: usize) -> Terminator
    {
        let range = self.index.range(line);
        // At most two bytes decide the terminator; a line break only ever ends a line.
        let tail = self.bytes(range.end.saturating_sub(2).max(range.start)..range.end);
        match tail.as_slice()
        {
            | [.., b'\r', b'\n'] => Terminator::CrLf,
            | [.., b'\n'] => Terminator::Lf,
            | _ => Terminator::None
        }
    }

    /// Returns all bytes, for saving
    pub fn to_bytes(&self) -> Vec<u8>
    {
        self.bytes(0..self.len())
    }

    /// Returns the bytes in a range known to be valid
    fn bytes(&self, range: Range<usize>) -> Vec<u8>
    {
        self.slice(range).unwrap_or_default()
    }
}

impl From<Vec<u8>> for Text
{
    /// Takes the file's bytes unchanged and indexes its lines
    fn from(bytes: Vec<u8>) -> Self
    {
        let index = LineIndex::new(&bytes);
        Self { buf: GapBuffer::from(bytes), index }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    const ROUND_TRIP: &[&[u8]] =
        &[b"", b"lf\nonly\n", b"crlf\r\nonly\r\n", b"mixed\r\nends\nhere\r\n", b"no final\nnewline", b"bad \xff\xfe utf8\n\xc3\n", b"\r\n\n\r\r\n"];

    #[test]
    fn test_load_save_round_trip_is_byte_exact()
    {
        for bytes in ROUND_TRIP
        {
            assert_eq!(Text::from(bytes.to_vec()).to_bytes(), *bytes);
        }
    }

    #[test]
    fn test_editing_one_line_keeps_other_lines_byte_exact()
    {
        let mut text = Text::from(b"one\r\ntwo\nthree\r\n".to_vec());
        text.insert(5, b"2").unwrap();
        assert_eq!(text.to_bytes(), b"one\r\n2two\nthree\r\n");
    }

    #[test]
    fn test_lines_and_terminators()
    {
        let text = Text::from(b"a\r\nb\nc\r\rd".to_vec());
        // A lone carriage return does not end a line.
        assert_eq!(text.line_count(), 3);
        assert_eq!(text.line(0), b"a");
        assert_eq!(text.terminator(0), Terminator::CrLf);
        assert_eq!(text.line(1), b"b");
        assert_eq!(text.terminator(1), Terminator::Lf);
        assert_eq!(text.line(2), b"c\r\rd");
        assert_eq!(text.terminator(2), Terminator::None);
        assert_eq!(text.line_range(2), 5..9);
        assert_eq!(text.line(7), b"");
    }

    #[test]
    fn test_final_line_break_gives_empty_last_line()
    {
        let text = Text::from(b"a\n".to_vec());
        assert_eq!(text.line_count(), 2);
        assert_eq!(text.line(1), b"");
        assert_eq!(text.terminator(1), Terminator::None);
    }

    #[test]
    fn test_lone_carriage_return_line_is_lf_terminated()
    {
        let text = Text::from(b"\r\n".to_vec());
        assert_eq!(text.line(0), b"");
        assert_eq!(text.terminator(0), Terminator::CrLf);
        let text = Text::from(b"\n".to_vec());
        assert_eq!(text.terminator(0), Terminator::Lf);
    }

    #[test]
    fn test_insert_and_delete_update_lines()
    {
        let mut text = Text::from(b"abc\ndef".to_vec());
        text.insert(3, b"\r\nnew").unwrap();
        assert_eq!(text.line_count(), 3);
        assert_eq!(text.line(1), b"new");
        assert_eq!(text.delete(3..8).unwrap(), b"\r\nnew");
        assert_eq!(text.line_count(), 2);
        assert_eq!(text.line_of(5), 1);
        assert_eq!(text.slice(1..5).unwrap(), b"bc\nd");
    }

    #[test]
    fn test_invalid_ranges_fail_unchanged()
    {
        let mut text = Text::from(b"abc".to_vec());
        assert!(text.insert(4, b"x").is_err());
        assert!(text.delete(2..9).is_err());
        assert!(text.slice(0..9).is_err());
        assert_eq!(text.to_bytes(), b"abc");
    }

    #[test]
    fn test_save_bytes_keeps_unedited_lines_and_cleans_edited_ones()
    {
        let mut text = Text::from(b"keep   \r\n        tab   \nend".to_vec());
        assert_eq!(text.save_bytes(Entab::Everywhere, false), text.to_bytes());
        text.insert(17, b"x").unwrap();
        assert!(text.is_edited(1));
        assert!(text.is_edited(0) == false);
        assert_eq!(text.save_bytes(Entab::OutsideQuotes, false), b"keep   \r\n\txtab\nend");
        assert_eq!(text.save_bytes(Entab::Never, true), b"keep   \r\n        xtab   \nend");
        text.clear_edited();
        assert_eq!(text.save_bytes(Entab::OutsideQuotes, false), text.to_bytes());
    }

    #[test]
    fn test_save_bytes_trims_line_of_spaces()
    {
        let mut text = Text::from(b"a\n".to_vec());
        text.insert(2, b"   ").unwrap();
        assert_eq!(text.save_bytes(Entab::Never, false), b"a\n");
    }

    #[test]
    fn test_len_and_is_empty()
    {
        assert!(Text::from(Vec::new()).is_empty());
        let text = Text::from(b"ab".to_vec());
        assert!(text.is_empty() == false);
        assert_eq!(text.len(), 2);
    }

    #[test]
    fn test_terminator_bytes()
    {
        assert_eq!(Terminator::None.bytes(), b"");
        assert_eq!(Terminator::Lf.bytes(), b"\n");
        assert_eq!(Terminator::CrLf.bytes(), b"\r\n");
    }
}
