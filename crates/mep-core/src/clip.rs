//! Clipboard model (IMPLEMENTATION_PLAN.md §3.3, §6.9)
//!
//! The clipboard holds lines in one of three shapes. Copying writes plain text
//! for other applications plus a private record that carries the shape; the
//! record only counts when its hash matches the text found on paste.

/// How clipboard text was selected and how it is pasted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape
{
    /// Whole lines, pasted above the current line
    Line,
    /// A rectangle `width` cells wide, pasted as a box at the cursor
    Box
    {
        /// Width of the box in cells
        width: usize
    },
    /// Characters across line breaks, pasted at the cursor
    Stream
}

/// Clipboard contents: the lines of the selection and its shape
///
/// For a stream, the lines are the pieces between line breaks; for a box, the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip
{
    /// Shape of the selection
    pub shape: Shape,
    /// The selected lines without terminators
    pub lines: Vec<Vec<u8>>
}

/// Private record stored next to the plain text on the system clipboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record
{
    /// Shape of the copied selection
    pub shape: Shape,
    /// FNV-1a 64 hash of the plain text exactly as written
    pub hash:  u64
}

/// What the system clipboard holds: plain text and optionally MEP's private record
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardPayload
{
    /// Plain UTF-8 text for other applications
    pub text:   String,
    /// Encoded [`Record`], if MEP wrote the text
    pub record: Option<Vec<u8>>
}

/// The system clipboard (plan §4.2); called by a worker, never by the core thread
pub trait SystemClipboard
{
    /// Reads the plain text and MEP's private record; `None` if the clipboard is unavailable
    fn read(&mut self) -> Option<ClipboardPayload>;

    /// Writes the plain text and the private record in one call; `false` if it failed
    fn write(&mut self, payload: &ClipboardPayload) -> bool;
}

/// In-memory system clipboard for tests and as fallback when no system clipboard exists
#[derive(Debug, Clone, Default)]
pub struct MemClipboard
{
    /// Current contents
    pub content:     Option<ClipboardPayload>,
    /// When set, reads return `None` and writes fail
    pub unavailable: bool
}

impl SystemClipboard for MemClipboard
{
    fn read(&mut self) -> Option<ClipboardPayload>
    {
        match self.unavailable
        {
            | true => None,
            | false => self.content.clone()
        }
    }

    fn write(&mut self, payload: &ClipboardPayload) -> bool
    {
        if self.unavailable == false
        {
            self.content = Some(payload.clone());
        }
        self.unavailable == false
    }
}

/// Version of the private record format
const RECORD_VERSION: u8 = 1;

/// Returns the FNV-1a 64 hash of `bytes`
pub fn fnv1a64(bytes: &[u8]) -> u64
{
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// Reads a little-endian number
fn little_endian(bytes: &[u8]) -> u64
{
    bytes.iter().rev().fold(0, |value, byte| value << 8 | u64::from(*byte))
}

impl Record
{
    /// Encodes the record: version, shape tag, box width (u64 LE), hash (u64 LE)
    pub fn encode(&self) -> Vec<u8>
    {
        let (tag, width) = match self.shape
        {
            | Shape::Line => (0, 0),
            | Shape::Stream => (1, 0),
            | Shape::Box { width } => (2, width as u64)
        };
        let mut bytes = vec![RECORD_VERSION, tag];
        bytes.extend(width.to_le_bytes());
        bytes.extend(self.hash.to_le_bytes());
        bytes
    }

    /// Decodes a record written by [`Record::encode`]; `None` for anything else
    pub fn decode(bytes: &[u8]) -> Option<Self>
    {
        require!(bytes.len() == 18 && bytes[0] == RECORD_VERSION, None);
        let shape = match bytes[1]
        {
            | 0 => Shape::Line,
            | 1 => Shape::Stream,
            | 2 => Shape::Box { width: little_endian(&bytes[2..10]) as usize },
            | _ => return None
        };
        Some(Self { shape, hash: little_endian(&bytes[10..18]) })
    }
}

impl Clip
{
    /// Builds the system clipboard payload, joining rows with `newline`
    ///
    /// Box rows lose their trailing padding; line-shaped text ends with a newline.
    pub fn to_payload(&self, newline: &str) -> ClipboardPayload
    {
        let rows: Vec<String> = self
            .lines
            .iter()
            .map(|line| String::from_utf8_lossy(line).into_owned())
            .map(|row| match self.shape
            {
                | Shape::Box { .. } => row.trim_end_matches(' ').to_string(),
                | _ => row
            })
            .collect();
        let mut text = rows.join(newline);
        if self.shape == Shape::Line
        {
            text.push_str(newline);
        }
        let record = Record { shape: self.shape, hash: fnv1a64(text.as_bytes()) };
        ClipboardPayload { text, record: Some(record.encode()) }
    }

    /// Reads a system clipboard payload
    ///
    /// A record whose hash matches the text gives the shape. Other text is a
    /// line paste if it ends with a newline and a stream paste otherwise.
    pub fn from_payload(payload: &ClipboardPayload) -> Self
    {
        let text = payload.text.as_str();
        let record = payload.record.as_deref().and_then(Record::decode).filter(|record| record.hash == fnv1a64(text.as_bytes()));
        let shape = record.map_or_else(
            || {
                if text.ends_with('\n')
                {
                    Shape::Line
                }
                else
                {
                    Shape::Stream
                }
            },
            |record| record.shape
        );
        // The newline after the last line of a line paste starts no further line.
        let body = if shape == Shape::Line
        {
            text.strip_suffix('\n').unwrap_or(text)
        }
        else
        {
            text
        };
        let lines = body.split('\n').map(|piece| piece.strip_suffix('\r').unwrap_or(piece).as_bytes().to_vec()).collect();
        Self { shape, lines }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn clip(shape: Shape, lines: &[&str]) -> Clip
    {
        Clip { shape, lines: lines.iter().map(|line| line.as_bytes().to_vec()).collect() }
    }

    #[test]
    fn test_fnv1a64_reference_values()
    {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn test_mem_clipboard()
    {
        let payload = ClipboardPayload { text: "x".into(), record: None };
        let mut clipboard = MemClipboard::default();
        assert_eq!(clipboard.read(), None);
        assert!(clipboard.write(&payload));
        assert_eq!(clipboard.read(), Some(payload.clone()));
        clipboard.unavailable = true;
        assert_eq!(clipboard.read(), None);
        assert!(clipboard.write(&payload) == false);
    }

    #[test]
    fn test_record_round_trip()
    {
        for shape in [Shape::Line, Shape::Stream, Shape::Box { width: 12 }]
        {
            let record = Record { shape, hash: 0x0123_4567_89ab_cdef };
            assert_eq!(Record::decode(&record.encode()), Some(record));
        }
    }

    #[test]
    fn test_record_decode_rejects_other_bytes()
    {
        let valid = Record { shape: Shape::Line, hash: 1 }.encode();
        assert_eq!(Record::decode(&valid[..valid.len() - 1]), None);
        let mut version = valid.clone();
        version[0] = 9;
        assert_eq!(Record::decode(&version), None);
        let mut shape = valid;
        shape[1] = 7;
        assert_eq!(Record::decode(&shape), None);
        assert_eq!(Record::decode(b""), None);
    }

    #[test]
    fn test_line_payload_ends_with_newline()
    {
        let payload = clip(Shape::Line, &["beta", "gamma"]).to_payload("\r\n");
        assert_eq!(payload.text, "beta\r\ngamma\r\n");
    }

    #[test]
    fn test_box_payload_trims_padding()
    {
        let payload = clip(Shape::Box { width: 4 }, &["ab  ", "    ", "cd"]).to_payload("\n");
        assert_eq!(payload.text, "ab\n\ncd");
    }

    #[test]
    fn test_stream_payload_joins_pieces()
    {
        let payload = clip(Shape::Stream, &["tail", "whole", "head"]).to_payload("\n");
        assert_eq!(payload.text, "tail\nwhole\nhead");
    }

    #[test]
    fn test_payload_round_trip_keeps_shape()
    {
        for original in [clip(Shape::Line, &["a", "b"]), clip(Shape::Box { width: 3 }, &["ab", "c"]), clip(Shape::Stream, &["x", "y"])]
        {
            assert_eq!(Clip::from_payload(&original.to_payload("\n")), original);
        }
    }

    #[test]
    fn test_payload_records_hash_of_text()
    {
        let payload = clip(Shape::Stream, &["x"]).to_payload("\n");
        let record = Record::decode(payload.record.as_deref().unwrap()).unwrap();
        assert_eq!(record.hash, fnv1a64(b"x"));
    }

    #[test]
    fn test_invalid_utf8_becomes_replacement_character()
    {
        let payload = Clip { shape: Shape::Stream, lines: vec![b"a\xffb".to_vec()] }.to_payload("\n");
        assert_eq!(payload.text, "a\u{fffd}b");
    }

    #[test]
    fn test_external_text_shape_follows_final_newline()
    {
        let external = |text: &str| Clip::from_payload(&ClipboardPayload { text: text.into(), record: None });
        assert_eq!(external("one\r\ntwo\n"), clip(Shape::Line, &["one", "two"]));
        assert_eq!(external("one\ntwo"), clip(Shape::Stream, &["one", "two"]));
        assert_eq!(external(""), clip(Shape::Stream, &[""]));
    }

    #[test]
    fn test_stale_record_is_ignored()
    {
        let mut payload = clip(Shape::Box { width: 2 }, &["ab", "cd"]).to_payload("\n");
        payload.text = "other\n".into();
        assert_eq!(Clip::from_payload(&payload), clip(Shape::Line, &["other"]));
        payload.record = Some(b"garbage".to_vec());
        assert_eq!(Clip::from_payload(&payload), clip(Shape::Line, &["other"]));
    }

    proptest! {
        #[test]
        fn prop_payload_round_trip(lines in proptest::collection::vec("[a-z ]{0,6}", 1..5), shape in prop_oneof![Just(Shape::Line), Just(Shape::Stream)])
        {
            let original = Clip { shape, lines: lines.iter().map(|line| line.as_bytes().to_vec()).collect() };
            prop_assert_eq!(Clip::from_payload(&original.to_payload("\r\n")), original);
        }
    }
}
