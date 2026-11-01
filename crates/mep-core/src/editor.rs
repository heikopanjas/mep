//! The editor state machine (IMPLEMENTATION_PLAN.md §3.5, §4.2, §6.5, §6.6, §6.14)
//!
//! [`Editor::handle`] takes keys, resizes and worker completions and returns
//! the requests for workers. Commands run one after another; a command that
//! needs I/O or an answer suspends until the completion or the key arrives.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    rc::Rc
};

use crate::{
    clip::{Clip, ClipboardPayload, Shape},
    cmd::{self, Desc, flags},
    commands::{
        replace::{Replace, Span},
        search::Search
    },
    env::{Env, Os},
    file::{File, FileId, FileName, FileTable},
    io::{Completion, Reply, Request, RequestId, RequestKind},
    keys::{Key, KeyCode, Keymap},
    macros::{Item, Jump, Macros},
    switches::Switches,
    view::{Pos, View}
};

/// Deepest macro nesting before a macro is aborted (plan §6.14)
pub const MAX_MACRO_DEPTH: usize = 256;

/// Lines a search scans per step before input is handled again (plan §3.5)
pub const SEARCH_SLICE_LINES: usize = 10_000;

/// Input to the editor
#[derive(Debug)]
pub enum Event
{
    /// A key press
    Key(Key),
    /// The terminal size changed to (columns, rows)
    Resize(u16, u16),
    /// A worker finished a request
    Completed(Completion),
    /// The front end can no longer show the editor, e.g. the terminal is gone; the editor quits
    Close
}

/// What the binary has to do after an event
#[derive(Debug, Default)]
pub struct Outcome
{
    /// The editor ended
    pub quit:     bool,
    /// Requests for the worker threads
    pub requests: Vec<Request>,
    /// The front end should repaint every cell (`setwindow`)
    pub redraw:   bool,
    /// A long operation continues: call [`Editor::tick`] when no input is waiting
    pub busy:     bool
}

/// How the editor starts (plan §6.1)
#[derive(Debug, Clone)]
pub struct Startup
{
    /// Terminal columns
    pub cols:      u16,
    /// Terminal rows
    pub rows:      u16,
    /// Editor name that selects the TOOLS.INI section
    pub exe:       String,
    /// Files from the command line
    pub files:     Vec<String>,
    /// `-e` commands in order
    pub execute:   Vec<String>,
    /// Read TOOLS.INI (not `-d`)
    pub tools_ini: bool
}

/// Color class of the dialog line message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind
{
    /// `infcolor`
    Info,
    /// `errcolor`
    Error
}

/// An argument as a command receives it (plan §6.5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Arg
{
    /// No `arg`
    None,
    /// `arg` without text or movement, at the anchor
    Null
    {
        count: usize, pos: Pos
    },
    /// Typed text, or text taken from the file for NULLEOL, NULLEOW and BOXSTR
    Text
    {
        count: usize, text: String, pos: Pos
    },
    /// A number for commands that count by themselves
    Num
    {
        count: usize, n: usize
    },
    /// Whole lines `first..=last`
    Line
    {
        count: usize, first: usize, last: usize
    },
    /// Characters from `start` up to but not including `end`
    Stream
    {
        count: usize, start: Pos, end: Pos
    },
    /// Lines `top..=bottom`, cells `left..right`
    Box
    {
        count: usize, top: usize, bottom: usize, left: usize, right: usize
    }
}

impl Arg
{
    /// Returns the number of `arg` presses; 0 without an argument
    pub(crate) fn count(&self) -> usize
    {
        match self
        {
            | Self::None => 0,
            | Self::Null { count, .. } |
            Self::Text { count, .. } |
            Self::Num { count, .. } |
            Self::Line { count, .. } |
            Self::Stream { count, .. } |
            Self::Box { count, .. } => *count
        }
    }

    /// Returns the position of a null or text argument (the anchor); the start of the file otherwise
    pub(crate) fn anchor(&self) -> Pos
    {
        match self
        {
            | Self::Null { pos, .. } | Self::Text { pos, .. } => *pos,
            | _ => Pos::default()
        }
    }

    /// Returns the text of a text argument; empty for other kinds
    pub(crate) fn into_text(self) -> String
    {
        match self
        {
            | Self::Text { text, .. } => text,
            | _ => String::new()
        }
    }
}

/// Result of a command
#[derive(Debug)]
pub(crate) enum Step
{
    /// Finished with a return value
    Done(bool),
    /// Suspended until the input arrives
    Wait(Pending)
}

/// What a suspended command waits for
#[derive(Debug)]
pub(crate) enum Pending
{
    /// A worker completion
    Request
    {
        id: RequestId, then: Then
    },
    /// A y/n answer
    Confirm
    {
        prompt: String, then: Then
    },
    /// Text typed and edited on the dialog line
    Prompt
    {
        prompt: String, edit: TextArg, then: Then
    },
    /// The next key, taken literally (`quote`)
    Quote,
    /// A search between slices
    Search(Box<Search>),
    /// A replace between slices
    Replace(Box<Replace>),
    /// `qreplace` asks about the match it found
    Query(Box<Replace>)
}

/// Answer that resumes a suspended command
#[derive(Debug)]
pub(crate) enum Answer
{
    /// The worker's reply
    Reply(Reply),
    /// Yes to a prompt
    Yes,
    /// No to a prompt, or Esc
    No,
    /// A typed name
    Name(String)
}

/// How a suspended command continues
#[derive(Debug)]
pub(crate) enum Then
{
    /// Install a loaded file and switch to it
    Open
    {
        name: FileName
    },
    /// Create an empty file after the user agreed
    Create
    {
        name: FileName
    },
    /// Fill `<clipboard>` from the system clipboard and switch to it
    ShowClipboard,
    /// A save finished; continue with `after`
    Saved
    {
        file: FileId, path: PathBuf, after: After
    },
    /// Ask for a new name for a read-only file
    SaveAs
    {
        file: FileId, after: After
    },
    /// Paste after reading the system clipboard
    Paste,
    /// Insert a loaded file at a position
    InsertFile
    {
        pos: Pos
    },
    /// Apply TOOLS.INI sections
    Ini
    {
        tags: Vec<String>, startup: bool
    },
    /// Reread a file after confirmation
    Reread
    {
        file: FileId
    },
    /// Discard a file after confirmation
    Discard
    {
        file: FileId
    },
    /// Quit after confirmation
    Quit,
    /// `replace` got its search string
    ReplaceFind
    {
        span: Span, regex: bool, query: bool
    },
    /// `replace` got its replacement string
    ReplaceWith
    {
        span: Span, regex: bool, query: bool, find: String
    },
    /// Look up a marker in `markfile`
    MarkFile
    {
        name: String
    }
}

/// What follows a successful save
#[derive(Debug, Clone)]
pub(crate) enum After
{
    /// Nothing; the save was the command
    Nothing,
    /// Switch to another file
    Switch(Switch),
    /// Continue `exit`
    Exit
    {
        advance: bool
    },
    /// A copy was written under another name; the file keeps its state
    Copied
}

/// Target of a file switch
#[derive(Debug, Clone)]
pub(crate) enum Switch
{
    /// An open file
    To(FileId),
    /// A file to open by name
    Open(FileName)
}

/// Text argument being typed on the dialog line
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TextArg
{
    pub(crate) chars:  Vec<char>,
    pub(crate) cursor: usize
}

/// Arg mode state
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArgState
{
    pub(crate) count:  usize,
    pub(crate) anchor: Pos,
    pub(crate) text:   Option<TextArg>
}

/// One editing change for undo
#[derive(Debug, Clone)]
pub(crate) struct Edit
{
    pub(crate) offset:   usize,
    pub(crate) removed:  Vec<u8>,
    pub(crate) inserted: Vec<u8>
}

/// One undo step: the changes of one top-level command
#[derive(Debug, Clone)]
pub(crate) struct Group
{
    pub(crate) cursor: Pos,
    pub(crate) edits:  Vec<Edit>
}

/// Per-file editor state
#[derive(Debug, Clone, Default)]
pub(crate) struct FileState
{
    pub(crate) view: View,
    pub(crate) undo: Vec<Group>
}

/// A running macro
#[derive(Debug)]
struct Frame
{
    items: Rc<[Item]>,
    pc:    usize
}

/// Work queued at startup
#[derive(Debug)]
enum Job
{
    Ini,
    Open(String),
    Execute(String)
}

/// The editor
pub struct Editor
{
    pub(crate) env:          Box<dyn Env>,
    pub(crate) exe:          String,
    pub(crate) cols:         u16,
    pub(crate) rows:         u16,
    pub(crate) files:        FileTable,
    pub(crate) current:      FileId,
    pub(crate) previous:     Option<FileId>,
    pub(crate) states:       HashMap<FileId, FileState>,
    pub(crate) file_list:    Vec<String>,
    pub(crate) next_file:    usize,
    pub(crate) keymap:       Keymap,
    pub(crate) macros:       Macros,
    pub(crate) switches:     Switches,
    pub(crate) insert:       bool,
    pub(crate) meta:         bool,
    pub(crate) arg:          Option<ArgState>,
    pub(crate) message:      Option<(String, MessageKind)>,
    pub(crate) clip:         Clip,
    pub(crate) written:      Option<ClipboardPayload>,
    pub(crate) clip_warned:  bool,
    pub(crate) search:       Option<(String, bool)>,
    pub(crate) last_find:    String,
    pub(crate) last_replace: String,
    pub(crate) marks:        HashMap<String, (FileName, Pos)>,
    pub(crate) jump_back:    Option<(FileId, View)>,
    pub(crate) before:       Option<(FileId, View)>,
    pub(crate) goto:         Option<Pos>,
    pub(crate) last_text:    Vec<char>,
    pub(crate) saved_cursor: Option<(FileId, Pos)>,
    pub(crate) result:       bool,
    pub(crate) typed:        Option<char>,
    pub(crate) last_key:     Option<Key>,
    pub(crate) undo_file:    Option<FileId>,
    pub(crate) highlight:    Option<(Pos, usize)>,
    pub(crate) pending:      Option<Pending>,
    pub(crate) function:     &'static str,
    pub(crate) redraw:       bool,
    pub(crate) slice_lines:  usize,
    frames:                  Vec<Frame>,
    jobs:                    VecDeque<Job>,
    typeahead:               VecDeque<Key>,
    requests:                Vec<Request>,
    next_id:                 RequestId,
    pub(crate) quit:         bool
}

impl Editor
{
    /// Creates the editor and queues startup: TOOLS.INI, the first file, `-e` commands
    pub fn new(env: Box<dyn Env>, startup: Startup) -> Self
    {
        let crlf = env.os() == Os::Windows;
        let mut files = FileTable::default();
        let current = files.add(File::new(FileName::Pseudo("untitled".into()), crlf));
        let mut jobs = VecDeque::new();
        if startup.tools_ini == true
        {
            jobs.push_back(Job::Ini);
        }
        jobs.extend(startup.files.first().cloned().map(Job::Open));
        jobs.extend(startup.execute.into_iter().map(Job::Execute));
        Self {
            env,
            exe: startup.exe,
            cols: startup.cols,
            rows: startup.rows,
            files,
            current,
            previous: None,
            states: HashMap::new(),
            next_file: 1,
            file_list: startup.files,
            keymap: Keymap::default(),
            macros: Macros::default(),
            switches: Switches::default(),
            insert: false,
            meta: false,
            arg: None,
            message: None,
            clip: Clip { shape: Shape::Stream, lines: vec![Vec::new()] },
            written: None,
            clip_warned: false,
            search: None,
            last_find: String::new(),
            last_replace: String::new(),
            marks: HashMap::new(),
            jump_back: None,
            before: None,
            goto: None,
            last_text: Vec::new(),
            saved_cursor: None,
            result: true,
            typed: None,
            last_key: None,
            undo_file: None,
            highlight: None,
            pending: None,
            function: "",
            redraw: false,
            slice_lines: SEARCH_SLICE_LINES,
            frames: Vec::new(),
            jobs,
            typeahead: VecDeque::new(),
            requests: Vec::new(),
            next_id: 0,
            quit: false
        }
    }

    /// Runs the startup work; call once before the first event
    pub fn start(&mut self) -> Outcome
    {
        self.pump()
    }

    /// Handles one event and runs everything it unblocks
    pub fn handle(&mut self, event: Event) -> Outcome
    {
        match event
        {
            | Event::Key(key) => self.key_event(key),
            | Event::Resize(cols, rows) =>
            {
                self.cols = cols;
                self.rows = rows;
            }
            | Event::Completed(completion) => self.completed(completion),
            | Event::Close => self.quit = true
        }
        self.pump()
    }

    /// Returns the value of the last function that ran
    pub fn result(&self) -> bool
    {
        self.result
    }

    /// Returns `true` while a command waits for I/O or an answer
    pub fn is_waiting(&self) -> bool
    {
        self.pending.is_some()
    }

    /// Returns the bytes of an open file, as saving would write them before tab and blank cleanup
    pub fn buffer(&self, name: &FileName) -> Option<Vec<u8>>
    {
        self.files.find(name).and_then(|id| self.files.get(id)).map(|file| file.text.to_bytes())
    }

    /// Returns the name of the current file
    pub fn current_name(&self) -> FileName
    {
        self.file().name.clone()
    }

    /// Returns the cursor position as (line, column), 0-based
    pub fn cursor_position(&self) -> (usize, usize)
    {
        let cursor = self.cursor();
        (cursor.line, cursor.col)
    }

    /// Returns the window position as (top line, left column), 0-based
    pub fn window_position(&self) -> (usize, usize)
    {
        let view = self.view();
        (view.top, view.left)
    }

    /// Returns the internal clipboard
    pub fn clipboard(&self) -> &Clip
    {
        &self.clip
    }

    /// Returns the dialog line message
    pub fn message(&self) -> Option<&str>
    {
        self.message.as_ref().map(|(text, _)| text.as_str())
    }

    /// Sets the number of lines a search scans per step; tests use small values
    pub fn set_slice_lines(&mut self, lines: usize)
    {
        self.slice_lines = lines.max(1);
    }

    /// Continues a long operation by one slice (plan §3.5)
    pub fn tick(&mut self) -> Outcome
    {
        let step = match self.pending.take()
        {
            | Some(Pending::Search(search)) => self.search_step(*search),
            | Some(Pending::Replace(replace)) => self.replace_step(*replace),
            | pending =>
            {
                self.pending = pending;
                return self.pump();
            }
        };
        self.run_step(step);
        self.pump()
    }

    /// Runs a function or macro by name as if a key bound to it was pressed
    pub fn press(&mut self, name: &str) -> Outcome
    {
        self.begin_command(None);
        self.call(name);
        self.pump()
    }

    /// Runs queued work until a command suspends or nothing is left
    fn pump(&mut self) -> Outcome
    {
        loop
        {
            if self.quit == true || self.pending.is_some()
            {
                break;
            }
            else if self.frames.is_empty() == false
            {
                self.step_macro();
            }
            else if let Some(job) = self.jobs.pop_front()
            {
                self.run_job(job);
            }
            else if let Some(key) = self.typeahead.pop_front()
            {
                self.begin_command(Some(key));
                let name = self.keymap.lookup(key).to_string();
                self.call(&name);
            }
            else
            {
                break;
            }
        }
        self.follow();
        let busy = matches!(self.pending, Some(Pending::Search(_) | Pending::Replace(_)));
        Outcome { quit: self.quit, requests: std::mem::take(&mut self.requests), redraw: std::mem::take(&mut self.redraw), busy }
    }

    /// Starts a new top-level command: new undo step, cleared message
    fn begin_command(&mut self, key: Option<Key>)
    {
        self.before = Some((self.current, self.view()));
        self.goto = None;
        self.undo_file = None;
        self.highlight = None;
        self.message = None;
        self.last_key = key;
        self.typed = key.and_then(key_char);
    }

    fn run_job(&mut self, job: Job)
    {
        self.begin_command(None);
        match job
        {
            | Job::Ini =>
            {
                let tags = vec![self.exe.clone(), format!("{}-{}", self.exe, self.env.os().tag())];
                let step = self.request_ini(tags, true);
                self.run_step(step);
            }
            | Job::Open(name) =>
            {
                let name = FileName::parse(&name, &self.env.cwd());
                let step = self.open(name);
                self.run_step(step);
            }
            | Job::Execute(text) =>
            {
                if let Err(error) = self.run_text(&text)
                {
                    self.result = false;
                    self.error(&error.to_string());
                }
            }
        }
    }

    /// Handles a key while a command may be waiting
    fn key_event(&mut self, key: Key)
    {
        match self.pending.take()
        {
            | None => self.typeahead.push_back(key),
            | Some(Pending::Quote) =>
            {
                self.typed = key_char(key);
                let done = self.quote_key();
                self.result = done;
            }
            | Some(Pending::Confirm { prompt, then }) => match answer_key(key)
            {
                | Some(true) => self.resume(then, Answer::Yes),
                | Some(false) => self.resume(then, Answer::No),
                | None => self.pending = Some(Pending::Confirm { prompt, then })
            },
            | Some(Pending::Prompt { prompt, mut edit, then }) => match key.code
            {
                | KeyCode::Esc => self.resume(then, Answer::No),
                | KeyCode::Enter => self.resume(then, Answer::Name(edit.chars.iter().collect())),
                | _ =>
                {
                    let arg_key = self.keymap.lookup(key) == "arg";
                    prompt_key(&mut edit, key, self.insert, arg_key);
                    self.pending = Some(Pending::Prompt { prompt, edit, then });
                }
            },
            | Some(Pending::Query(replace)) => self.query_key(*replace, key),
            | Some(pending @ (Pending::Request { .. } | Pending::Search(_) | Pending::Replace(_))) => match key.code
            {
                | KeyCode::Esc =>
                {
                    self.info("Operation cancelled");
                    self.result = false;
                }
                | _ =>
                {
                    self.typeahead.push_back(key);
                    self.pending = Some(pending);
                }
            }
        }
    }

    /// Delivers a worker completion
    fn completed(&mut self, completion: Completion)
    {
        match self.pending.take()
        {
            | Some(Pending::Request { id, then }) if id == completion.id => self.resume(then, Answer::Reply(completion.reply)),
            | pending =>
            {
                self.pending = pending;
                // Clipboard writes are not awaited; a failure is reported once (plan §6.9).
                if let Reply::ClipboardWritten(false) = completion.reply &&
                    self.clip_warned == false
                {
                    self.clip_warned = true;
                    self.error("System clipboard not available; using the internal clipboard");
                }
            }
        }
    }

    /// Continues a suspended command and records its result
    fn resume(&mut self, then: Then, answer: Answer)
    {
        let step = match then
        {
            | Then::Ini { tags, startup } => self.ini_loaded(&tags, startup, answer),
            | Then::Paste => self.paste_read(answer),
            | Then::InsertFile { pos } => self.insert_loaded(pos, answer),
            | then @ (Then::ReplaceFind { .. } | Then::ReplaceWith { .. }) => self.replace_continue(then, answer),
            | Then::MarkFile { name } => self.mark_continue(&name, answer),
            | then => self.file_continue(then, answer)
        };
        self.run_step(step);
    }

    /// Records a command's result or suspends it
    pub(crate) fn run_step(&mut self, step: Step)
    {
        match step
        {
            | Step::Done(result) => self.result = result,
            | Step::Wait(pending) => self.pending = Some(pending)
        }
    }

    /// Queues a request for a worker and returns its id
    pub(crate) fn request(&mut self, kind: RequestKind) -> RequestId
    {
        self.next_id += 1;
        self.requests.push(Request { id: self.next_id, kind });
        self.next_id
    }

    /// Runs one macro item; the macro's value is the value of the last function it ran
    // ORACLE: MACRO-1
    fn step_macro(&mut self)
    {
        let top = self.frames.len() - 1;
        let frame = &mut self.frames[top];
        let Some(item) = frame.items.get(frame.pc).cloned()
        else
        {
            self.frames.pop();
            return;
        };
        frame.pc += 1;
        match item
        {
            | Item::Call(name) => self.call(&name),
            | Item::Text(text) => self.result = self.type_text(&text),
            | Item::Label(_) =>
            {}
            | Item::Jump(jump, label) =>
            {
                let taken = match jump
                {
                    | Jump::Always => true,
                    | Jump::IfTrue => self.result,
                    | Jump::IfFalse => self.result == false
                };
                if taken == true
                {
                    self.jump(label);
                }
            }
        }
    }

    /// Jumps to a label in the current macro, or leaves the macro without one
    fn jump(&mut self, label: Option<String>)
    {
        let top = self.frames.len() - 1;
        let Some(label) = label
        else
        {
            self.frames.pop();
            return;
        };
        let target = self.frames[top].items.iter().position(|item| *item == Item::Label(label.clone()));
        match target
        {
            | Some(index) => self.frames[top].pc = index + 1,
            | None =>
            {
                self.frames.clear();
                self.result = false;
                self.error(&format!("Macro label not found: {label}"));
            }
        }
    }

    /// Enters text from a macro string: text argument input in arg mode, otherwise typed characters
    fn type_text(&mut self, text: &str) -> bool
    {
        if let Some(arg) = &mut self.arg
        {
            let edit = arg.text.get_or_insert_with(TextArg::default);
            for ch in text.chars()
            {
                edit.chars.insert(edit.cursor, ch);
                edit.cursor += 1;
            }
            return true;
        }
        text.chars().fold(true, |_, ch| self.put_char(ch))
    }

    /// Starts running macro items, e.g. `execute` text
    pub(crate) fn push_macro(&mut self, items: Vec<Item>)
    {
        self.frames.push(Frame { items: items.into(), pc: 0 });
    }

    /// Calls a function or macro by name
    pub(crate) fn call(&mut self, name: &str)
    {
        let name = name.to_lowercase();
        if let Some(items) = self.macros.get(&name)
        {
            if self.frames.len() >= MAX_MACRO_DEPTH
            {
                self.frames.clear();
                self.result = false;
                self.error("Macro nesting too deep");
            }
            else
            {
                self.frames.push(Frame { items: items.into(), pc: 0 });
            }
        }
        else if let Some(desc) = cmd::find(&name)
        {
            self.dispatch(desc);
        }
        else
        {
            self.result = false;
            self.error(&format!("{name}: unknown function"));
        }
    }

    /// Runs a built-in function with its argument (plan §6.5, §6.6)
    fn dispatch(&mut self, desc: &'static Desc)
    {
        self.function = desc.name;
        let flags = desc.flags;
        let text_active = self.arg.as_ref().is_some_and(|arg| arg.text.is_some());
        // `graphic` edits text only with a typed character; by name it is an ordinary function.
        let typing = desc.name != "graphic" || self.typed.is_some();
        let (insert, typed, cursor) = (self.insert, self.typed.unwrap_or(' '), self.cursor());
        let step = if flags & flags::TEXTEDIT != 0 &&
            typing == true &&
            let Some(edit) = self.arg.as_mut().and_then(|arg| arg.text.as_mut())
        {
            let done = edit_text(edit, desc.name, typed, insert, &self.last_text);
            if desc.name == "quote"
            {
                // The next key goes into the text literally.
                self.pending = Some(Pending::Quote);
            }
            Step::Done(done)
        }
        else if desc.name == "graphic" &&
            typing == true &&
            let Some(arg) = self.arg.as_mut()
        {
            // A typed character starts a text argument; a pending selection is dropped (CMD-ARG-2).
            arg.anchor = cursor;
            arg.text = Some(TextArg { chars: vec![typed], cursor: 1 });
            Step::Done(true)
        }
        else if flags & flags::RAW != 0 || (flags & flags::CURSORFUNC != 0 && text_active == false)
        {
            // Raw functions see the argument state; cursor functions extend a selection (plan §6.5).
            (desc.func)(self, Arg::None)
        }
        else
        {
            match self.classify(flags)
            {
                | Some(arg) =>
                {
                    self.arg = None;
                    (desc.func)(self, arg)
                }
                | None =>
                {
                    self.arg = None;
                    self.fail("Invalid argument")
                }
            }
        };
        if flags & flags::KEEPMETA == 0
        {
            self.meta = false;
        }
        self.run_step(step);
    }

    /// Classifies the pending argument against a function's flags; `None` if it does not accept it
    fn classify(&mut self, flags: u32) -> Option<Arg>
    {
        let has = |flag: u32| flags & flag != 0;
        let Some(state) = self.arg.clone()
        else
        {
            return has(flags::NOARG).then_some(Arg::None);
        };
        let (count, anchor, cursor) = (state.count, state.anchor, self.cursor());
        if let Some(text) = state.text
        {
            let text: String = text.chars.iter().collect();
            // A marker of the current file is a selection from the marker to the cursor (plan §6.5).
            if has(flags::MARKARG) &&
                let Some(mark) = self.mark_here(&text)
            {
                // ORACLE: S6
                let (top, bottom, left, right) = (mark.line.min(cursor.line), mark.line.max(cursor.line), mark.col.min(cursor.col), mark.col.max(cursor.col));
                return match has(flags::MARKBOX)
                {
                    | true => Some(Arg::Box { count, top, bottom, left, right }),
                    | false => self.selection(flags, count, mark, cursor)
                };
            }
            let number = text.parse::<usize>().ok().filter(|_| has(flags::NUMARG) && text.bytes().all(|byte| byte.is_ascii_digit()));
            return match number
            {
                | Some(n) if has(flags::COUNT) => Some(Arg::Num { count, n }),
                | Some(n) => (n > 0).then(|| Arg::Line { count, first: cursor.line, last: cursor.line + n - 1 }),
                | None if has(flags::TEXTARG) =>
                {
                    self.last_text = text.chars().collect();
                    Some(Arg::Text { count, text, pos: anchor })
                }
                | None => None
            };
        }
        if anchor == cursor
        {
            require!(has(flags::NULLARG), None);
            let line = self.shown_line(anchor.line);
            let tail = crate::cells::byte_at(&line, anchor.col);
            let rest = &line[tail..];
            return Some(
                if has(flags::NULLEOL)
                {
                    Arg::Text { count, text: String::from_utf8_lossy(rest).into_owned(), pos: anchor }
                }
                else if has(flags::NULLEOW)
                {
                    let word = rest.iter().position(|byte| *byte == b' ').map_or(rest, |end| &rest[..end]);
                    Arg::Text { count, text: String::from_utf8_lossy(word).into_owned(), pos: anchor }
                }
                else
                {
                    Arg::Null { count, pos: anchor }
                }
            );
        }
        self.selection(flags, count, anchor, cursor)
    }

    /// Classifies the selection from `anchor` to `cursor`
    fn selection(&self, flags: u32, count: usize, anchor: Pos, cursor: Pos) -> Option<Arg>
    {
        let has = |flag: u32| flags & flag != 0;
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));
        let (left, right) = (anchor.col.min(cursor.col), anchor.col.max(cursor.col));
        let stream = Arg::Stream { count, start, end };
        let block = Arg::Box { count, top: start.line, bottom: end.line, left, right };
        if anchor.line == cursor.line
        {
            if has(flags::STREAMARG)
            {
                Some(stream)
            }
            else if has(flags::BOXSTR)
            {
                let line = self.shown_line(anchor.line);
                let range = crate::cells::box_range(&line, left, right);
                Some(Arg::Text { count, text: String::from_utf8_lossy(&line[range]).into_owned(), pos: start })
            }
            else
            {
                has(flags::BOXARG).then_some(block)
            }
        }
        else if anchor.col == cursor.col
        {
            if has(flags::LINEARG)
            {
                Some(Arg::Line { count, first: start.line, last: end.line })
            }
            else
            {
                has(flags::STREAMARG).then_some(stream)
            }
        }
        else if has(flags::BOXARG)
        {
            Some(block)
        }
        else
        {
            has(flags::STREAMARG).then_some(stream)
        }
    }

    /// Takes the key after `quote` and puts its character into the text argument or the file
    fn quote_key(&mut self) -> bool
    {
        let Some(ch) = self.typed
        else
        {
            return true;
        };
        match self.arg.as_mut().and_then(|arg| arg.text.as_mut())
        {
            | Some(edit) =>
            {
                edit.chars.insert(edit.cursor, ch);
                edit.cursor += 1;
                true
            }
            | None => self.put_char(ch)
        }
    }

    /// Shows an informative message
    pub(crate) fn info(&mut self, text: &str)
    {
        self.message = Some((text.to_string(), MessageKind::Info));
    }

    /// Shows an error message
    pub(crate) fn error(&mut self, text: &str)
    {
        self.message = Some((text.to_string(), MessageKind::Error));
    }

    /// Shows an error message and returns false
    pub(crate) fn fail(&mut self, text: &str) -> Step
    {
        self.error(text);
        Step::Done(false)
    }

    /// Returns the current file
    pub(crate) fn file(&self) -> &File
    {
        self.files.get(self.current).expect("the current file is always open")
    }

    /// Returns the current file for changing
    pub(crate) fn file_mut(&mut self) -> &mut File
    {
        let current = self.current;
        self.files.get_mut(current).expect("the current file is always open")
    }

    /// Returns an open file for changing; suspended commands only refer to files that stay open
    pub(crate) fn file_at(&mut self, id: FileId) -> &mut File
    {
        self.files.get_mut(id).expect("a suspended command's file stays open")
    }

    /// Returns the state of the current file
    pub(crate) fn state(&mut self) -> &mut FileState
    {
        self.states.entry(self.current).or_default()
    }

    /// Returns the view of the current file
    pub(crate) fn view(&self) -> View
    {
        self.states.get(&self.current).map(|state| state.view).unwrap_or_default()
    }

    /// Returns the cursor position
    pub(crate) fn cursor(&self) -> Pos
    {
        self.view().cursor
    }

    /// Moves the cursor
    pub(crate) fn set_cursor(&mut self, pos: Pos)
    {
        self.state().view.cursor = pos;
    }

    /// Returns the height of the text window
    pub(crate) fn height(&self) -> usize
    {
        let terminal = usize::from(self.rows).saturating_sub(2);
        self.switches.height.map_or(terminal, |height| height.min(terminal))
    }

    /// Returns the width of the text window
    pub(crate) fn width(&self) -> usize
    {
        let terminal = usize::from(self.cols);
        self.switches.width.map_or(terminal, |width| width.min(terminal))
    }

    /// Scrolls the window so the cursor is visible and remembers where a window jump came from (`arg mark`)
    fn follow(&mut self)
    {
        let (height, width, scroll) = (self.height(), self.width(), self.switches.scroll());
        self.state().view.follow(height, width, scroll);
        let (current, top) = (self.current, self.view().top);
        if let Some((file, view)) = self.before.take() &&
            file == current &&
            view.top != top
        {
            self.jump_back = Some((file, view));
        }
    }
}

/// Edits a dialog line prompt with a key; Enter and Esc are handled by the caller
// ORACLE: RPL-1
fn prompt_key(edit: &mut TextArg, key: Key, insert: bool, arg_key: bool)
{
    let name = match key.code
    {
        | KeyCode::Bksp => "cdelete",
        | KeyCode::Del => "sdelete",
        | KeyCode::Left => "left",
        | KeyCode::Right => "right",
        | KeyCode::Home => "begline",
        | KeyCode::End => "endline",
        | _ if arg_key == true =>
        {
            edit.chars.truncate(edit.cursor);
            return;
        }
        | _ => "graphic"
    };
    let typed = key_char(key).filter(|ch| ch.is_control() == false).unwrap_or('\0');
    if name != "graphic" || typed != '\0'
    {
        edit_text(edit, name, typed, insert, &[]);
    }
}

/// Edits the text argument on the dialog line (M guide p. 19; ORACLE CMD-ARG-3)
// ORACLE: CMD-ARG-3
fn edit_text(edit: &mut TextArg, name: &str, typed: char, insert: bool, last: &[char]) -> bool
{
    let at_end = edit.cursor >= edit.chars.len();
    match name
    {
        | "graphic" =>
        {
            if insert == true || at_end == true
            {
                edit.chars.insert(edit.cursor, typed);
            }
            else
            {
                edit.chars[edit.cursor] = typed;
            }
            edit.cursor += 1;
            true
        }
        | "sdelete" =>
        {
            if at_end == false
            {
                edit.chars.remove(edit.cursor);
            }
            true
        }
        | "cdelete" | "emacscdel" | "left" if edit.cursor == 0 => false,
        | "cdelete" | "emacscdel" =>
        {
            edit.cursor -= 1;
            edit.chars.remove(edit.cursor);
            true
        }
        | "left" =>
        {
            edit.cursor -= 1;
            true
        }
        | "right" =>
        {
            // Past the end, the previous text argument supplies the next character.
            let next = match at_end
            {
                | true => last.get(edit.cursor).copied(),
                | false => Some(edit.chars[edit.cursor])
            };
            match next
            {
                | Some(ch) =>
                {
                    if at_end == true
                    {
                        edit.chars.push(ch);
                    }
                    edit.cursor += 1;
                    true
                }
                | None => false
            }
        }
        | "sinsert" =>
        {
            edit.chars.insert(edit.cursor, ' ');
            true
        }
        | "begline" =>
        {
            edit.cursor = 0;
            true
        }
        | "endline" =>
        {
            edit.cursor = edit.chars.len();
            true
        }
        | _ => true
    }
}

/// Returns the character a key types, for `graphic` and `quote` (M guide p. 48)
pub(crate) fn key_char(key: Key) -> Option<char>
{
    match key.code
    {
        | KeyCode::Char(ch) if key.alt == false && key.ctrl == false => Some(ch),
        | KeyCode::Char(ch) if key.alt == false && ('@'..='_').contains(&ch.to_ascii_uppercase()) => Some(char::from(ch.to_ascii_uppercase() as u8 & 0x1f)),
        | KeyCode::Esc => Some('\u{1b}'),
        | KeyCode::Tab => Some('\t'),
        | KeyCode::Bksp => Some('\u{8}'),
        | KeyCode::NumDigit(digit) => char::from_digit(u32::from(digit), 10),
        | KeyCode::NumPlus => Some('+'),
        | KeyCode::NumMinus => Some('-'),
        | _ => None
    }
}

/// Reads a y/n answer: `Some(true)` for y, `Some(false)` for n or Esc
fn answer_key(key: Key) -> Option<bool>
{
    match key.code
    {
        | KeyCode::Char('y' | 'Y') => Some(true),
        | KeyCode::Char('n' | 'N') | KeyCode::Esc => Some(false),
        | _ => None
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;
    use crate::env::FixedEnv;

    fn editor(execute: &[&str]) -> Editor
    {
        let startup = Startup {
            cols:      40,
            rows:      8,
            exe:       "mep".into(),
            files:     Vec::new(),
            execute:   execute.iter().map(|text| text.to_string()).collect(),
            tools_ini: false
        };
        let mut editor = Editor::new(Box::new(FixedEnv::default()), startup);
        editor.start();
        editor
    }

    #[test]
    fn test_startup_without_tools_ini_sends_no_request()
    {
        let startup = Startup { cols: 40, rows: 8, exe: "mep".into(), files: Vec::new(), execute: Vec::new(), tools_ini: false };
        let mut editor = Editor::new(Box::new(FixedEnv::default()), startup);
        assert!(editor.start().requests.is_empty());
    }

    #[test]
    fn test_startup_execute_runs_and_reports_errors()
    {
        let ed = editor(&["\"ab\""]);
        assert_eq!(ed.buffer(&FileName::Pseudo("untitled".into())).unwrap(), b"ab");
        let ed = editor(&["nosuch"]);
        assert_eq!(ed.message(), Some("nosuch: unknown function"));
        assert!(ed.result() == false);
    }

    #[test]
    fn test_resize_changes_the_window()
    {
        let mut ed = editor(&[]);
        ed.handle(Event::Resize(20, 5));
        assert_eq!((ed.width(), ed.height()), (20, 3));
        let frame = ed.render();
        assert_eq!((frame.cols(), frame.rows()), (20, 5));
    }

    #[test]
    fn test_height_and_width_switches_are_clamped_to_the_terminal()
    {
        let mut ed = editor(&[]);
        ed.switches.height = Some(3);
        ed.switches.width = Some(100);
        assert_eq!((ed.height(), ed.width()), (3, 40));
    }

    #[test]
    fn test_key_characters()
    {
        let key = |name: &str| key_char(Key::parse(name).unwrap());
        assert_eq!(key("x"), Some('x'));
        assert_eq!(key("ctrl+l"), Some('\u{c}'));
        assert_eq!(key("ctrl+["), Some('\u{1b}'));
        assert_eq!(key("ctrl+1"), None);
        assert_eq!(key("alt+x"), None);
        assert_eq!(key("esc"), Some('\u{1b}'));
        assert_eq!(key("tab"), Some('\t'));
        assert_eq!(key("bksp"), Some('\u{8}'));
        assert_eq!(key("num7"), Some('7'));
        assert_eq!(key("num+"), Some('+'));
        assert_eq!(key("num-"), Some('-'));
        assert_eq!(key("f1"), None);
    }

    #[test]
    fn test_tick_without_work_does_nothing()
    {
        let mut ed = editor(&[]);
        let outcome = ed.tick();
        assert!(outcome.busy == false && outcome.requests.is_empty());
    }

    #[test]
    fn test_unexpected_completions_are_ignored()
    {
        let mut ed = editor(&[]);
        ed.press("paste");
        assert!(ed.is_waiting());
        ed.handle(Event::Completed(Completion { id: 999, reply: Reply::ClipboardWritten(true) }));
        assert!(ed.is_waiting());
        ed.handle(Event::Completed(Completion { id: 999, reply: Reply::ClipboardWritten(false) }));
        ed.handle(Event::Completed(Completion { id: 999, reply: Reply::ClipboardWritten(false) }));
        assert_eq!(ed.message(), Some("System clipboard not available; using the internal clipboard"));
    }

    #[test]
    fn test_continuations_with_unexpected_answers_return_false()
    {
        for then in [Then::Paste, Then::InsertFile { pos: Pos::default() }, Then::Ini { tags: Vec::new(), startup: false }, Then::Quit]
        {
            let mut ed = editor(&[]);
            ed.resume(then, Answer::No);
            assert!(ed.result() == false || ed.message().is_some());
        }
    }

    #[test]
    fn test_arg_into_text()
    {
        assert_eq!(Arg::Text { count: 1, text: "t".into(), pos: Pos::default() }.into_text(), "t");
        assert_eq!(Arg::None.into_text(), "");
        assert_eq!(Arg::Null { count: 1, pos: Pos::new(1, 2) }.anchor(), Pos::new(1, 2));
        assert_eq!(Arg::None.anchor(), Pos::default());
    }
}
