//! Performance tests on the reference document (IMPLEMENTATION_PLAN.md §3.5, §9.6)
//!
//! Ignored by default; run in release mode before accepting a milestone:
//! `cargo test --release -p mep-core --test perf -- --ignored perf_`.
//! The reference document has 2 million lines and 100 MB.

#![feature(coverage_attribute)]
#![coverage(off)]
#![allow(clippy::bool_comparison)]

use std::time::{Duration, Instant};

use mep_core::{
    Editor, Event, Outcome, Startup,
    clip::MemClipboard,
    env::FixedEnv,
    io::serve,
    keys::{Key, KeyCode},
    memfs::MemFs
};

/// Lines of the reference document
const LINES: usize = 2_000_000;

/// Bytes per line, line break included: 2 million lines make 100 MB
const LINE_BYTES: usize = 50;

/// One frame at 60 Hz
const FRAME: Duration = Duration::from_millis(16);

/// Builds the reference document: numbered lines of 50 bytes
fn reference() -> Vec<u8>
{
    let mut bytes = Vec::with_capacity(LINES * LINE_BYTES);
    for line in 0..LINES
    {
        let text = format!("{line:08} the quick brown fox jumps over the lazy dog");
        bytes.extend_from_slice(&text.as_bytes()[..LINE_BYTES - 1]);
        bytes.push(b'\n');
    }
    bytes
}

/// Times one step
fn time<T>(step: impl FnOnce() -> T) -> (T, Duration)
{
    let start = Instant::now();
    let result = step();
    (result, start.elapsed())
}

/// Checks a measurement against its limit and reports it
fn check(what: &str, took: Duration, limit: Duration)
{
    eprintln!("{what:<40} {took:>12.3?} (limit {limit:?})");
    assert!(took < limit, "{what} took {took:?}, more than {limit:?}");
}

/// Refuses to measure a debug build; its timings mean nothing
fn require_release()
{
    assert!(cfg!(debug_assertions) == false, "run the performance tests with --release");
}

/// Starts an editor on the reference document; returns it, the time the load completion took, and the file system
fn open() -> (Editor, Duration, MemFs)
{
    let fs = MemFs::default().with_file("/work/big.txt", &reference());
    let startup = Startup {
        cols:      80,
        rows:      25,
        exe:       "mep".into(),
        files:     vec!["big.txt".into()],
        execute:   Vec::new(),
        tools_ini: false,
        temporary: true
    };
    let mut editor = Editor::new(Box::new(FixedEnv::default()), startup);
    let mut outcome = editor.start();
    let mut clipboard = MemClipboard::default();
    let mut load = Duration::ZERO;
    while outcome.requests.is_empty() == false
    {
        // Reading the bytes is the I/O worker's job; the core's share is taking the completion.
        let completion = serve(outcome.requests.remove(0), &fs, &mut clipboard);
        let (next, took) = time(|| editor.handle(Event::Completed(completion)));
        load += took;
        outcome = next;
    }
    assert_eq!(editor.cursor_position(), (0, 0));
    (editor, load, fs)
}

fn key(editor: &mut Editor, code: KeyCode) -> Outcome
{
    editor.handle(Event::Key(Key::plain(code)))
}

/// Puts the cursor on a 0-based line with `arg "n" mark`
fn go_to(editor: &mut Editor, line: usize)
{
    editor.press("arg");
    for digit in line.to_string().chars()
    {
        key(editor, KeyCode::Char(digit));
    }
    editor.press("mark");
    assert_eq!(editor.cursor_position().0, line);
}

#[test]
#[ignore = "performance test: run with --release"]
fn perf_load_reference_document()
{
    require_release();
    let (_, load, _) = open();
    check("load into buffer and line index", load, Duration::from_secs(1));
}

#[test]
#[ignore = "performance test: run with --release"]
fn perf_edits_moves_and_frames_anywhere_in_the_document()
{
    require_release();
    let (mut editor, ..) = open();
    let (_, took) = time(|| {
        editor.press("arg");
        editor.press("ppage")
    });
    check("move to the last line", took, FRAME);
    assert_eq!(editor.cursor_position().0, LINES - 1);
    for (place, line) in [("start", 0), ("middle", LINES / 2), ("end", LINES - 1)]
    {
        go_to(&mut editor, line);
        let (_, took) = time(|| key(&mut editor, KeyCode::Char('x')));
        check(&format!("insert a character at the {place}"), took, FRAME);
        let (_, took) = time(|| editor.press("left"));
        check(&format!("move left at the {place}"), took, FRAME);
        let (_, took) = time(|| editor.press("sdelete"));
        check(&format!("delete a character at the {place}"), took, FRAME);
        let (_, took) = time(|| editor.render());
        check(&format!("render a frame at the {place}"), took, FRAME);
        let (_, took) = time(|| editor.press("ppage"));
        check(&format!("page down at the {place}"), took, FRAME);
        let (_, took) = time(|| editor.press("down"));
        check(&format!("move down at the {place}"), took, FRAME);
    }
    let (_, took) = time(|| editor.press("undo"));
    check("undo an edit", took, FRAME);
    let (_, took) = time(|| editor.status());
    check("status line", took, FRAME);
}

#[test]
#[ignore = "performance test: run with --release"]
fn perf_search_slices_stay_within_a_frame()
{
    require_release();
    let (mut editor, ..) = open();
    editor.press("arg");
    for ch in "not in the document".chars()
    {
        key(&mut editor, KeyCode::Char(ch));
    }
    let (mut outcome, first) = time(|| editor.press("psearch"));
    check("first search slice", first, FRAME);
    let (mut slices, mut slowest, mut total) = (1, first, first);
    while outcome.busy == true
    {
        let (next, took) = time(|| editor.tick());
        (outcome, slowest, total, slices) = (next, slowest.max(took), total + took, slices + 1);
    }
    eprintln!("search over the whole document: {slices} slices, {total:?}");
    check("slowest search slice", slowest, FRAME);
    assert_eq!(editor.message(), Some("not in the document not found"));
}

#[test]
#[ignore = "performance test: run with --release"]
fn perf_saving_hands_over_one_copy()
{
    require_release();
    let (mut editor, _, fs) = open();
    key(&mut editor, KeyCode::Char('x'));
    let (outcome, took) = time(|| {
        editor.press("arg");
        editor.press("arg");
        editor.press("setfile")
    });
    check("prepare a save of the whole document", took, Duration::from_secs(1));
    assert_eq!(outcome.requests.len(), 1);
    assert!(fs.file("/work/big.txt").is_some());
}
