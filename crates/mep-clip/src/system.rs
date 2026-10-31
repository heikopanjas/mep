//! The operating system clipboard through `clipboard-rs` (excluded from coverage, plan §9.1)
//!
//! Only forwards calls; all decisions are in `lib.rs`.

use clipboard_rs::{Clipboard, ClipboardContext, common::ClipboardContent};

use crate::Backend;

/// The system clipboard
pub struct ClipboardRs(ClipboardContext);

impl ClipboardRs
{
    /// Connects to the system clipboard; `None` without one (no display, error)
    pub fn new() -> Option<Self>
    {
        ClipboardContext::new().ok().map(Self)
    }
}

impl Backend for ClipboardRs
{
    fn text(&self) -> Option<String>
    {
        self.0.get_text().ok()
    }

    fn buffer(&self, format: &str) -> Option<Vec<u8>>
    {
        self.0.get_buffer(format).ok()
    }

    fn set(&self, contents: Vec<ClipboardContent>) -> bool
    {
        self.0.set(contents).is_ok()
    }
}
