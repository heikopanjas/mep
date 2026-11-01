//! Editor core
//!
//! All editor logic lives here. The core never touches the terminal, the file
//! system, the environment or the system clipboard directly.

#![forbid(unsafe_code)]
#![cfg_attr(test, feature(coverage_attribute))]
#![allow(clippy::bool_comparison)]

/// Early-return guard macro for precondition checks
///
/// Returns the given expression when the condition is false.
/// Works with any return type: `Result`, `Option`, or bare values.
#[macro_export]
macro_rules! require {
    ($cond:expr, $ret:expr) => {
        if ($cond) == false
        {
            return $ret;
        }
    };
}

pub mod cells;
pub mod clip;
mod cmd;
mod commands;
mod edit;
pub mod editor;
pub mod env;
pub mod ext;
pub mod file;
pub mod frame;
mod gap;
pub mod history;
pub mod host;
pub mod ini;
pub mod io;
pub mod keys;
mod lines;
pub mod macros;
pub mod memfs;
mod render;
pub mod run;
pub mod switches;
mod text;
pub mod view;
pub mod window;

pub use anyhow::Result;
pub use cmd::is_function;
pub use editor::{Editor, Event, Outcome, Startup};
pub use gap::GapBuffer;
pub use text::{Terminator, Text};

/// Returns the MEP version shared by all workspace crates
pub fn version() -> &'static str
{
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_version_is_semver()
    {
        let parts: Vec<&str> = version().split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|part| part.is_empty() == false && part.bytes().all(|b| b.is_ascii_digit())));
    }
}
