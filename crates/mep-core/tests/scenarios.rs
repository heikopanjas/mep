//! Scenario test harness (IMPLEMENTATION_PLAN.md §9.3)
//!
//! Runs every `tests/scenarios/*.mep-test` file of the workspace. A file holds
//! cases that start with `== name: …`; sections start with `-- `.
//!
//! Setup sections:
//! - `-- tools.ini`, `-- file NAME [crlf] [noeol] [readonly]`, `-- clipboard [record SHAPE | unavailable]`
//! - `-- args NAME …`, `-- size COLS ROWS`, `-- env NAME=VALUE`, `-- os windows|macos|linux`
//! - `-- slice N`, `-- notoolsini`, `-- temporary`, `-- execute TEXT` (a startup `-e` command)
//!
//! Sections that run in order:
//! - `-- keys`: items in macro syntax; function names run functions, other names are keys, quoted strings are typed key by key, `key:NAME` sends a key whose name is
//!   also a function, `hold` keeps the next requests pending, `tick` runs one slice
//! - `-- release` serves the held requests; `-- inject saved|loaded|clipboard|ini` answers the first held request with that reply kind instead
//! - `-- fail read|write|rename|metadata|readdir|createdir PATH` (relative to `/work` or absolute), `-- resize COLS ROWS`
//! - `-- expect file NAME [crlf] [noeol]`, `-- expect disk NAME …`, `-- expect cursor L C`, `-- expect window TOP LEFT`, `-- expect clipboard line|stream|box W`, `--
//!   expect message [TEXT]`, `-- expect status`, `-- expect frame`, `-- expect result true|false`, `-- expect quit`, `-- expect waiting`, `-- expect current NAME`
//!
//! Bodies end every line with a line break unless `noeol` is given; `~` is an empty line and
//! `\t`, `\f`, `\e`, `\b`, `\xff` stand for tab, form feed, escape, backspace and an invalid byte.

#![feature(coverage_attribute)]
#![coverage(off)]
#![allow(clippy::bool_comparison)]

use std::{
    fs,
    path::{Path, PathBuf}
};

use mep_core::{
    Editor, Event, Startup,
    clip::{Clip, ClipboardPayload, MemClipboard, Shape},
    env::{FixedEnv, Os},
    file::FileName,
    io::{Completion, Reply, Request, serve},
    keys::{Key, KeyCode},
    macros::{self, Item},
    memfs::{MemFs, Op}
};

/// One section of a case: header words and body lines
#[derive(Debug, Clone)]
struct Section
{
    head: Vec<String>,
    body: Vec<String>
}

/// A parsed case
#[derive(Debug)]
struct Case
{
    file:     String,
    name:     String,
    sections: Vec<Section>
}

fn parse(file: &str, text: &str) -> Vec<Case>
{
    let mut cases: Vec<Case> = Vec::new();
    for line in text.lines()
    {
        if let Some(name) = line.strip_prefix("== name:")
        {
            cases.push(Case { file: file.to_string(), name: name.trim().to_string(), sections: Vec::new() });
        }
        else if let Some(head) = line.strip_prefix("-- ")
        {
            let case = cases.last_mut().expect("section before case");
            case.sections.push(Section { head: head.split_whitespace().map(String::from).collect(), body: Vec::new() });
        }
        else if let Some(section) = cases.last_mut().and_then(|case| case.sections.last_mut())
        {
            section.body.push(line.to_string());
        }
    }
    // The blank lines that separate cases do not belong to the last section.
    for section in cases.iter_mut().flat_map(|case| case.sections.iter_mut())
    {
        while section.body.last().is_some_and(String::is_empty)
        {
            section.body.pop();
        }
    }
    cases
}

/// Joins body lines into file bytes
fn body_bytes(body: &[String], flags: &[String]) -> Vec<u8>
{
    let newline = if flags.iter().any(|flag| flag == "crlf")
    {
        "\r\n"
    }
    else
    {
        "\n"
    };
    // `~` stands for an empty line, so trailing empty lines survive the case separator trimming.
    let mut text: String = body
        .iter()
        .map(|line| {
            format!(
                "{}{newline}",
                if line == "~"
                {
                    ""
                }
                else
                {
                    line
                }
            )
        })
        .collect();
    if flags.iter().any(|flag| flag == "noeol")
    {
        text.truncate(text.len() - newline.len().min(text.len()));
    }
    let text = text.replace("\\t", "\t").replace("\\f", "\u{c}").replace("\\e", "\u{1b}");
    // `\xff` stands for an invalid UTF-8 byte.
    let text = text.replace("\\b", "\u{8}");
    text.split("\\xff").map(str::as_bytes).collect::<Vec<_>>().join(&0xff)
}

struct Run
{
    editor: Editor,
    fs:     MemFs,
    clip:   MemClipboard,
    held:   Vec<Request>,
    hold:   bool,
    paused: bool,
    quit:   bool
}

impl Run
{
    /// Serves all requests until none are left, keeping one back when `hold` is set
    fn settle(&mut self, mut requests: Vec<Request>, busy: bool)
    {
        let mut busy = busy;
        loop
        {
            if self.hold == true && (requests.is_empty() == false || busy == true)
            {
                self.held.append(&mut requests);
                self.paused = busy;
                self.hold = false;
            }
            if requests.is_empty() == false
            {
                let request = requests.remove(0);
                let completion = serve(request, &self.fs, &mut self.clip);
                let outcome = self.editor.handle(Event::Completed(completion));
                self.quit |= outcome.quit;
                busy = outcome.busy;
                requests.extend(outcome.requests);
            }
            else if busy == true && self.paused == false
            {
                let outcome = self.editor.tick();
                self.quit |= outcome.quit;
                busy = outcome.busy;
                requests.extend(outcome.requests);
            }
            else
            {
                break;
            }
        }
    }

    fn event(&mut self, event: Event)
    {
        let outcome = self.editor.handle(event);
        self.quit |= outcome.quit;
        self.settle(outcome.requests, outcome.busy);
    }

    fn press(&mut self, name: &str)
    {
        let outcome = self.editor.press(name);
        self.quit |= outcome.quit;
        self.settle(outcome.requests, outcome.busy);
    }
}

fn shape_of(words: &[String]) -> Shape
{
    match words.first().map(String::as_str)
    {
        | Some("line") => Shape::Line,
        | Some("box") => Shape::Box { width: words.get(1).and_then(|width| width.parse().ok()).unwrap_or(0) },
        | _ => Shape::Stream
    }
}

fn run_case(case: &Case) -> Vec<String>
{
    let mut env = FixedEnv::default();
    env.vars.insert("INIT".into(), "/ini".into());
    let fs = MemFs::default();
    let mut clip = MemClipboard::default();
    let (mut cols, mut rows, mut args, mut slice) = (40u16, 8u16, Vec::new(), None);
    let (mut tools_ini, mut execute, mut temporary) = (true, Vec::new(), false);
    for section in &case.sections
    {
        let head: Vec<&str> = section.head.iter().map(String::as_str).collect();
        match head.as_slice()
        {
            | ["tools.ini"] => drop(fs.write_file("/ini/tools.ini", section.body.join("\n").as_bytes())),
            | ["file", name, flags @ ..] =>
            {
                let path = Path::new("/work").join(name);
                let flags: Vec<String> = flags.iter().map(|flag| flag.to_string()).collect();
                fs.write_file(&path, &body_bytes(&section.body, &flags));
                if flags.iter().any(|flag| flag == "readonly")
                {
                    fs.set_readonly(path, true);
                }
            }
            | ["clipboard", rest @ ..] =>
            {
                let text = section
                    .body
                    .iter()
                    .map(|line| {
                        if line == "~"
                        {
                            ""
                        }
                        else
                        {
                            line.as_str()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let record = match rest
                {
                    | ["record", shape @ ..] =>
                    {
                        let shape = shape_of(&shape.iter().map(|word| word.to_string()).collect::<Vec<_>>());
                        Some(mep_core::clip::Record { shape, hash: mep_core::clip::fnv1a64(text.as_bytes()) }.encode())
                    }
                    | ["unavailable"] =>
                    {
                        clip.unavailable = true;
                        None
                    }
                    | _ => None
                };
                clip.content = (clip.unavailable == false).then(|| ClipboardPayload { text: text.clone(), record });
            }
            | ["args", names @ ..] => args = names.iter().map(|name| name.to_string()).collect(),
            | ["size", c, r] => (cols, rows) = (c.parse().unwrap(), r.parse().unwrap()),
            | ["env", pair] =>
            {
                let (name, value) = pair.split_once('=').unwrap();
                env.vars.insert(name.into(), value.into());
            }
            | ["os", os] =>
            {
                env.os = match *os
                {
                    | "windows" => Os::Windows,
                    | "macos" => Os::MacOs,
                    | _ => Os::Linux
                };
            }
            | ["slice", lines] => slice = lines.parse().ok(),
            | ["notoolsini"] => tools_ini = false,
            | ["temporary"] => temporary = true,
            | ["execute", words @ ..] => execute.push(words.join(" ")),
            | _ =>
            {}
        }
    }
    let startup = Startup { cols, rows, exe: "mep".into(), files: args, execute, tools_ini, temporary };
    let mut editor = Editor::new(Box::new(env), startup);
    if let Some(lines) = slice
    {
        editor.set_slice_lines(lines);
    }
    let outcome = editor.start();
    let mut run = Run { editor, fs, clip, held: Vec::new(), hold: false, paused: false, quit: outcome.quit };
    run.settle(outcome.requests, outcome.busy);
    let mut failures = Vec::new();
    for section in &case.sections
    {
        let head: Vec<&str> = section.head.iter().map(String::as_str).collect();
        let fail =
            |what: &str, want: &dyn std::fmt::Debug, got: &dyn std::fmt::Debug| format!("{}: {}: {what}\n  want: {want:?}\n  got:  {got:?}", case.file, case.name);
        match head.as_slice()
        {
            | ["keys"] =>
            {
                for line in &section.body
                {
                    for item in macros::parse(line).expect("keys parse")
                    {
                        match item
                        {
                            | Item::Call(name) if name == "hold" => run.hold = true,
                            | Item::Call(name) if name == "tick" =>
                            {
                                let outcome = run.editor.tick();
                                run.settle(outcome.requests, outcome.busy);
                            }
                            | Item::Call(name) if name.starts_with("key:") => run.event(Event::Key(Key::parse(&name[4..]).expect("key name"))),
                            | Item::Call(name) if mep_core::is_function(&name) => run.press(&name),
                            | Item::Call(name) => match Key::parse(&name)
                            {
                                | Ok(key) => run.event(Event::Key(key)),
                                | Err(_) => run.press(&name)
                            },
                            | Item::Text(text) =>
                            {
                                for ch in text.chars()
                                {
                                    run.event(Event::Key(Key::plain(KeyCode::Char(ch))));
                                }
                            }
                            | other => panic!("unsupported key item {other:?}")
                        }
                    }
                }
            }
            | ["expect", "file" | "disk", name, flags @ ..] =>
            {
                let flags: Vec<String> = flags.iter().map(|flag| flag.to_string()).collect();
                let want = body_bytes(&section.body, &flags);
                let got = match head[1]
                {
                    | "file" => run.editor.buffer(&FileName::parse(name, Path::new("/work"))),
                    | _ => run.fs.file(Path::new("/work").join(name))
                };
                if got.as_deref() != Some(want.as_slice())
                {
                    failures.push(fail(&format!("{} {name}", head[1]), &String::from_utf8_lossy(&want), &got.map(|got| String::from_utf8_lossy(&got).into_owned())));
                }
            }
            | ["expect", "cursor", line, col] =>
            {
                let want = (line.parse::<usize>().unwrap(), col.parse::<usize>().unwrap());
                if run.editor.cursor_position() != want
                {
                    failures.push(fail("cursor", &want, &run.editor.cursor_position()));
                }
            }
            | ["expect", "window", top, left] =>
            {
                let want = (top.parse::<usize>().unwrap(), left.parse::<usize>().unwrap());
                if run.editor.window_position() != want
                {
                    failures.push(fail("window", &want, &run.editor.window_position()));
                }
            }
            | ["expect", "clipboard", shape @ ..] =>
            {
                let shape = shape_of(&shape.iter().map(|word| word.to_string()).collect::<Vec<_>>());
                let want = Clip { shape, lines: section.body.iter().map(|line| body_bytes(std::slice::from_ref(line), &["noeol".to_string()])).collect() };
                if run.editor.clipboard() != &want
                {
                    failures.push(fail("clipboard", &want, run.editor.clipboard()));
                }
            }
            | ["expect", "message", words @ ..] =>
            {
                let want = (words.is_empty() == false).then(|| words.join(" "));
                if run.editor.message().map(String::from) != want
                {
                    failures.push(fail("message", &want, &run.editor.message()));
                }
            }
            | ["expect", "status"] =>
            {
                let want = section.body.join("\n");
                if run.editor.status() != want
                {
                    failures.push(fail("status", &want, &run.editor.status()));
                }
            }
            | ["expect", "frame"] =>
            {
                let got = run.editor.render().to_text();
                let want = section.body.join("\n");
                if got.trim_end_matches('\n') != want.trim_end_matches('\n')
                {
                    failures.push(fail("frame", &want, &got));
                }
            }
            | ["expect", "result", value] =>
            {
                let want = *value == "true";
                if run.editor.result() != want
                {
                    failures.push(fail("result", &want, &run.editor.result()));
                }
            }
            | ["expect", "quit"] =>
            {
                if run.quit == false
                {
                    failures.push(fail("quit", &true, &false));
                }
            }
            | ["expect", "meta", state] =>
            {
                let want = *state == "on";
                let got = run.editor.status().ends_with(" meta");
                if got != want
                {
                    failures.push(fail("meta", &want, &got));
                }
            }
            | ["expect", "waiting"] =>
            {
                if run.editor.is_waiting() == false
                {
                    failures.push(fail("waiting", &true, &false));
                }
            }
            | ["expect", "current", name] =>
            {
                let want = FileName::parse(name, Path::new("/work"));
                if run.editor.current_name() != want
                {
                    failures.push(fail("current", &want, &run.editor.current_name()));
                }
            }
            | ["fail", op, name] =>
            {
                let op = match *op
                {
                    | "read" => Op::Read,
                    | "write" => Op::Write,
                    | "rename" => Op::Rename,
                    | "metadata" => Op::Metadata,
                    | "createdir" => Op::CreateDir,
                    | _ => Op::ReadDir
                };
                run.fs.fail(op, Path::new("/work").join(name));
                run.fs.fail(op, Path::new(name));
            }
            | ["resize", c, r] => run.event(Event::Resize(c.parse().unwrap(), r.parse().unwrap())),
            | ["close"] => run.event(Event::Close),
            | ["inject", kind] =>
            {
                let request = run.held.remove(0);
                let reply = match *kind
                {
                    | "saved" => Reply::Saved(Ok(())),
                    | "loaded" => Reply::Loaded(Ok(mep_core::io::Loaded { bytes: Vec::new(), readonly: false })),
                    | "clipboard" => Reply::ClipboardRead(None),
                    | _ => Reply::Ini(Ok(None))
                };
                run.event(Event::Completed(Completion { id: request.id, reply }));
            }
            | ["release"] =>
            {
                let held = std::mem::take(&mut run.held);
                let busy = run.paused;
                run.paused = false;
                run.settle(held, busy);
            }
            | ["expect", ..] => failures.push(format!("{}: {}: unknown expectation {head:?}", case.file, case.name)),
            | _ =>
            {}
        }
    }
    failures
}

fn scenario_files() -> Vec<PathBuf>
{
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scenarios");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "mep-test"))
        .collect();
    files.sort();
    files
}

#[test]
fn scenarios()
{
    let mut failures = Vec::new();
    let mut count = 0;
    for path in scenario_files()
    {
        let text = fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        for case in parse(&name, &text)
        {
            count += 1;
            failures.extend(run_case(&case));
        }
    }
    assert!(failures.is_empty(), "{} of {count} scenario checks failed:\n{}", failures.len(), failures.join("\n"));
}

/// Writes a file into a MemFs, for fixtures
trait WriteFile
{
    fn write_file(&self, path: impl AsRef<Path>, bytes: &[u8]) -> bool;
}

impl WriteFile for MemFs
{
    fn write_file(&self, path: impl AsRef<Path>, bytes: &[u8]) -> bool
    {
        mep_core::io::Fs::write_atomic(self, path.as_ref(), bytes).is_ok()
    }
}
