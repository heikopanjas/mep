//! Command line (IMPLEMENTATION_PLAN.md §6.1; M guide §2.5 p. 12–13)
//!
//! Options start with `-` or `--` on every system; an argument starting with
//! `/` is always a file name.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf}
};

use clap::{CommandFactory, Parser};

/// Parsed command line
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "mep", version = mep_core::version(), about = "Microsoft Editor (MEP) for modern terminals")]
pub struct CliOptions
{
    /// Do not read TOOLS.INI
    #[arg(short = 'd', long = "no-tools-ini")]
    pub no_tools_ini: bool,

    /// Execute <command> (macro syntax) after startup; may be repeated
    #[arg(short = 'e', long = "execute", value_name = "command")]
    pub execute: Vec<String>,

    /// Files are temporary (not recorded in the information file)
    #[arg(short = 't', long = "temporary")]
    pub temporary: bool,

    /// Files to edit; `exit` walks through them in order
    #[arg(value_name = "file")]
    pub files: Vec<PathBuf>
}

/// A command line that does not start the editor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliExit
{
    /// Text to print
    pub text:   String,
    /// Process exit code: 0 for help and version, 2 for usage errors
    pub code:   i32,
    /// Print to stdout (help, version) instead of stderr
    pub stdout: bool
}

/// Parses the arguments including the program name
///
/// # Errors
///
/// Returns what to print and the exit code for help, version and usage errors.
pub fn parse<I, T>(args: I) -> Result<CliOptions, CliExit>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone
{
    CliOptions::try_parse_from(args).map_err(|error| {
        let code = error.exit_code();
        let mut text = error.render().to_string();
        // Usage errors always show the usage line (plan §6.1); clap omits it for missing values.
        if code != 0 && text.contains("Usage:") == false
        {
            text.push_str(&format!("\n{}\n", CliOptions::command().render_usage()));
        }
        CliExit { text, code, stdout: code == 0 }
    })
}

/// Returns the editor name that selects the TOOLS.INI section: the file stem of
/// the program name as typed, lowercase; `mep` if there is none
pub fn exe_name(program: Option<&OsStr>) -> String
{
    program.map(Path::new).and_then(Path::file_stem).map_or_else(|| "mep".to_string(), |stem| stem.to_string_lossy().to_lowercase())
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn ok(args: &[&str]) -> CliOptions
    {
        parse(std::iter::once("mep").chain(args.iter().copied())).unwrap()
    }

    fn err(args: &[&str]) -> CliExit
    {
        parse(std::iter::once("mep").chain(args.iter().copied())).unwrap_err()
    }

    fn files(names: &[&str]) -> Vec<PathBuf>
    {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn test_no_arguments()
    {
        assert_eq!(ok(&[]), CliOptions { no_tools_ini: false, execute: Vec::new(), temporary: false, files: Vec::new() });
    }

    #[test]
    fn test_short_and_long_flags()
    {
        for args in [["-d", "-t"], ["--no-tools-ini", "--temporary"]]
        {
            let options = ok(&args);
            assert!(options.no_tools_ini);
            assert!(options.temporary);
        }
    }

    #[test]
    fn test_bundled_short_flags()
    {
        let options = ok(&["-dt"]);
        assert!(options.no_tools_ini && options.temporary);
    }

    #[test]
    fn test_execute_forms_run_in_order()
    {
        assert_eq!(ok(&["-e", "arg up", "--execute", "down", "--execute=meta exit", "-ehome"]).execute, vec!["arg up", "down", "meta exit", "home"]);
    }

    #[test]
    fn test_files_and_end_of_options()
    {
        assert_eq!(ok(&["a.c", "/abs/b.c", "--", "-d", "--x"]).files, files(&["a.c", "/abs/b.c", "-d", "--x"]));
        assert!(ok(&["/d", "/e", "x"]).no_tools_ini == false);
        assert_eq!(ok(&["/d", "/e", "x"]).files, files(&["/d", "/e", "x"]));
    }

    #[test]
    fn test_help_and_version_go_to_stdout()
    {
        for flag in ["-h", "--help"]
        {
            let exit = err(&[flag]);
            assert_eq!((exit.code, exit.stdout), (0, true));
            assert!(exit.text.contains("--no-tools-ini"));
        }
        for flag in ["-V", "--version"]
        {
            let exit = err(&[flag]);
            assert_eq!((exit.code, exit.stdout), (0, true));
            assert_eq!(exit.text, format!("mep {}\n", mep_core::version()));
        }
    }

    #[test]
    fn test_usage_errors_go_to_stderr()
    {
        for args in [&["--bogus"][..], &["-x"], &["-e"], &["--execute"]]
        {
            let exit = err(args);
            assert_eq!((exit.code, exit.stdout), (2, false), "{args:?}");
            assert!(exit.text.contains("Usage:"), "{args:?}");
        }
    }

    #[test]
    fn test_exe_name()
    {
        assert_eq!(exe_name(Some(OsStr::new("/usr/bin/mep"))), "mep");
        assert_eq!(exe_name(Some(OsStr::new("MZ.EXE"))), "mz");
        assert_eq!(exe_name(Some(OsStr::new(""))), "mep");
        assert_eq!(exe_name(None), "mep");
    }

    proptest! {
        #[test]
        fn prop_plain_names_are_files(names in proptest::collection::vec("[a-zA-Z0-9_./][a-zA-Z0-9_./-]{0,8}", 0..5))
        {
            let options = parse(std::iter::once("mep".to_string()).chain(names.iter().cloned())).unwrap();
            prop_assert_eq!(options.files, names.iter().map(PathBuf::from).collect::<Vec<_>>());
        }
    }
}
