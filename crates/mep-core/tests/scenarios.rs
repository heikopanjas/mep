//! Scenario test harness (IMPLEMENTATION_PLAN.md §9.3)
//!
//! Runs every `tests/scenarios/*.mep-test` file of the workspace. A file holds
//! cases that start with `== name: …`; sections start with `-- `.
//!
//! Setup sections:
//! - `-- tools.ini`, `-- file NAME [crlf] [noeol] [readonly]`, `-- clipboard [record SHAPE | unavailable]`
//! - `-- args NAME …`, `-- size COLS ROWS`, `-- env NAME=VALUE`, `-- os windows|macos|linux` (also the path style: `\work\a.txt` on Windows)
//! - `-- slice N`, `-- notoolsini`, `-- temporary`, `-- execute TEXT` (a startup `-e` command)
//! - `-- readkeys KEY…`: keys an extension reads with `ReadChar`/`ReadCmd` after the typed-ahead ones; `load:…/probe.so` loads the scripted `probe` extension, whose
//!   text argument is a callback command (see `probe`)
//! - `-- run [fail] COMMAND…`: the body is the output of the command line for `compile`, `paste` and `shell`; `fail` makes it exit unsuccessfully. Unknown command
//!   lines cannot start in the background and succeed on the terminal.
//!
//! Sections that run in order:
//! - `-- keys`: items in macro syntax; function names run functions, other names are keys, quoted strings are typed key by key, `key:NAME` sends a key whose name is
//!   also a function, `hold` keeps the next requests pending, `tick` runs one slice
//! - `-- release` serves the held requests; `-- inject saved|loaded|clipboard|ini` answers the first held request or command with that reply kind instead
//! - `-- fail read|write|rename|metadata|readdir|createdir PATH` (relative to `/work` or absolute), `-- resize COLS ROWS`
//! - `-- expect file NAME [crlf] [noeol]`, `-- expect disk NAME …`, `-- expect cursor L C`, `-- expect window TOP LEFT`, `-- expect clipboard line|stream|box W`, `--
//!   expect message [TEXT]`, `-- expect status`, `-- expect frame`, `-- expect result true|false`, `-- expect quit`, `-- expect waiting`, `-- expect current NAME`
//! - `-- expect shape block|bar|underline` (the cursor shape),
//! - `-- expect log` (the commands run so far, one per line: `run: …`, `shell: …`, `shell` for the interactive shell, `wait` for askrtn; clears the log), `-- expect
//!   bell` (a bell rang since the last check)
//!
//! Bodies end every line with a line break unless `noeol` is given; `~` is an empty line and
//! `\t`, `\f`, `\e`, `\b`, `\xff` stand for tab, form feed, escape, backspace and an invalid byte.

#![feature(coverage_attribute)]
#![coverage(off)]
#![allow(clippy::bool_comparison)]

use std::{
    cell::RefCell,
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc
};

use mep_core::{
    Editor, Event, Startup, Utf8TypedPathBuf,
    clip::{Clip, ClipboardPayload, MemClipboard, Shape},
    env::{Env, FixedEnv, Os},
    ext::{self, ExtApi, ExtArg, ExtFunction, ExtSwitch, Extension, SwitchKind, SwitchValue},
    file::FileName,
    host::Host,
    io::{Completion, Reply, Request, serve},
    keys::{Key, KeyCode},
    macros::{self, Item},
    memfs::{MemFs, Op},
    run::{self, Ran, Runner}
};

/// The host: scripted command lines, the file system and clipboard, the `probe` extension and the log of what ran
#[derive(Default)]
struct Fake
{
    scripts: HashMap<String, (bool, Vec<u8>)>,
    log:     RefCell<Vec<String>>,
    fs:      MemFs,
    clip:    RefCell<MemClipboard>,
    keys:    RefCell<std::collections::VecDeque<Key>>
}

impl Fake
{
    fn note(&self, text: String)
    {
        self.log.borrow_mut().push(text);
    }
}

/// Resolves a handle word of a probe command: `.` the current file, a number, or a file name
fn handle(api: &mut dyn ExtApi, word: &str) -> usize
{
    match word
    {
        | "." => api.file_handle("", None),
        | word => word.parse().unwrap_or_else(|_| api.file_handle(word, None))
    }
}

/// Runs one probe command against the callbacks and returns what to log
fn probe(api: &mut dyn ExtApi, command: &str) -> String
{
    let words: Vec<&str> = command.split_whitespace().collect();
    let n = |index: usize| words[index].parse::<usize>().unwrap();
    let rest = |index: usize| words[index..].join(" ");
    match words[0]
    {
        | "add" => format!("{}", api.add_file(words[1])),
        | "badarg" => format!("{}", api.bad_arg()),
        | "copybox" =>
        {
            let (src, dst) = (handle(api, words[1]), handle(api, words[2]));
            api.copy_box(src, dst, [n(3), n(4), n(5), n(6)], [n(7), n(8)]);
            String::new()
        }
        | "copyline" =>
        {
            let (src, dst) = (handle(api, words[1]), handle(api, words[2]));
            api.copy_line(src, dst, n(3), n(4), n(5));
            String::new()
        }
        | "copystream" =>
        {
            let (src, dst) = (handle(api, words[1]), handle(api, words[2]));
            api.copy_stream(src, dst, [n(3), n(4), n(5), n(6)], [n(7), n(8)]);
            String::new()
        }
        | "delbox" =>
        {
            let file = handle(api, words[1]);
            api.del_box(file, [n(2), n(3), n(4), n(5)]);
            String::new()
        }
        | "delfile" =>
        {
            let file = handle(api, words[1]);
            api.del_file(file);
            String::new()
        }
        | "delline" =>
        {
            let file = handle(api, words[1]);
            api.del_line(file, n(2), n(3));
            String::new()
        }
        | "delstream" =>
        {
            let file = handle(api, words[1]);
            api.del_stream(file, [n(2), n(3), n(4), n(5)]);
            String::new()
        }
        | "msg" => format!("{}", api.message(&rest(1))),
        | "display" =>
        {
            api.display();
            String::new()
        }
        | "exec" => format!("{}", api.execute(&rest(1))),
        | "length" =>
        {
            let file = handle(api, words[1]);
            format!("{}", api.file_length(file))
        }
        | "handle" => format!("{}", api.file_handle(words[1], words.get(2).copied())),
        | "read" =>
        {
            let file = handle(api, words[2]);
            format!("{}", api.file_read(words[1], file))
        }
        | "write" =>
        {
            let file = handle(api, words[2]);
            format!("{}", api.file_write(words[1], file))
        }
        | "cursor" => format!("{:?}", api.get_cursor()),
        | "getline" =>
        {
            let file = handle(api, words[2]);
            String::from_utf8_lossy(&api.get_line(n(1), file)).into_owned()
        }
        | "unhook" | "hook" =>
        {
            api.keyboard(words[0] == "hook");
            String::new()
        }
        | "move" =>
        {
            api.move_cursor(n(1), n(2));
            String::new()
        }
        | "top" =>
        {
            let file = handle(api, words[1]);
            api.to_top(file);
            String::new()
        }
        | "putline" =>
        {
            let file = handle(api, words[2]);
            api.put_line(n(1), rest(3).as_bytes(), file);
            String::new()
        }
        | "remove" =>
        {
            let file = handle(api, words[1]);
            api.remove_file(file);
            String::new()
        }
        | "char" => format!("{}", api.read_char()),
        | "cmd" => format!("{:?}", api.read_command()),
        | "replace" =>
        {
            let file = handle(api, words[4]);
            format!("{}", api.replace_char(words[1].as_bytes()[0], n(2), n(3), file, words[5] == "insert"))
        }
        | "setkey" => format!("{}", api.set_key(words[1], words[2])),
        | other => panic!("unknown probe command {other}")
    }
}

impl Runner for Fake
{
    fn capture(&self, command: &str, _: &Utf8TypedPathBuf, _: bool) -> io::Result<Ran>
    {
        self.log.borrow_mut().push(format!("run: {command}"));
        let (success, output) = self.scripts.get(command).cloned().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{command}: not found")))?;
        Ok(Ran { success, output })
    }
}

impl Host for Fake
{
    fn serve(&self, request: Request) -> Option<Completion>
    {
        // TOOLS.INI is served only in the background, to reach the path where a host cannot serve at once.
        match request.kind
        {
            | mep_core::io::RequestKind::FindIni(_) => None,
            | _ => Some(serve(request, &self.fs, &mut *self.clip.borrow_mut()))
        }
    }

    fn run(&self, job: run::Run) -> Option<Completion>
    {
        Some(run::execute(job, self))
    }

    fn read_key(&self) -> Option<Key>
    {
        self.keys.borrow_mut().pop_front()
    }

    fn keyboard(&self, hooked: bool)
    {
        self.note(format!("keyboard {hooked}"));
    }

    fn load(&self, path: &Utf8TypedPathBuf) -> Result<Extension, String>
    {
        let switch = |name: &str, kind, id| ExtSwitch { name: name.into(), kind, id };
        let function = |name: &str, flags, id| ExtFunction { name: name.into(), flags, id };
        let all = ext::flags::ALL & !(ext::flags::CURSORFUNC | ext::flags::WINDOWFUNC | ext::flags::BOXSTR | ext::flags::NULLEOL | ext::flags::NULLEOW);
        match path.file_name()
        {
            | Some("probe.so") => Ok(Extension {
                id:        7,
                functions: vec![function("Probe", all, 0), function("psearch", ext::flags::NOARG, 1), function("probebox", ext::flags::BOXSTR, 2)],
                switches:  vec![switch("probeflag", SwitchKind::Boolean, 0), switch("probenum", SwitchKind::Numeric, 1), switch("probetext", SwitchKind::Special, 2)]
            }),
            | _ => Err(format!("{}: no such library", path))
        }
    }

    fn init(&self, library: usize, api: &mut dyn ExtApi)
    {
        self.note(format!("init {library} {}", api.set_key("probe", "alt+p")));
    }

    fn call(&self, id: usize, arg: &ExtArg, key: u32, meta: bool, api: &mut dyn ExtApi) -> bool
    {
        let (text, result) = match arg
        {
            | ExtArg::Text { count: 1, text, .. } if id == 0 => (probe(api, text), text.starts_with("badarg") == false),
            | arg => (format!("{id} {arg:?} key {key} meta {meta}"), true)
        };
        self.note(format!("probe: {text}").trim_end().to_string());
        result
    }

    fn set_switch(&self, id: usize, value: &SwitchValue, _: &mut dyn ExtApi) -> bool
    {
        self.note(format!("switch {id} {value:?}"));
        *value != SwitchValue::Special("bad".into())
    }

    fn shell(&self, commands: &[String], wait: bool) -> bool
    {
        let mut log = self.log.borrow_mut();
        if commands.is_empty()
        {
            log.push("shell".into());
        }
        log.extend(commands.iter().map(|command| format!("shell: {command}")));
        if wait == true
        {
            log.push("wait".into());
        }
        commands.iter().all(|command| self.scripts.get(command).is_none_or(|(success, _)| *success))
    }
}

/// Work for the harness to serve: a worker request or a background command
enum Work
{
    Request(Request),
    Run(run::Run)
}

/// Collects an outcome's work in the order a front end would start it
fn work(outcome: &mut mep_core::Outcome) -> Vec<Work>
{
    let requests = std::mem::take(&mut outcome.requests).into_iter().map(Work::Request);
    requests.chain(std::mem::take(&mut outcome.runs).into_iter().map(Work::Run)).collect()
}

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
    fake:   Rc<Fake>,
    held:   Vec<Work>,
    hold:   bool,
    paused: bool,
    quit:   bool,
    bell:   bool
}

impl Run
{
    /// Serves all requests until none are left, keeping one back when `hold` is set
    fn settle(&mut self, mut outcome: mep_core::Outcome)
    {
        let mut queue = work(&mut outcome);
        let mut busy = outcome.busy;
        self.quit |= outcome.quit;
        self.bell |= outcome.bell;
        loop
        {
            if self.hold == true && (queue.is_empty() == false || busy == true)
            {
                self.held.append(&mut queue);
                self.paused = busy;
                self.hold = false;
            }
            let next = if queue.is_empty() == false
            {
                let completion = match queue.remove(0)
                {
                    | Work::Request(request) => serve(request, &self.fake.fs, &mut *self.fake.clip.borrow_mut()),
                    | Work::Run(job) => run::execute(job, self.fake.as_ref())
                };
                Some(self.editor.handle(Event::Completed(completion)))
            }
            else if busy == true && self.paused == false
            {
                Some(self.editor.tick())
            }
            else
            {
                None
            };
            let Some(mut outcome) = next
            else
            {
                break;
            };
            self.quit |= outcome.quit;
            self.bell |= outcome.bell;
            busy = outcome.busy;
            queue.extend(work(&mut outcome));
        }
    }

    /// Serves held work as if it had been there all along
    fn release(&mut self, held: Vec<Work>, busy: bool)
    {
        let mut outcome = mep_core::Outcome { busy, ..Default::default() };
        for item in held
        {
            match item
            {
                | Work::Request(request) => outcome.requests.push(request),
                | Work::Run(job) => outcome.runs.push(job)
            }
        }
        self.settle(outcome);
    }

    fn event(&mut self, event: Event)
    {
        let outcome = self.editor.handle(event);
        self.settle(outcome);
    }

    fn press(&mut self, name: &str)
    {
        let outcome = self.editor.press(name);
        self.settle(outcome);
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
    let mut fake = Fake::default();
    for section in &case.sections
    {
        let head: Vec<&str> = section.head.iter().map(String::as_str).collect();
        match head.as_slice()
        {
            | ["tools.ini"] => drop(fs.write_file("/ini/tools.ini", section.body.join("\n").as_bytes())),
            | ["file", name, flags @ ..] =>
            {
                let path = in_work(name);
                let flags: Vec<String> = flags.iter().map(|flag| flag.to_string()).collect();
                fs.write_file(&path, &body_bytes(&section.body, &flags));
                if flags.iter().any(|flag| flag == "readonly")
                {
                    fs.set_readonly(&path, true);
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
            | ["run", words @ ..] =>
            {
                let (success, words) = match words
                {
                    | ["fail", rest @ ..] => (false, rest),
                    | _ => (true, words)
                };
                let output = section.body.iter().map(|line| format!("{line}\n")).collect::<String>();
                fake.scripts.insert(words.join(" "), (success, output.into_bytes()));
            }
            | ["execute", words @ ..] => execute.push(words.join(" ")),
            | ["readkeys", names @ ..] => fake.keys.borrow_mut().extend(names.iter().map(|name| Key::parse(name).expect("key name"))),
            | _ =>
            {}
        }
    }
    let startup = Startup { cols, rows, exe: "mep".into(), files: args, execute, tools_ini, temporary };
    // Names in expectations resolve like the editor's own, with the separators of the scenario's OS.
    let cwd = env.cwd();
    let mut editor = Editor::new(Box::new(env), startup);
    if let Some(lines) = slice
    {
        editor.set_slice_lines(lines);
    }
    fake.fs = fs;
    fake.clip = RefCell::new(clip);
    let fake = Rc::new(fake);
    editor.set_host(fake.clone());
    let outcome = editor.start();
    let mut run = Run { editor, fake, held: Vec::new(), hold: false, paused: false, quit: false, bell: false };
    run.settle(outcome);
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
                                run.settle(outcome);
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
                    | "file" => run.editor.buffer(&FileName::parse(name, &cwd)),
                    | _ => run.fake.fs.file(&in_work(name))
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
                let want = FileName::parse(name, &cwd);
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
                run.fake.fs.fail(op, &in_work(name));
                run.fake.fs.fail(op, name);
            }
            | ["resize", c, r] => run.event(Event::Resize(c.parse().unwrap(), r.parse().unwrap())),
            | ["close"] => run.event(Event::Close),
            | ["inject", kind] =>
            {
                let id = match run.held.remove(0)
                {
                    | Work::Request(request) => request.id,
                    | Work::Run(job) => job.id
                };
                let reply = match *kind
                {
                    | "saved" => Reply::Saved(Ok(())),
                    | "loaded" => Reply::Loaded(Ok(mep_core::io::Loaded { bytes: Vec::new(), readonly: false })),
                    | "clipboard" => Reply::ClipboardRead(None),
                    | _ => Reply::Ini(Ok(None))
                };
                run.event(Event::Completed(Completion { id, reply }));
            }
            | ["release"] =>
            {
                let held = std::mem::take(&mut run.held);
                let busy = run.paused;
                run.paused = false;
                run.release(held, busy);
            }
            | ["expect", "shape", want] =>
            {
                let got = format!("{:?}", run.editor.render().shape).to_lowercase();
                if got != *want
                {
                    failures.push(fail("shape", want, &got));
                }
            }
            | ["expect", "log"] =>
            {
                let got = std::mem::take(&mut *run.fake.log.borrow_mut());
                if got != section.body
                {
                    failures.push(fail("log", &section.body, &got));
                }
            }
            | ["expect", "bell"] =>
            {
                if std::mem::take(&mut run.bell) == false
                {
                    failures.push(fail("bell", &true, &false));
                }
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
    fn write_file(&self, path: &str, bytes: &[u8]) -> bool;
}

impl WriteFile for MemFs
{
    fn write_file(&self, path: &str, bytes: &[u8]) -> bool
    {
        mep_core::io::Fs::write_atomic(self, &Utf8TypedPathBuf::from_unix(path), bytes).is_ok()
    }
}

/// Returns the in-memory file system key of `name` relative to the working directory `/work`
fn in_work(name: &str) -> String
{
    Utf8TypedPathBuf::from_unix("/work").join(name).into_string()
}
