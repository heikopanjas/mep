//! Environment and clock boundary (IMPLEMENTATION_PLAN.md §4.2)
//!
//! The core reads environment variables, the clock, the operating system and
//! the working directory only through [`Env`], so tests inject fixed values.

use std::{collections::HashMap, ffi::OsString};

use typed_path::Utf8TypedPathBuf;

/// The operating system the editor runs on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os
{
    /// Windows
    Windows,
    /// macOS
    MacOs,
    /// Linux and other Unix systems
    Linux
}

impl Os
{
    /// Returns the name used in TOOLS.INI tags such as `[mep-windows]`
    pub fn tag(self) -> &'static str
    {
        match self
        {
            | Self::Windows => "windows",
            | Self::MacOs => "macos",
            | Self::Linux => "linux"
        }
    }

    /// Returns the separator of directory lists such as `INIT`
    pub fn list_separator(self) -> char
    {
        match self
        {
            | Self::Windows => ';',
            | _ => ':'
        }
    }

    /// Returns `text` as a path with this operating system's separators and roots
    pub fn path(self, text: &str) -> Utf8TypedPathBuf
    {
        match self
        {
            | Self::Windows => Utf8TypedPathBuf::from_windows(text),
            | _ => Utf8TypedPathBuf::from_unix(text)
        }
    }
}

/// Local date and time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDateTime
{
    /// Year, e.g. 1987
    pub year:    u16,
    /// Month 1–12
    pub month:   u8,
    /// Day of the month 1–31
    pub day:     u8,
    /// Day of the week, 0 = Sunday
    pub weekday: u8,
    /// Hour 0–23
    pub hour:    u8,
    /// Minute 0–59
    pub minute:  u8,
    /// Second 0–59
    pub second:  u8
}

/// Environment variables, clock, operating system and working directory
pub trait Env
{
    /// Returns an environment variable
    fn var(&self, name: &str) -> Option<OsString>;

    /// Returns the local date and time
    fn now(&self) -> LocalDateTime;

    /// Returns the operating system
    fn os(&self) -> Os;

    /// Returns the working directory as an absolute path
    fn cwd(&self) -> Utf8TypedPathBuf;
}

/// Environment with fixed values, for tests
#[derive(Debug, Clone)]
pub struct FixedEnv
{
    /// Operating system
    pub os:   Os,
    /// Environment variables
    pub vars: HashMap<String, String>,
    /// Clock value
    pub now:  LocalDateTime,
    /// Working directory, a path of `os`
    pub cwd:  String
}

impl Default for FixedEnv
{
    /// Linux, no variables, Sat Jun-27-1987 13:45:55 (the manual's examples), working directory `/work`
    fn default() -> Self
    {
        Self {
            os:   Os::Linux,
            vars: HashMap::new(),
            now:  LocalDateTime { year: 1987, month: 6, day: 27, weekday: 6, hour: 13, minute: 45, second: 55 },
            cwd:  "/work".to_string()
        }
    }
}

impl Env for FixedEnv
{
    fn var(&self, name: &str) -> Option<OsString>
    {
        self.vars.get(name).map(OsString::from)
    }

    fn now(&self) -> LocalDateTime
    {
        self.now
    }

    fn os(&self) -> Os
    {
        self.os
    }

    fn cwd(&self) -> Utf8TypedPathBuf
    {
        self.os.path(&self.cwd)
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_os_tags_and_separators()
    {
        assert_eq!(Os::Windows.tag(), "windows");
        assert_eq!(Os::MacOs.tag(), "macos");
        assert_eq!(Os::Linux.tag(), "linux");
        assert_eq!(Os::Windows.list_separator(), ';');
        let env = FixedEnv::default();
        assert_eq!((env.os(), env.var("X"), env.now().year, env.cwd()), (Os::Linux, None, 1987, Utf8TypedPathBuf::from_unix("/work")));
        assert_eq!(Os::MacOs.list_separator(), ':');
        assert_eq!(Os::Windows.path("/a").join("b").as_str(), "/a\\b");
        assert_eq!(Os::MacOs.path("/a").join("b").as_str(), "/a/b");
    }
}
