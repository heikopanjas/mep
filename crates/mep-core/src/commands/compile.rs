//! Compiling and compile messages (docs/spec/commands: compile, argcompile; M guide §4.5 pp. 35–37)
//!
//! A compile runs in the background. Its output goes into `<compile>`; the
//! lines that name a file and a row are the messages `compile` steps through.

use std::{io, path::Path, sync::LazyLock};

use regex::Regex;

use crate::{
    editor::{After, Arg, Editor, Step},
    file::FileName,
    run::Ran,
    view::Pos
};

/// A compile message: where and what
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message
{
    pub(crate) file: FileName,
    pub(crate) pos:  Pos,
    pub(crate) text: String
}

/// The messages of the last compile and the one shown last
#[derive(Debug, Clone, Default)]
pub(crate) struct Messages
{
    pub(crate) list:    Vec<Message>,
    /// Index of the message shown last
    pub(crate) current: Option<usize>,
    /// Highest index shown so far; `Arg Meta Compile` goes back at most `maxmsg` from it
    pub(crate) newest:  usize
}

/// Message forms, tried in order (p. 36 plus the gcc form; ORACLE COMPILE-1)
static FORMS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r#"^"(?P<f>[^"]+)"\s*,\s*(?P<r>\d+)\s+(?P<c>\d+)\s*:\s*(?P<m>.*)$"#, r"^(?P<f>.+?)\s*\(\s*(?P<r>\d+)\s*,\s*(?P<c>\d+)\s*\)\s*:\s*(?P<m>.*)$",
        r"^(?P<f>.+?)\s*\(\s*(?P<r>\d+)\s*\)\s*:\s*(?P<m>.*)$", r"^(?P<f>.+?):(?P<r>\d+):(?P<c>\d+):\s*(?P<m>.*)$", r"^(?P<f>.+?)\s*:\s*(?P<r>\d+)\s*:\s*(?P<m>.*)$",
        r"^(?P<f>\S+)\s+(?P<r>\d+)\s+(?P<c>\d+)\s*:\s*(?P<m>.*)$"
    ]
    .iter()
    .map(|form| Regex::new(form).expect("the message forms are valid"))
    .collect()
});

/// Reads the messages from compiler output; relative names are relative to `cwd`
// ORACLE: COMPILE-1
pub(crate) fn parse(output: &[u8], cwd: &Path) -> Vec<Message>
{
    let text = String::from_utf8_lossy(output);
    text.lines()
        .filter_map(|line| {
            let captures = FORMS.iter().find_map(|form| form.captures(line.trim_end()))?;
            let number = |name: &str| captures.name(name).and_then(|value| value.as_str().parse::<usize>().ok()).unwrap_or(1).saturating_sub(1);
            Some(Message { file: FileName::parse(&captures["f"], cwd), pos: Pos::new(number("r"), number("c")), text: captures["m"].to_string() })
        })
        .collect()
}

/// `compile` (compile.md)
// ORACLE: COMPILE-2, COMPILE-3, COMPILE-4
pub(crate) fn compile(ed: &mut Editor, arg: Arg) -> Step
{
    match (arg, ed.meta)
    {
        | (Arg::None, false) => ed.next_message(false),
        | (Arg::None, true) => ed.next_message(true),
        | (Arg::Null { .. }, true) => ed.previous_message(),
        | (Arg::Null { .. }, false) => ed.compile_current(),
        | (Arg::Text { count, text, .. }, _) if count >= 2 => match text.trim().is_empty()
        {
            | true => ed.fail("Invalid argument"),
            | false => ed.start_compile(text)
        },
        | (other, _) =>
        {
            let text = other.into_text();
            let extension = Path::new(&text).extension().map(|extension| extension.to_string_lossy().to_lowercase()).unwrap_or_default();
            let found = ed.switches.extmake.get(&extension).or_else(|| ed.switches.extmake.get("text")).cloned();
            match found
            {
                | Some(command) => ed.start_compile(command.replace("%s", &text)),
                | None => ed.fail(&format!("No extmake command for .{extension}"))
            }
        }
    }
}

/// `argcompile` (argcompile.md)
pub(crate) fn argcompile(ed: &mut Editor, _: Arg) -> Step
{
    ed.compile_current()
}

impl Editor
{
    /// Compiles the current file with its `extmake` command, saving it first if it is modified
    fn compile_current(&mut self) -> Step
    {
        let file = self.file();
        let Some(path) = file.name.path().map(Path::to_path_buf)
        else
        {
            return self.fail("No file name");
        };
        let extension = path.extension().map(|extension| extension.to_string_lossy().to_lowercase()).unwrap_or_default();
        let Some(command) = self.switches.extmake.get(&extension).map(|command| command.replace("%s", &path.display().to_string()))
        else
        {
            return self.fail(&format!("No extmake command for .{extension}"));
        };
        match self.file().modified
        {
            | true => self.save(self.current, After::Compile(command)),
            | false => self.start_compile(command)
        }
    }

    /// Starts a compile in the background
    pub(crate) fn start_compile(&mut self, command: String) -> Step
    {
        let id = self.start_run(command.clone(), false);
        self.compiles.push(id);
        self.info(&format!("Compiling: {command}"));
        Step::Done(true)
    }

    /// Takes the output of a finished compile: fills `<compile>`, reads the messages and goes to the first
    pub(crate) fn compile_done(&mut self, result: io::Result<Ran>)
    {
        let (success, output) = match result
        {
            | Ok(ran) => (ran.success, ran.output),
            | Err(error) => (false, error.to_string().into_bytes())
        };
        self.bell = true;
        self.fill_pseudo("compile", output.clone());
        self.messages = Messages { list: parse(&output, &self.env.cwd()), current: None, newest: 0 };
        let count = self.messages.list.len();
        if count > 0 && self.pending.is_none()
        {
            let step = self.show_message(0);
            self.run_step(step);
        }
        else if count > 0
        {
            self.info(&format!("{count} compile messages"));
        }
        else if success == true
        {
            self.info("Compile finished");
        }
        else
        {
            self.error("Compile failed (see <compile>)");
        }
    }

    /// Goes to the next message; with `other_file`, to the next one for another file
    fn next_message(&mut self, other_file: bool) -> Step
    {
        let start = self.messages.current.map_or(0, |current| current + 1);
        let here = self.file().name.clone();
        let found = (start..self.messages.list.len()).find(|index| other_file == false || self.messages.list[*index].file != here);
        match found
        {
            | Some(index) => self.show_message(index),
            | None => self.fail("No more compile messages")
        }
    }

    /// Goes back one message, at most `maxmsg` behind the newest one shown
    fn previous_message(&mut self) -> Step
    {
        let previous = self.messages.current.and_then(|current| current.checked_sub(1)).filter(|previous| self.messages.newest - previous <= self.switches.maxmsg);
        match previous
        {
            | Some(index) => self.show_message(index),
            | None => self.fail("No previous compile message")
        }
    }

    /// Shows a message and puts the cursor at its place
    fn show_message(&mut self, index: usize) -> Step
    {
        let message = self.messages.list[index].clone();
        self.messages.current = Some(index);
        self.messages.newest = self.messages.newest.max(index);
        self.info(&message.text);
        self.jump_to(message.file, message.pos)
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    fn one(line: &str) -> Option<(String, usize, usize, String)>
    {
        parse(line.as_bytes(), Path::new("/w")).first().map(|message| (message.file.display(), message.pos.line, message.pos.col, message.text.clone()))
    }

    #[test]
    fn test_message_forms()
    {
        let want = |file: &str, row, col, text: &str| Some((file.to_string(), row, col, text.to_string()));
        assert_eq!(one(r#""a.c", 12 5: syntax error"#), want("/w/a.c", 11, 4, "syntax error"));
        assert_eq!(one("a.c(12, 5): syntax error"), want("/w/a.c", 11, 4, "syntax error"));
        assert_eq!(one("a.c (12) : warning C4013"), want("/w/a.c", 11, 0, "warning C4013"));
        assert_eq!(one("src/a.c:12:5: error: x"), want("/w/src/a.c", 11, 4, "error: x"));
        assert_eq!(one("a.c: 12: error"), want("/w/a.c", 11, 0, "error"));
        assert_eq!(one("a.c:3:int main(void)"), want("/w/a.c", 2, 0, "int main(void)"));
        assert_eq!(one("a.c 12 5: syntax error"), want("/w/a.c", 11, 4, "syntax error"));
        assert_eq!(one("/abs/b.c:1:1: x"), want("/abs/b.c", 0, 0, "x"));
        assert_eq!(one("make: *** [all] Error 1"), None);
        assert_eq!(one("a.c:0: zero row"), want("/w/a.c", 0, 0, "zero row"));
    }
}
