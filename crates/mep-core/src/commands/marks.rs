//! File markers (docs/spec/commands: mark; M guide §4.3 p. 31–32, `markfile` p. 63)

use crate::{
    editor::{Answer, Arg, Editor, Pending, Step, Switch, Then},
    file::{FileName, resolve},
    io::{Reply, RequestKind},
    view::Pos
};

/// `mark` (mark.md)
// ORACLE: S3, MARK-1
pub(crate) fn mark(ed: &mut Editor, arg: Arg) -> Step
{
    match arg
    {
        | Arg::None =>
        {
            ed.set_cursor(Pos::default());
            Step::Done(true)
        }
        | Arg::Null { .. } => match ed.jump_back
        {
            | Some((file, view)) if file == ed.current =>
            {
                *ed.view_mut() = view;
                Step::Done(true)
            }
            | _ => Step::Done(false)
        },
        | Arg::Num { n, .. } =>
        {
            ed.set_cursor(Pos::new(n, 0));
            Step::Done(true)
        }
        | other if other.count() >= 2 =>
        {
            let pos = other.anchor();
            let name = ed.file().name.clone();
            ed.marks.insert(other.into_text().to_lowercase(), (name, pos));
            Step::Done(true)
        }
        | other => ed.goto_mark(&other.into_text())
    }
}

impl Editor
{
    /// Returns the position of a marker defined in the current file, for mark arguments (plan §6.5)
    pub(crate) fn mark_here(&self, name: &str) -> Option<Pos>
    {
        self.marks.get(&name.to_lowercase()).filter(|(file, _)| *file == self.file().name).map(|(_, pos)| *pos)
    }

    /// Moves to a marker, looking it up in `markfile` if it is not defined
    fn goto_mark(&mut self, name: &str) -> Step
    {
        if let Some((file, pos)) = self.marks.get(&name.to_lowercase()).cloned()
        {
            return self.jump_to(file, pos);
        }
        match FileName::parse(&self.switches.markfile, &self.env.cwd())
        {
            | FileName::Disk(path) if self.switches.markfile.is_empty() == false =>
            {
                let id = self.request(RequestKind::Load(path));
                Step::Wait(Pending::Request { id, then: Then::MarkFile { name: name.to_string() } })
            }
            | _ => self.fail(&format!("{name}: mark not found"))
        }
    }

    /// Puts the cursor at a position in a file, switching files if needed
    pub(crate) fn jump_to(&mut self, file: FileName, pos: Pos) -> Step
    {
        if file == self.file().name
        {
            self.set_cursor(pos);
            return Step::Done(true);
        }
        self.goto = Some(pos);
        self.switch(Switch::Open(file))
    }

    /// Looks a marker up in the loaded `markfile`: lines `name file line column` (ORACLE MARK-2)
    // ORACLE: MARK-2
    pub(crate) fn mark_continue(&mut self, name: &str, answer: Answer) -> Step
    {
        let markfile = FileName::parse(&self.switches.markfile, &self.env.cwd());
        let dir = markfile.path().and_then(|path| path.parent()).map(|dir| dir.to_path_buf()).unwrap_or(self.env.cwd());
        let text = match answer
        {
            | Answer::Reply(Reply::Loaded(Ok(loaded))) => String::from_utf8_lossy(&loaded.bytes).into_owned(),
            | _ => String::new()
        };
        let found = text.lines().find_map(|line| match line.split_whitespace().collect::<Vec<_>>().as_slice()
        {
            | [mark, file, line, col] if mark.eq_ignore_ascii_case(name) =>
                line.parse().ok().zip(col.parse().ok()).map(|(line, col)| (file.to_string(), Pos::new(line, col))),
            | _ => None
        });
        match found
        {
            | Some((file, pos)) => self.jump_to(FileName::Disk(resolve(&dir, &file)), pos),
            | None => self.fail(&format!("{name}: mark not found"))
        }
    }
}
