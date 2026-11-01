//! Argument, control and text insertion functions (docs/spec/commands: arg, meta, cancel,
//! lasttext, unassigned, assign, execute, initialize, cur*)

use anyhow::anyhow;

use crate::{
    Result,
    editor::{Answer, Arg, ArgState, Editor, Pending, Step, TextArg, Then},
    file::FileName,
    ini,
    io::{Reply, RequestKind},
    keys::Key,
    macros::{self, Item},
    switches::Switches,
    view::Pos
};

/// `arg`: starts arg mode or raises the count; clears the text to its end while typing (arg.md)
pub(crate) fn arg(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    match &mut ed.arg
    {
        | None => ed.arg = Some(ArgState { count: 1, anchor: cursor, text: None }),
        | Some(ArgState { text: Some(text), .. }) => text.chars.truncate(text.cursor),
        // ORACLE: CMD-ARG-1, CMD-ARG-2
        | Some(state) => state.count += 1
    }
    Step::Done(true)
}

/// `meta`: toggles meta and returns the new state (meta.md)
pub(crate) fn meta(ed: &mut Editor, _: Arg) -> Step
{
    ed.meta = ed.meta == false;
    Step::Done(ed.meta)
}

/// `cancel`: ends arg mode (cancel.md)
// ORACLE: CMD-CANCEL-1
pub(crate) fn cancel(ed: &mut Editor, _: Arg) -> Step
{
    if ed.arg.take().is_some()
    {
        ed.info("Argument cancelled");
    }
    Step::Done(true)
}

/// `lasttext`: recalls the previous text argument for editing (lasttext.md)
// ORACLE: CMD-LASTTEXT-1
pub(crate) fn lasttext(ed: &mut Editor, _: Arg) -> Step
{
    let cursor = ed.cursor();
    let text = TextArg { chars: ed.last_text.clone(), cursor: ed.last_text.len() };
    match &mut ed.arg
    {
        | None => ed.arg = Some(ArgState { count: 1, anchor: cursor, text: Some(text) }),
        | Some(state) if state.text.is_some() || state.anchor == cursor => state.text = Some(text),
        | Some(_) =>
        {
            ed.arg = None;
            return ed.fail("Invalid argument");
        }
    }
    Step::Done(true)
}

/// `unassigned`: reports the key; the argument stays pending (unassigned.md)
// ORACLE: CMD-UNASSIGNED-1
pub(crate) fn unassigned(ed: &mut Editor, _: Arg) -> Step
{
    let key = ed.last_key.map_or_else(|| "unassigned".to_string(), |key| key.to_string());
    ed.fail(&format!("{key} is not assigned to any editor function"))
}

/// Functions of later milestones (plan §6.7)
pub(crate) fn not_available(ed: &mut Editor, _: Arg) -> Step
{
    ed.arg = None;
    ed.fail(&format!("{}: not available in this version", ed.function))
}

/// `assign`: applies TOOLS.INI statements (assign.md)
// ORACLE: CMD-ASSIGN-1
pub(crate) fn assign(ed: &mut Editor, arg: Arg) -> Step
{
    let statements: Vec<String> = match arg
    {
        | Arg::Text { text, .. } if text == "?" => return ed.fail("assign ?: not available in this version"),
        | Arg::Line { first, last, .. } => (first..=last).map(|line| String::from_utf8_lossy(&ed.shown_line(line)).into_owned()).collect(),
        | Arg::Box { top, bottom, left, right, .. } => (top..=bottom)
            .map(|line| {
                let shown = ed.shown_line(line);
                String::from_utf8_lossy(&shown[crate::cells::box_range(&shown, left, right)]).trim_end().to_string()
            })
            .collect(),
        | other => vec![other.into_text()]
    };
    let errors: Vec<String> = statements
        .iter()
        .filter(|statement| statement.trim().is_empty() == false)
        .filter_map(|statement| ed.apply(statement.trim()).err())
        .map(|error| error.to_string())
        .collect();
    match errors.first()
    {
        | Some(error) => ed.fail(error),
        | None => Step::Done(true)
    }
}

/// `execute`: runs text as a macro sequence (execute.md)
// ORACLE: CMD-EXECUTE-1
pub(crate) fn execute(ed: &mut Editor, arg: Arg) -> Step
{
    let text = match arg
    {
        | Arg::Line { first, last, .. } => (first..=last).map(|line| String::from_utf8_lossy(&ed.shown_line(line)).into_owned()).collect::<Vec<_>>().join(" "),
        | Arg::Stream { start, end, .. } =>
        {
            let range = ed.clamped_offset(start)..ed.clamped_offset(end);
            String::from_utf8_lossy(&ed.bytes(range)).replace(['\r', '\n'], " ")
        }
        | other => other.into_text()
    };
    match ed.run_text(&text)
    {
        | Ok(()) => Step::Done(true),
        | Err(error) => ed.fail(&error.to_string())
    }
}

/// `initialize`: rereads TOOLS.INI sections on top of the current state (initialize.md)
// ORACLE: CMD-INITIALIZE-1, CMD-INITIALIZE-2
pub(crate) fn initialize(ed: &mut Editor, arg: Arg) -> Step
{
    let os_tag = format!("{}-{}", ed.exe, ed.env.os().tag());
    let tags = match arg
    {
        | Arg::Text { text, .. } if text.is_empty() => return ed.fail("Invalid argument"),
        | Arg::Text { text, .. } => vec![ed.exe.clone(), format!("{}-{text}", ed.exe)],
        | _ => vec![ed.exe.clone(), os_tag]
    };
    ed.request_ini(tags, false)
}

/// Inserts text at the cursor and moves the cursor after it (CMD-CURTEXT-1)
// ORACLE: CMD-CURTEXT-1
fn insert_text(ed: &mut Editor, text: &str) -> Step
{
    let cursor = ed.cursor();
    let at = ed.prepare(cursor);
    ed.replace(at..at, text.as_bytes());
    ed.set_cursor(Pos::new(cursor.line, cursor.col + crate::cells::width(text.as_bytes())));
    Step::Done(true)
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// `curdate`: inserts the date as `Jun-27-1987`
// ORACLE: CMD-CURDATE-1
pub(crate) fn curdate(ed: &mut Editor, _: Arg) -> Step
{
    let now = ed.env.now();
    let month = MONTHS[usize::from(now.month.clamp(1, 12)) - 1];
    insert_text(ed, &format!("{month}-{:02}-{:04}", now.day, now.year))
}

/// `curday`: inserts the day of the week
pub(crate) fn curday(ed: &mut Editor, _: Arg) -> Step
{
    let day = DAYS[usize::from(ed.env.now().weekday % 7)];
    insert_text(ed, day)
}

/// `curtime`: inserts the time as `13:45:55`
pub(crate) fn curtime(ed: &mut Editor, _: Arg) -> Step
{
    let now = ed.env.now();
    insert_text(ed, &format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second))
}

/// `curuser`: inserts MAILNAME, else USER (USERNAME on Windows)
pub(crate) fn curuser(ed: &mut Editor, _: Arg) -> Step
{
    let fallback = match ed.env.os()
    {
        | crate::env::Os::Windows => "USERNAME",
        | _ => "USER"
    };
    let name =
        ["MAILNAME", fallback].into_iter().filter_map(|var| ed.env.var(var)).map(|value| value.to_string_lossy().into_owned()).find(|value| value.is_empty() == false);
    match name
    {
        | Some(name) => insert_text(ed, &name),
        | None => ed.fail("User name not set")
    }
}

/// Returns the name of the current file for the `curfile*` functions; `None` for the untitled buffer
fn file_name(ed: &Editor) -> Option<FileName>
{
    let name = ed.file().name.clone();
    (name != FileName::Pseudo("untitled".into())).then_some(name)
}

/// `curfile`: inserts the full path of the current file
pub(crate) fn curfile(ed: &mut Editor, _: Arg) -> Step
{
    match file_name(ed)
    {
        | Some(name) => insert_text(ed, &name.display()),
        | None => Step::Done(false)
    }
}

/// Splits a file name into base name and extension with its dot
fn split_name(name: &FileName) -> (String, String)
{
    match name.path().and_then(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned())
    {
        | Some(file) => match file.rfind('.')
        {
            | Some(dot) if dot > 0 => (file[..dot].to_string(), file[dot..].to_string()),
            | _ => (file, String::new())
        },
        | None => (name.display(), String::new())
    }
}

/// `curfilenam`: inserts the base name of the current file
pub(crate) fn curfilenam(ed: &mut Editor, _: Arg) -> Step
{
    match file_name(ed)
    {
        | Some(name) => insert_text(ed, &split_name(&name).0),
        | None => Step::Done(false)
    }
}

/// `curfileext`: inserts the extension of the current file with its dot
// ORACLE: CMD-CURFILEEXT-1
pub(crate) fn curfileext(ed: &mut Editor, _: Arg) -> Step
{
    match file_name(ed)
    {
        | Some(name) => insert_text(ed, &split_name(&name).1),
        | None => Step::Done(false)
    }
}

impl Editor
{
    /// Applies one TOOLS.INI statement: key assignment, macro definition or switch (plan §6.12)
    pub(crate) fn apply(&mut self, statement: &str) -> Result<()>
    {
        let (name, value) = statement.split_once(':').ok_or_else(|| anyhow!("{statement}: missing ':'"))?;
        let name = name.trim();
        if let Some(body) = value.strip_prefix('=')
        {
            return self.macros.define(name, body);
        }
        if Switches::is_switch(name) == true
        {
            self.switches.set(name, value)?;
            return match name.eq_ignore_ascii_case("load")
            {
                | true => self.load_extension(value.trim()),
                | false => Ok(())
            };
        }
        if let Some(result) = self.set_ext_switch(name, value)
        {
            return result;
        }
        let known = self.is_function(name) || self.macros.get(name).is_some();
        require!(known, Err(anyhow!("{name}: unknown function, macro or switch")));
        let key = Key::parse(value.trim())?;
        self.keymap.assign(key, &name.to_lowercase());
        Ok(())
    }

    /// Parses macro text and checks function names and labels before anything runs (execute.md)
    pub(crate) fn run_text(&mut self, text: &str) -> Result<()>
    {
        let items = macros::parse(text)?;
        for item in &items
        {
            match item
            {
                | Item::Call(name) if self.is_function(name) == false && self.macros.get(name).is_none() => return Err(anyhow!("{name}: unknown function")),
                | Item::Jump(_, Some(label)) if items.contains(&Item::Label(label.clone())) == false => return Err(anyhow!("Macro label not found: {label}")),
                | _ =>
                {}
            }
        }
        self.push_macro(items);
        Ok(())
    }

    /// Asks a worker for TOOLS.INI and suspends until it arrives
    pub(crate) fn request_ini(&mut self, tags: Vec<String>, startup: bool) -> Step
    {
        let dirs = ini::search_dirs(self.env.as_ref());
        let id = self.request(RequestKind::FindIni(dirs));
        Step::Wait(Pending::Request { id, then: Then::Ini { tags, startup } })
    }

    /// Applies the TOOLS.INI sections once the file is read
    pub(crate) fn ini_loaded(&mut self, tags: &[String], startup: bool, answer: Answer) -> Step
    {
        let Answer::Reply(Reply::Ini(result)) = answer
        else
        {
            return Step::Done(false);
        };
        let step = match result
        {
            | Ok(Some((path, text))) =>
            {
                let mut errors = Vec::new();
                for tag in tags
                {
                    let statements = ini::section(&text, tag);
                    // The editor's own section may be empty; a named text section must exist.
                    if statements.is_empty() && startup == false && tag != &self.exe && tag.ends_with(self.env.os().tag()) == false
                    {
                        errors.push(format!("Section [{tag}] not found"));
                    }
                    for statement in statements
                    {
                        if let Err(error) = self.apply(&statement.text)
                        {
                            errors.push(format!("{} line {}: {error}", path.display(), statement.line));
                        }
                    }
                }
                match errors.as_slice()
                {
                    | [] => Step::Done(true),
                    | [first] => self.fail(first),
                    | [first, rest @ ..] => self.fail(&format!("{first} (and {} more errors)", rest.len()))
                }
            }
            | Ok(None) if startup == true => Step::Done(true),
            | Ok(None) => self.fail("TOOLS.INI not found"),
            | Err(error) => self.fail(&format!("TOOLS.INI: {error}"))
        };
        if startup == true
        {
            self.insert = self.switches.enterinsmode;
        }
        step
    }
}
