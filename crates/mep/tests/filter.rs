//! Acceptance test of the extension interface (IMPLEMENTATION_PLAN.md §8 M4)
//!
//! Compiles the MSJ `filter` extension (`crates/mep-ext/c/filter.c`) with the
//! platform C compiler (`$CC`, else `cc`, `clang` on Windows), loads it with
//! `load:` and sorts a range of lines through `sort`.

#![feature(coverage_attribute)]
#![coverage(off)]
#![allow(clippy::bool_comparison)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex, mpsc}
};

use mep::{
    host::{Inbox, TerminalHost},
    os::OsFs,
    process::OsRunner
};
use mep_core::{
    Editor, Event, Outcome, Startup,
    clip::MemClipboard,
    env::{Env, FixedEnv},
    file::FileName,
    io::serve
};
use mep_ext::Extensions;
use mep_term::{InputGate, SharedOut};

/// Compiles C sources into the shared library `dir/name.<ext>`
fn build(dir: &Path, name: &str, sources: &[PathBuf])
{
    let headers = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mep-ext/c");
    let library = dir.join(name).with_extension(std::env::consts::DLL_EXTENSION);
    let fallback = if cfg!(windows)
    {
        "clang"
    }
    else
    {
        "cc"
    };
    let compiler = std::env::var("CC").unwrap_or_else(|_| fallback.to_string());
    let mut command = Command::new(compiler);
    command.arg("-shared").arg("-I").arg(&headers).arg("-o").arg(&library).args(sources);
    if cfg!(unix)
    {
        command.arg("-fPIC");
    }
    let status = command.status().expect("a C compiler (set CC)");
    assert!(status.success(), "compiling {name} failed");
}

/// Runs the editor on `a.txt` in `dir` with startup commands and serves its requests as the worker would
fn edit(dir: &Path, execute: &[&str]) -> Editor
{
    let env = FixedEnv { cwd: dir.to_path_buf(), ..FixedEnv::default() };
    let (_events, incoming) = mpsc::channel();
    let quiet: Box<dyn Fn() -> Box<dyn std::any::Any>> = Box::new(|| Box::new(()));
    let host = TerminalHost {
        suspend: Rc::new(|| Ok(())),
        resume: Rc::new(|| Ok(())),
        wait_enter: Box::new(|| Ok(())),
        quiet,
        gate: Arc::new(InputGate::default()),
        out: SharedOut::new(Box::new(std::io::sink())),
        os: env.os(),
        interactive: String::new(),
        cwd: dir.to_path_buf(),
        fs: Arc::new(OsFs),
        clipboard: Arc::new(Mutex::new(Box::new(MemClipboard::default()))),
        runner: Arc::new(OsRunner(env.os())),
        inbox: Rc::new(Inbox::new(incoming)),
        extensions: Extensions::default()
    };
    let execute = execute.iter().map(|text| text.to_string()).collect();
    let startup = Startup { cols: 40, rows: 10, exe: "mep".into(), files: vec!["a.txt".into()], execute, tools_ini: false, temporary: true };
    let mut editor = Editor::new(Box::new(env), startup);
    editor.set_host(Rc::new(host));
    let mut outcome: Outcome = editor.start();
    let mut clipboard = MemClipboard::default();
    while outcome.requests.is_empty() == false
    {
        let request = outcome.requests.remove(0);
        let next = editor.handle(Event::Completed(serve(request, &OsFs, &mut clipboard)));
        outcome.requests.extend(next.requests);
    }
    editor
}

#[test]
fn test_filter_sorts_lines_through_sort()
{
    let dir = tempfile::TempDir::new().unwrap();
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mep-ext/c");
    build(dir.path(), "filter", &[sources.join("filter.c"), sources.join("exthdr.c")]);
    fs::write(dir.path().join("a.txt"), "head\npear\napple\nfig\ntail\n").unwrap();
    // `load:filter` finds filter.so, filter.dylib or filter.dll (ORACLE EXT-1).
    let editor = edit(dir.path(), &[r#"arg "load:filter" assign"#, r#"arg "filtcmd:sort <" assign"#, "down arg down down filter"]);
    assert!(editor.result(), "{:?}", editor.message());
    let text = editor.buffer(&FileName::Disk(dir.path().join("a.txt"))).unwrap();
    assert_eq!(String::from_utf8(text).unwrap(), "head\napple\nfig\npear\ntail\n");
    assert!(dir.path().join("filter1.tmp").exists() == false);
    assert!(dir.path().join("filter2.tmp").exists() == false);
}

#[test]
fn test_libraries_that_are_missing_or_no_extensions_fail()
{
    let dir = tempfile::TempDir::new().unwrap();
    let source = dir.path().join("empty.c");
    fs::write(&source, "int nothing(void) { return 0; }\n").unwrap();
    build(dir.path(), "empty", &[source]);
    fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    let editor = edit(dir.path(), &[r#"arg "load:missing" assign"#]);
    assert!(editor.result() == false);
    assert!(editor.message().is_some_and(|text| text.starts_with("missing: cannot load extension: ")));
    let editor = edit(dir.path(), &[r#"arg "load:empty" assign"#]);
    assert_eq!(editor.message(), Some("empty: cannot load extension: not an extension: it needs MepCallbacks (exthdr.c), WhenLoaded, cmdTable and swiTable"));
}
