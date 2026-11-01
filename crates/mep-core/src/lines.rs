//! Incremental line index (IMPLEMENTATION_PLAN.md §3.5, §5.1)
//!
//! A line starts at offset 0 and after every `\n`. Updates cost time proportional
//! to the distance from the previous edit and the number of line breaks edited,
//! never to the size of the file.

use std::ops::Range;

/// Start offsets and edited flags of all lines, split into two stacks at the last edit position
///
/// `head` holds absolute starts in ascending order. `tail` holds the remaining
/// starts as distances from the end of the text, nearest line last, so inserts
/// and deletes before them leave them valid without shifting. Each start
/// carries whether the line was edited since loading.
#[derive(Debug)]
pub struct LineIndex
{
    head: Vec<(usize, bool)>,
    tail: Vec<(usize, bool)>,
    len:  usize
}

impl LineIndex
{
    /// Builds the index for `bytes`
    pub fn new(bytes: &[u8]) -> Self
    {
        let mut head = vec![(0, false)];
        head.extend(line_breaks(bytes, 0).map(|start| (start, false)));
        Self { head, tail: Vec::new(), len: bytes.len() }
    }

    /// Returns the number of lines, counting an empty line after a final line break
    pub fn count(&self) -> usize
    {
        self.head.len() + self.tail.len()
    }

    /// Returns the start offset of line `line`; `line` must be below [`LineIndex::count`]
    pub fn start(&self, line: usize) -> usize
    {
        match line.checked_sub(self.head.len())
        {
            | None => self.head[line].0,
            | Some(behind) => self.len - self.tail[self.tail.len() - 1 - behind].0
        }
    }

    /// Returns the byte range of line `line` including its terminator
    pub fn range(&self, line: usize) -> Range<usize>
    {
        let end = match line + 1 < self.count()
        {
            | true => self.start(line + 1),
            | false => self.len
        };
        self.start(line)..end
    }

    /// Returns `true` if line `line` was edited since loading
    pub fn edited(&self, line: usize) -> bool
    {
        match line.checked_sub(self.head.len())
        {
            | None => self.head[line].1,
            | Some(behind) => self.tail[self.tail.len() - 1 - behind].1
        }
    }

    /// Returns the line that contains byte `offset` (the last line for the end offset)
    pub fn line_of(&self, offset: usize) -> usize
    {
        self.lines_starting_at_or_before(offset) - 1
    }

    /// Records that `bytes` were inserted at `offset`
    pub fn insert(&mut self, offset: usize, bytes: &[u8])
    {
        // Lines starting after `offset` move to the tail, where distances from the end stay valid.
        self.split_at(self.lines_starting_at_or_before(offset));
        self.len += bytes.len();
        self.mark_last_head_line();
        self.head.extend(line_breaks(bytes, offset).map(|start| (start, true)));
    }

    /// Records that `range` was deleted
    pub fn delete(&mut self, range: Range<usize>)
    {
        self.split_at(self.lines_starting_at_or_before(range.start));
        // Lines starting inside the deleted range lost their line break.
        while let Some(&(distance, _)) = self.tail.last() &&
            self.len - distance <= range.end
        {
            self.tail.pop();
        }
        self.len -= range.len();
        self.mark_last_head_line();
    }

    /// Marks the line holding the edit position as edited; it is the last line in `head` after a split
    fn mark_last_head_line(&mut self)
    {
        let last = self.head.len() - 1;
        self.head[last].1 = true;
    }

    /// Returns the number of lines whose start is at or before `offset`
    fn lines_starting_at_or_before(&self, offset: usize) -> usize
    {
        let (mut low, mut high) = (0, self.count());
        while low < high
        {
            let mid = low + (high - low) / 2;
            if self.start(mid) <= offset
            {
                low = mid + 1;
            }
            else
            {
                high = mid;
            }
        }
        low
    }

    /// Moves the split between head and tail so that `head` holds `lines` lines
    fn split_at(&mut self, lines: usize)
    {
        let len = self.len;
        if lines < self.head.len()
        {
            self.tail.extend(self.head.drain(lines..).rev().map(|(start, edited)| (len - start, edited)));
        }
        else
        {
            let keep = self.tail.len() - (lines - self.head.len());
            self.head.extend(self.tail.drain(keep..).rev().map(|(distance, edited)| (len - distance, edited)));
        }
    }
}

/// Yields the start of the line after every `\n` in `bytes`, offset by `base`
fn line_breaks(bytes: &[u8], base: usize) -> impl Iterator<Item = usize> + '_
{
    bytes.iter().enumerate().filter(|(_, byte)| **byte == b'\n').map(move |(at, _)| base + at + 1)
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn starts(index: &LineIndex) -> Vec<usize>
    {
        (0..index.count()).map(|line| index.start(line)).collect()
    }

    #[test]
    fn test_empty_text_has_one_empty_line()
    {
        let index = LineIndex::new(b"");
        assert_eq!(index.count(), 1);
        assert_eq!(index.range(0), 0..0);
        assert_eq!(index.line_of(0), 0);
    }

    #[test]
    fn test_line_starts_follow_line_breaks()
    {
        let index = LineIndex::new(b"ab\ncd\r\n\nlast");
        assert_eq!(starts(&index), vec![0, 3, 7, 8]);
        assert_eq!(index.range(1), 3..7);
        assert_eq!(index.range(3), 8..12);
    }

    #[test]
    fn test_final_line_break_adds_empty_line()
    {
        let index = LineIndex::new(b"a\n");
        assert_eq!(starts(&index), vec![0, 2]);
        assert_eq!(index.range(1), 2..2);
    }

    #[test]
    fn test_line_of_offset()
    {
        let index = LineIndex::new(b"ab\ncd\nef");
        let lines: Vec<usize> = (0..=8).map(|offset| index.line_of(offset)).collect();
        assert_eq!(lines, vec![0, 0, 0, 1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn test_insert_and_delete_line_breaks()
    {
        let mut index = LineIndex::new(b"abc\ndef");
        index.insert(1, b"\nX\n");
        assert_eq!(starts(&index), vec![0, 2, 4, 7]);
        index.delete(1..4);
        assert_eq!(starts(&index), vec![0, 4]);
        index.insert(7, b"\n");
        assert_eq!(starts(&index), vec![0, 4, 8]);
        index.insert(0, b"\n");
        assert_eq!(starts(&index), vec![0, 1, 5, 9]);
    }

    fn edited(index: &LineIndex) -> Vec<bool>
    {
        (0..index.count()).map(|line| index.edited(line)).collect()
    }

    #[test]
    fn test_edits_mark_lines_as_edited()
    {
        let mut index = LineIndex::new(b"a\nb\nc\nd");
        assert_eq!(edited(&index), vec![false; 4]);
        index.insert(2, b"x\ny");
        assert_eq!(edited(&index), vec![false, true, true, false, false]);
        index.delete(8..10);
        assert_eq!(edited(&index), vec![false, true, true, true]);
        index.insert(0, b"z");
        assert_eq!(edited(&index), vec![true, true, true, true]);
    }

    #[test]
    fn test_insert_at_line_start_keeps_that_line()
    {
        let mut index = LineIndex::new(b"a\nb");
        index.insert(2, b"x");
        assert_eq!(starts(&index), vec![0, 2]);
    }

    #[test]
    fn test_delete_without_line_breaks_shifts_later_lines()
    {
        let mut index = LineIndex::new(b"abc\ndef\nghi");
        index.delete(5..6);
        assert_eq!(starts(&index), vec![0, 4, 7]);
        index.delete(0..2);
        assert_eq!(starts(&index), vec![0, 2, 5]);
    }

    #[derive(Debug, Clone)]
    enum Op
    {
        Insert(usize, Vec<u8>),
        Delete(usize, usize)
    }

    fn op() -> impl Strategy<Value = Op>
    {
        let byte = prop_oneof![Just(b'\n'), Just(b'\r'), Just(b'a'), any::<u8>()];
        prop_oneof![
            (any::<usize>(), proptest::collection::vec(byte, 0..12)).prop_map(|(at, bytes)| Op::Insert(at, bytes)),
            (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Op::Delete(a, b))
        ]
    }

    proptest! {
        #[test]
        fn prop_incremental_index_equals_rebuild(initial in proptest::collection::vec(prop_oneof![Just(b'\n'), Just(b'x')], 0..60), ops in proptest::collection::vec(op(), 0..60), probe in any::<usize>())
        {
            let mut index = LineIndex::new(&initial);
            let mut model = initial;
            for op in ops
            {
                match op
                {
                    | Op::Insert(at, bytes) =>
                    {
                        let at = at % (model.len() + 1);
                        index.insert(at, &bytes);
                        model.splice(at..at, bytes);
                    }
                    | Op::Delete(a, b) =>
                    {
                        let (a, b) = (a % (model.len() + 1), b % (model.len() + 1));
                        let range = a.min(b)..a.max(b);
                        index.delete(range.clone());
                        model.drain(range);
                    }
                }
                let rebuilt = LineIndex::new(&model);
                prop_assert_eq!(starts(&index), starts(&rebuilt));
                let offset = probe % (model.len() + 1);
                prop_assert_eq!(index.line_of(offset), rebuilt.line_of(offset));
            }
        }
    }
}
