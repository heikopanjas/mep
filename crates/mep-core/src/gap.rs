//! Byte-based gap buffer
//!
//! Stores a file's bytes unchanged, including invalid UTF-8 (IMPLEMENTATION_PLAN.md §3.2, §3.4).

use std::ops::Range;

use anyhow::anyhow;

use crate::Result;

/// Smallest gap allocated when the buffer grows
const MIN_GAP: usize = 64;

/// Growable byte buffer with a movable gap at the edit position
///
/// Inserts and deletes move the gap to the edit offset first, so a run of
/// edits at one place costs no copying beyond the edited bytes.
#[derive(Debug, Default)]
pub struct GapBuffer
{
    buf:       Vec<u8>,
    gap_start: usize,
    gap_end:   usize
}

impl GapBuffer
{
    /// Inserts `bytes` before logical `offset`
    ///
    /// # Errors
    ///
    /// Returns an error and leaves the buffer unchanged if `offset` is beyond the end.
    pub fn insert(&mut self, offset: usize, bytes: &[u8]) -> Result<()>
    {
        self.check_range(&(offset..offset))?;
        self.move_gap(offset);
        self.reserve_gap(bytes.len());
        self.buf[self.gap_start..self.gap_start + bytes.len()].copy_from_slice(bytes);
        self.gap_start += bytes.len();
        Ok(())
    }

    /// Deletes the logical byte `range`
    ///
    /// # Errors
    ///
    /// Returns an error and leaves the buffer unchanged if `range` is reversed or extends beyond the end.
    pub fn delete(&mut self, range: Range<usize>) -> Result<()>
    {
        self.check_range(&range)?;
        self.move_gap(range.start);
        self.gap_end += range.len();
        Ok(())
    }

    /// Returns `true` if the buffer holds no bytes
    pub fn is_empty(&self) -> bool
    {
        self.len() == 0
    }

    /// Returns the number of bytes in the buffer, not counting the gap
    pub fn len(&self) -> usize
    {
        self.buf.len() - self.gap_len()
    }

    /// Returns the logical byte `range` as the parts before and after the gap
    ///
    /// Either part may be empty. The range content is the first part followed by the second.
    ///
    /// # Errors
    ///
    /// Returns an error if `range` is reversed or extends beyond the end.
    pub fn slices(&self, range: Range<usize>) -> Result<(&[u8], &[u8])>
    {
        self.check_range(&range)?;
        // Clamping both ends to the gap splits the range without branching.
        let gap = self.gap_len();
        let first = range.start.min(self.gap_start)..range.end.min(self.gap_start);
        let second = range.start.max(self.gap_start) + gap..range.end.max(self.gap_start) + gap;
        Ok((&self.buf[first], &self.buf[second]))
    }

    /// Fails unless `range` is ordered and lies within the logical content
    fn check_range(&self, range: &Range<usize>) -> Result<()>
    {
        require!(range.start <= range.end && range.end <= self.len(), Err(anyhow!("range {range:?} out of bounds for length {}", self.len())));
        Ok(())
    }

    fn gap_len(&self) -> usize
    {
        self.gap_end - self.gap_start
    }

    /// Moves the gap so that it starts at logical `offset`
    fn move_gap(&mut self, offset: usize)
    {
        if offset < self.gap_start
        {
            let count = self.gap_start - offset;
            self.buf.copy_within(offset..self.gap_start, self.gap_end - count);
            self.gap_start = offset;
            self.gap_end -= count;
        }
        else if offset > self.gap_start
        {
            let count = offset - self.gap_start;
            self.buf.copy_within(self.gap_end..self.gap_end + count, self.gap_start);
            self.gap_start = offset;
            self.gap_end += count;
        }
    }

    /// Grows the gap to hold at least `needed` bytes
    ///
    /// The new gap is at least as large as the content, so the total size at
    /// least doubles and repeated inserts cost amortized constant time.
    fn reserve_gap(&mut self, needed: usize)
    {
        if self.gap_len() < needed
        {
            let old_size = self.buf.len();
            let tail = old_size - self.gap_end;
            let new_size = self.len() + needed.max(self.len()).max(MIN_GAP);
            self.buf.resize(new_size, 0);
            self.buf.copy_within(self.gap_end..old_size, new_size - tail);
            self.gap_end = new_size - tail;
        }
    }
}

impl From<Vec<u8>> for GapBuffer
{
    /// Takes ownership of `bytes`, with an empty gap at the end
    fn from(bytes: Vec<u8>) -> Self
    {
        let len = bytes.len();
        Self { buf: bytes, gap_start: len, gap_end: len }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn contents(buffer: &GapBuffer) -> Vec<u8>
    {
        let (first, second) = buffer.slices(0..buffer.len()).unwrap();
        [first, second].concat()
    }

    #[test]
    fn test_default_is_empty()
    {
        let buffer = GapBuffer::default();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(contents(&buffer), b"");
    }

    #[test]
    fn test_from_vec_keeps_bytes_unchanged()
    {
        let bytes = b"a\r\nb\xff\xfe\n".to_vec();
        let buffer = GapBuffer::from(bytes.clone());
        assert!(buffer.is_empty() == false);
        assert_eq!(buffer.len(), bytes.len());
        assert_eq!(contents(&buffer), bytes);
    }

    #[test]
    fn test_insert_into_empty_buffer()
    {
        let mut buffer = GapBuffer::default();
        buffer.insert(0, b"hello").unwrap();
        assert_eq!(contents(&buffer), b"hello");
    }

    #[test]
    fn test_insert_at_start_middle_and_end()
    {
        let mut buffer = GapBuffer::from(b"bd".to_vec());
        buffer.insert(2, b"e").unwrap();
        buffer.insert(0, b"a").unwrap();
        buffer.insert(2, b"c").unwrap();
        assert_eq!(contents(&buffer), b"abcde");
    }

    #[test]
    fn test_insert_moves_gap_left_and_right()
    {
        let mut buffer = GapBuffer::from(b"0123456789".to_vec());
        buffer.insert(5, b"x").unwrap();
        buffer.insert(2, b"y").unwrap();
        buffer.insert(9, b"z").unwrap();
        buffer.insert(9, b"w").unwrap();
        assert_eq!(contents(&buffer), b"01y234x56wz789");
    }

    #[test]
    fn test_insert_grows_beyond_initial_gap()
    {
        let mut buffer = GapBuffer::default();
        let chunk = [b'x'; 100];
        for round in 0..20
        {
            buffer.insert(round * 50, &chunk).unwrap();
        }
        assert_eq!(buffer.len(), 2000);
        assert_eq!(contents(&buffer), vec![b'x'; 2000]);
    }

    #[test]
    fn test_insert_empty_slice_changes_nothing()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        buffer.insert(1, b"").unwrap();
        assert_eq!(contents(&buffer), b"abc");
    }

    #[test]
    fn test_insert_beyond_end_fails_unchanged()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        let error = buffer.insert(4, b"x").unwrap_err();
        assert_eq!(error.to_string(), "range 4..4 out of bounds for length 3");
        assert_eq!(contents(&buffer), b"abc");
    }

    #[test]
    fn test_delete_ranges()
    {
        let mut buffer = GapBuffer::from(b"0123456789".to_vec());
        buffer.delete(8..10).unwrap();
        buffer.delete(0..2).unwrap();
        buffer.delete(2..4).unwrap();
        assert_eq!(contents(&buffer), b"2367");
    }

    #[test]
    fn test_delete_empty_range_changes_nothing()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        buffer.delete(1..1).unwrap();
        assert_eq!(contents(&buffer), b"abc");
    }

    #[test]
    fn test_delete_everything()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        buffer.delete(0..3).unwrap();
        assert!(buffer.is_empty());
        assert_eq!(contents(&buffer), b"");
    }

    #[test]
    fn test_delete_reversed_range_fails_unchanged()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        #[allow(clippy::reversed_empty_ranges)]
        let error = buffer.delete(2..1).unwrap_err();
        assert_eq!(error.to_string(), "range 2..1 out of bounds for length 3");
        assert_eq!(contents(&buffer), b"abc");
    }

    #[test]
    fn test_delete_beyond_end_fails_unchanged()
    {
        let mut buffer = GapBuffer::from(b"abc".to_vec());
        let error = buffer.delete(1..4).unwrap_err();
        assert_eq!(error.to_string(), "range 1..4 out of bounds for length 3");
        assert_eq!(contents(&buffer), b"abc");
    }

    #[test]
    fn test_slices_before_across_and_after_gap()
    {
        let mut buffer = GapBuffer::from(b"abcdef".to_vec());
        buffer.insert(3, b"X").unwrap();
        // Logical content "abcXdef"; the gap sits after the inserted byte.
        assert_eq!(buffer.slices(0..2).unwrap(), (&b"ab"[..], &b""[..]));
        assert_eq!(buffer.slices(2..6).unwrap(), (&b"cX"[..], &b"de"[..]));
        assert_eq!(buffer.slices(5..7).unwrap(), (&b""[..], &b"ef"[..]));
        assert_eq!(buffer.slices(4..4).unwrap(), (&b""[..], &b""[..]));
    }

    #[test]
    fn test_slices_out_of_bounds_fails()
    {
        let buffer = GapBuffer::from(b"abc".to_vec());
        assert_eq!(buffer.slices(0..4).unwrap_err().to_string(), "range 0..4 out of bounds for length 3");
        #[allow(clippy::reversed_empty_ranges)]
        let reversed = buffer.slices(3..2);
        assert_eq!(reversed.unwrap_err().to_string(), "range 3..2 out of bounds for length 3");
    }

    #[derive(Debug, Clone)]
    enum Op
    {
        Insert(usize, Vec<u8>),
        Delete(usize, usize)
    }

    fn op() -> impl Strategy<Value = Op>
    {
        prop_oneof![
            (any::<usize>(), proptest::collection::vec(any::<u8>(), 0..40)).prop_map(|(at, bytes)| Op::Insert(at, bytes)),
            (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Op::Delete(a, b))
        ]
    }

    proptest! {
        #[test]
        fn prop_edits_match_vec_model(initial in proptest::collection::vec(any::<u8>(), 0..200), ops in proptest::collection::vec(op(), 0..100))
        {
            let mut buffer = GapBuffer::from(initial.clone());
            let mut model = initial;
            for op in ops
            {
                match op
                {
                    | Op::Insert(at, bytes) =>
                    {
                        let at = at % (model.len() + 1);
                        buffer.insert(at, &bytes).unwrap();
                        model.splice(at..at, bytes);
                    }
                    | Op::Delete(a, b) =>
                    {
                        let (a, b) = (a % (model.len() + 1), b % (model.len() + 1));
                        let range = a.min(b)..a.max(b);
                        buffer.delete(range.clone()).unwrap();
                        model.drain(range);
                    }
                }
                prop_assert_eq!(buffer.len(), model.len());
                prop_assert_eq!(contents(&buffer), model.clone());
            }
        }

        #[test]
        fn prop_slices_match_vec_model(initial in proptest::collection::vec(any::<u8>(), 0..200), at in any::<usize>(), insert in proptest::collection::vec(any::<u8>(), 0..40), a in any::<usize>(), b in any::<usize>())
        {
            let mut buffer = GapBuffer::from(initial.clone());
            let mut model = initial;
            let at = at % (model.len() + 1);
            buffer.insert(at, &insert).unwrap();
            model.splice(at..at, insert);
            let (a, b) = (a % (model.len() + 1), b % (model.len() + 1));
            let range = a.min(b)..a.max(b);
            let (first, second) = buffer.slices(range.clone()).unwrap();
            prop_assert_eq!([first, second].concat(), &model[range]);
        }

        #[test]
        fn prop_out_of_bounds_offsets_fail(initial in proptest::collection::vec(any::<u8>(), 0..50), beyond in 1usize..100)
        {
            let mut buffer = GapBuffer::from(initial.clone());
            let end = initial.len() + beyond;
            prop_assert!(buffer.insert(end, b"x").is_err());
            prop_assert!(buffer.delete(0..end).is_err());
            prop_assert!(buffer.slices(0..end).is_err());
            prop_assert_eq!(contents(&buffer), initial);
        }
    }
}
