//! Editor core
//!
//! All editor logic lives here. The core never touches the terminal, the file
//! system, the environment or the system clipboard directly.

#![forbid(unsafe_code)]
#![allow(clippy::bool_comparison)]

/// Returns the MEP version shared by all workspace crates
pub fn version() -> &'static str
{
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
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
