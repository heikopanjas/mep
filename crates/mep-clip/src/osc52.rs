//! Clipboard for remote sessions through the terminal (IMPLEMENTATION_PLAN.md §3.3)
//!
//! Copying sends the plain text to the terminal as an OSC 52 sequence, so it
//! reaches the clipboard of the machine the terminal runs on. Terminals do not
//! answer reads reliably, so a read returns what MEP wrote last: paste uses the
//! internal clipboard.

use std::io::Write;

use mep_core::clip::{ClipboardPayload, SystemClipboard};

/// The OSC 52 clipboard
pub struct Osc52
{
    out:  Box<dyn Write + Send>,
    last: ClipboardPayload
}

impl Osc52
{
    /// Creates a clipboard that writes its sequences to `out`
    pub fn new(out: Box<dyn Write + Send>) -> Self
    {
        Self { out, last: ClipboardPayload { text: String::new(), record: None } }
    }
}

impl SystemClipboard for Osc52
{
    fn read(&mut self) -> Option<ClipboardPayload>
    {
        Some(self.last.clone())
    }

    fn write(&mut self, payload: &ClipboardPayload) -> bool
    {
        let written = self.out.write_all(sequence(&payload.text).as_bytes()).and_then(|()| self.out.flush()).is_ok();
        if written == true
        {
            self.last = payload.clone();
        }
        written
    }
}

/// Returns the OSC 52 sequence that puts `text` on the clipboard
pub fn sequence(text: &str) -> String
{
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// Encodes bytes as standard base64 with padding
pub fn base64(bytes: &[u8]) -> String
{
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3)
    {
        let value = chunk.iter().enumerate().fold(0u32, |value, (index, byte)| value | u32::from(*byte) << (16 - 8 * index));
        for index in 0..4
        {
            let digit = match index <= chunk.len()
            {
                | true => char::from(ALPHABET[(value >> (18 - 6 * index) & 63) as usize]),
                | false => '='
            };
            out.push(digit);
        }
    }
    out
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::{
        io,
        sync::{Arc, Mutex}
    };

    use super::*;

    #[derive(Clone, Default)]
    struct Out(Arc<Mutex<Vec<u8>>>, bool);

    impl Write for Out
    {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>
        {
            match self.1
            {
                | true => Err(io::Error::other("broken")),
                | false =>
                {
                    self.0.lock().unwrap().extend_from_slice(buf);
                    Ok(buf.len())
                }
            }
        }

        fn flush(&mut self) -> io::Result<()>
        {
            Ok(())
        }
    }

    #[test]
    fn test_base64_matches_rfc_4648_vectors()
    {
        for (plain, encoded) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")]
        {
            assert_eq!(base64(plain.as_bytes()), encoded);
        }
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn test_write_sends_the_sequence_and_read_returns_it()
    {
        let out = Out::default();
        let mut clipboard = Osc52::new(Box::new(out.clone()));
        assert_eq!(clipboard.read(), Some(ClipboardPayload { text: String::new(), record: None }));
        let payload = ClipboardPayload { text: "ab".into(), record: Some(vec![1]) };
        assert!(clipboard.write(&payload));
        assert_eq!(out.0.lock().unwrap().as_slice(), b"\x1b]52;c;YWI=\x07");
        assert_eq!(clipboard.read(), Some(payload));
    }

    #[test]
    fn test_failed_write_keeps_the_previous_contents()
    {
        let mut clipboard = Osc52::new(Box::new(Out(Arc::default(), true)));
        assert!(clipboard.write(&ClipboardPayload { text: "x".into(), record: None }) == false);
        assert_eq!(clipboard.read().unwrap().text, "");
    }
}
