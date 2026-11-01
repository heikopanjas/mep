//! TOOLS.INI search path and sections (IMPLEMENTATION_PLAN.md §6.12; M guide Chapter 7 p. 55–65)

use std::path::PathBuf;

use crate::env::{Env, Os};

/// One statement from a selected section
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement
{
    /// 1-based line number in TOOLS.INI, for error messages
    pub line: usize,
    /// The statement without surrounding blanks
    pub text: String
}

/// Returns the directories searched for TOOLS.INI, in order
///
/// The directories listed in `INIT`, then `%APPDATA%\mep` (Windows) or
/// `$XDG_CONFIG_HOME/mep` / `~/.config/mep` (macOS, Linux), then the home directory.
pub fn search_dirs(env: &dyn Env) -> Vec<PathBuf>
{
    let os = env.os();
    let var = |name: &str| env.var(name).map(PathBuf::from);
    // INIT, not USER: on Unix, USER is the login name (trap T1).
    let mut dirs: Vec<PathBuf> = env
        .var("INIT")
        .map(|list| list.to_string_lossy().split(os.list_separator()).filter(|dir| dir.is_empty() == false).map(PathBuf::from).collect())
        .unwrap_or_default();
    let home = match os
    {
        | Os::Windows => var("USERPROFILE"),
        | _ => var("HOME")
    };
    dirs.extend(config_dir(env));
    dirs.extend(home);
    dirs
}

/// Returns the editor's configuration directory: `%APPDATA%\mep` (Windows) or
/// `$XDG_CONFIG_HOME/mep` / `~/.config/mep` (macOS, Linux)
pub fn config_dir(env: &dyn Env) -> Option<PathBuf>
{
    let var = |name: &str| env.var(name).map(PathBuf::from);
    let config = match env.os()
    {
        | Os::Windows => var("APPDATA"),
        | _ => var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|home| home.join(".config")))
    };
    config.map(|dir| dir.join("mep"))
}

/// Returns `true` if `name` is a TOOLS.INI file name, ignoring case
pub fn is_tools_ini(name: &str) -> bool
{
    name.eq_ignore_ascii_case("tools.ini")
}

/// Returns the statements of all sections tagged `tag`, in file order
///
/// A tag line lists one or more names, e.g. `[m mep]`; names match ignoring
/// case. Blank lines and lines starting with `;` are skipped.
// ORACLE: INI-1
pub fn section(content: &str, tag: &str) -> Vec<Statement>
{
    let mut selected = false;
    let mut statements = Vec::new();
    for (index, line) in content.lines().enumerate()
    {
        let line = line.trim();
        if let Some(tags) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']'))
        {
            selected = tags.split_whitespace().any(|name| name.eq_ignore_ascii_case(tag));
        }
        else if selected == true && line.is_empty() == false && line.starts_with(';') == false
        {
            statements.push(Statement { line: index + 1, text: line.to_string() });
        }
    }
    statements
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::{collections::HashMap, ffi::OsString, path::Path};

    use super::*;
    use crate::env::LocalDateTime;

    struct FakeEnv
    {
        os:   Os,
        vars: HashMap<&'static str, &'static str>
    }

    impl Env for FakeEnv
    {
        fn var(&self, name: &str) -> Option<OsString>
        {
            self.vars.get(name).map(OsString::from)
        }

        fn now(&self) -> LocalDateTime
        {
            LocalDateTime { year: 1987, month: 6, day: 27, weekday: 6, hour: 13, minute: 45, second: 55 }
        }

        fn os(&self) -> Os
        {
            self.os
        }

        fn cwd(&self) -> PathBuf
        {
            PathBuf::from("/")
        }
    }

    fn env(os: Os, vars: &[(&'static str, &'static str)]) -> FakeEnv
    {
        FakeEnv { os, vars: vars.iter().copied().collect() }
    }

    #[test]
    fn test_search_order_on_unix()
    {
        let env = env(Os::Linux, &[("INIT", "/a::/b"), ("HOME", "/home/u"), ("USER", "/not/a/dir")]);
        assert_eq!(search_dirs(&env), vec![PathBuf::from("/a"), PathBuf::from("/b"), PathBuf::from("/home/u/.config/mep"), PathBuf::from("/home/u")]);
        assert_eq!(env.now().year, 1987);
        assert_eq!(env.cwd(), Path::new("/"));
    }

    #[test]
    fn test_xdg_config_home_wins_on_unix()
    {
        let env = env(Os::MacOs, &[("XDG_CONFIG_HOME", "/cfg"), ("HOME", "/home/u")]);
        assert_eq!(search_dirs(&env), vec![PathBuf::from("/cfg/mep"), PathBuf::from("/home/u")]);
    }

    #[test]
    fn test_search_order_on_windows()
    {
        let env = env(Os::Windows, &[("INIT", "C:\\init;D:\\x"), ("APPDATA", "C:\\Users\\u\\AppData\\Roaming"), ("USERPROFILE", "C:\\Users\\u")]);
        assert_eq!(search_dirs(&env), vec![
            PathBuf::from("C:\\init"),
            PathBuf::from("D:\\x"),
            PathBuf::from("C:\\Users\\u\\AppData\\Roaming").join("mep"),
            PathBuf::from("C:\\Users\\u")
        ]);
    }

    #[test]
    fn test_nothing_set_searches_nothing()
    {
        assert!(search_dirs(&env(Os::Linux, &[])).is_empty());
        assert!(search_dirs(&env(Os::Windows, &[])).is_empty());
    }

    #[test]
    fn test_tools_ini_name_ignores_case()
    {
        assert!(is_tools_ini("TOOLS.INI"));
        assert!(is_tools_ini("tools.ini"));
        assert!(is_tools_ini("Tools.Ini"));
        assert!(is_tools_ini("tools.ini.bak") == false);
    }

    const INI: &str = "\
top:level
[make]
CFLAGS=-O
[M MEP]
; comment
  exit:alt+x

extmake:asm masm -Mx %s;
[mep-linux]
hike:5
[Mep]
case:
[other mep-c]
x:y
";

    #[test]
    fn test_section_selects_matching_tags_in_order()
    {
        let statements = section(INI, "mep");
        let texts: Vec<&str> = statements.iter().map(|statement| statement.text.as_str()).collect();
        assert_eq!(texts, vec!["exit:alt+x", "extmake:asm masm -Mx %s;", "case:"]);
        assert_eq!(statements[0].line, 6);
    }

    #[test]
    fn test_section_with_os_and_text_tags()
    {
        assert_eq!(section(INI, "MEP-LINUX"), vec![Statement { line: 10, text: "hike:5".into() }]);
        assert_eq!(section(INI, "mep-c"), vec![Statement { line: 14, text: "x:y".into() }]);
        assert!(section(INI, "mz").is_empty());
        assert!(section("", "mep").is_empty());
    }
}
