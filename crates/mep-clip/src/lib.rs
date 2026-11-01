//! System clipboard adapter (IMPLEMENTATION_PLAN.md §3.3, §7)
//!
//! Copying writes the plain text and MEP's private shape record in one call;
//! reading returns both. The calls into `clipboard-rs` live in `system.rs`;
//! remote sessions use the terminal's OSC 52 sequence (`osc52.rs`).

#![cfg_attr(test, feature(coverage_attribute))]
#![allow(clippy::bool_comparison)]

pub mod osc52;
pub mod system;

use clipboard_rs::common::ClipboardContent;
use mep_core::clip::{ClipboardPayload, SystemClipboard};

/// Name of the private clipboard format that carries the shape record
#[cfg(target_os = "macos")]
pub const FORMAT: &str = "com.panjas.mep.shape";
/// Name of the private clipboard format that carries the shape record
#[cfg(windows)]
pub const FORMAT: &str = "MEP Clipboard Shape";
/// Name of the private clipboard format that carries the shape record
#[cfg(not(any(windows, target_os = "macos")))]
pub const FORMAT: &str = "application/x-mep-shape";

/// The clipboard operations MEP needs
pub trait Backend
{
    /// Reads the plain text; `None` if there is none or the clipboard is unavailable
    fn text(&self) -> Option<String>;

    /// Reads the bytes of a format; `None` if it is not present
    fn buffer(&self, format: &str) -> Option<Vec<u8>>;

    /// Replaces the clipboard contents with all of `contents` at once; `false` on failure
    fn set(&self, contents: Vec<ClipboardContent>) -> bool;
}

/// Returns the clipboard contents for a payload: the plain text and, if present, the record
pub fn encode(payload: &ClipboardPayload) -> Vec<ClipboardContent>
{
    let mut contents = vec![ClipboardContent::Text(payload.text.clone())];
    contents.extend(payload.record.iter().map(|record| ClipboardContent::Other(FORMAT.to_string(), record.clone())));
    contents
}

/// [`SystemClipboard`] on top of a [`Backend`]
#[derive(Debug)]
pub struct Adapter<B>
{
    backend: B
}

impl<B: Backend> Adapter<B>
{
    /// Wraps a backend
    pub fn new(backend: B) -> Self
    {
        Self { backend }
    }
}

impl<B: Backend> SystemClipboard for Adapter<B>
{
    fn read(&mut self) -> Option<ClipboardPayload>
    {
        let text = self.backend.text()?;
        Some(ClipboardPayload { text, record: self.backend.buffer(FORMAT) })
    }

    fn write(&mut self, payload: &ClipboardPayload) -> bool
    {
        self.backend.set(encode(payload))
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct Fake
    {
        contents: RefCell<Vec<ClipboardContent>>,
        broken:   bool
    }

    impl Backend for Fake
    {
        fn text(&self) -> Option<String>
        {
            self.contents.borrow().iter().find_map(|content| match content
            {
                | ClipboardContent::Text(text) => Some(text.clone()),
                | _ => None
            })
        }

        fn buffer(&self, format: &str) -> Option<Vec<u8>>
        {
            self.contents.borrow().iter().find_map(|content| match content
            {
                | ClipboardContent::Other(name, bytes) if name == format => Some(bytes.clone()),
                | _ => None
            })
        }

        fn set(&self, contents: Vec<ClipboardContent>) -> bool
        {
            *self.contents.borrow_mut() = contents;
            self.broken == false
        }
    }

    #[test]
    fn test_write_then_read_returns_text_and_record()
    {
        let payload = ClipboardPayload { text: "ab\ncd".into(), record: Some(vec![1, 2, 3]) };
        let mut adapter = Adapter::new(Fake::default());
        assert!(adapter.write(&payload));
        assert_eq!(adapter.read(), Some(payload));
    }

    #[test]
    fn test_text_without_record()
    {
        let payload = ClipboardPayload { text: "plain".into(), record: None };
        assert_eq!(encode(&payload).len(), 1);
        let mut adapter = Adapter::new(Fake::default());
        adapter.write(&payload);
        assert_eq!(adapter.read(), Some(payload));
    }

    #[test]
    fn test_empty_or_failing_clipboard()
    {
        let mut adapter = Adapter::new(Fake { broken: true, ..Fake::default() });
        assert_eq!(adapter.read(), None);
        assert!(adapter.write(&ClipboardPayload { text: String::new(), record: None }) == false);
    }

    #[test]
    fn test_fake_ignores_other_contents()
    {
        let fake = Fake::default();
        fake.set(vec![ClipboardContent::Html("<b>".into()), ClipboardContent::Other("x".into(), vec![9])]);
        assert_eq!(fake.text(), None);
        assert_eq!(fake.buffer(FORMAT), None);
    }
}
