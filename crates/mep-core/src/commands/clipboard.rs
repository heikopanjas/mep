//! Clipboard functions (docs/spec/commands: copy, paste; IMPLEMENTATION_PLAN.md §3.3, §6.9)

use crate::{
    cells,
    clip::{Clip, Shape},
    editor::{Answer, Arg, Editor, Pending, Step, Then},
    file::FileName,
    io::{Reply, RequestKind},
    view::Pos
};

/// `copy` (copy.md)
pub(crate) fn copy(ed: &mut Editor, arg: Arg) -> Step
{
    let cursor = ed.cursor();
    let clip = match arg
    {
        | Arg::Null { pos, .. } =>
        {
            let shown = ed.shown_line(pos.line);
            Clip { shape: Shape::Stream, lines: vec![shown[cells::byte_at(&shown, pos.col)..].to_vec()] }
        }
        | Arg::Text { text, .. } => Clip { shape: Shape::Stream, lines: vec![text.into_bytes()] },
        | Arg::Line { first, last, .. } =>
        {
            let last = last.min(ed.last_line());
            let lines: Vec<Vec<u8>> = (first..=last).map(|line| ed.file().text.line(line)).collect();
            Clip {
                shape: Shape::Line,
                lines: if lines.is_empty()
                {
                    vec![Vec::new()]
                }
                else
                {
                    lines
                }
            }
        }
        | Arg::Stream { start, end, .. } =>
        {
            let shown = ed.shown_line(start.line);
            Clip { shape: Shape::Stream, lines: vec![shown[cells::box_range(&shown, start.col, end.col)].to_vec()] }
        }
        | Arg::Box { top, bottom, left, right, .. } =>
        {
            let rows = (top..=bottom)
                .map(|line| {
                    let shown = ed.shown_line(line);
                    let mut row = shown[cells::box_range(&shown, left, right)].to_vec();
                    let width = cells::width(&row);
                    row.extend(b" ".repeat((right - left).saturating_sub(width)));
                    row
                })
                .collect();
            Clip { shape: Shape::Box { width: right - left }, lines: rows }
        }
        | _ => Clip { shape: Shape::Line, lines: vec![ed.file().text.line(cursor.line)] }
    };
    ed.set_clip(clip);
    Step::Done(true)
}

/// `paste` (paste.md)
// ORACLE: S5, CMD-PASTE-1, CMD-PASTE-2, CMD-PASTE-3
pub(crate) fn paste(ed: &mut Editor, arg: Arg) -> Step
{
    match arg
    {
        | Arg::Text { count, text, pos } if count >= 2 && text.starts_with('!') => match text[1..].trim()
        {
            | "" => ed.fail("Invalid argument"),
            | command =>
            {
                let id = ed.start_run(command.to_string(), true);
                Step::Wait(Pending::Request { id, then: Then::InsertOutput { pos } })
            }
        },
        | Arg::Text { count, text, pos } if count >= 2 =>
        {
            let name = FileName::parse(&text, &ed.env.cwd());
            match name.path()
            {
                | Some(path) =>
                {
                    let id = ed.request(RequestKind::Load(path.clone()));
                    Step::Wait(Pending::Request { id, then: Then::InsertFile { pos } })
                }
                | None => ed.fail(&format!("{text}: cannot insert a pseudo-file"))
            }
        }
        | Arg::Text { text, pos, .. } =>
        {
            let clip = Clip { shape: Shape::Stream, lines: vec![text.into_bytes()] };
            ed.set_clip(clip.clone());
            ed.insert_clip(&clip, pos);
            Step::Done(true)
        }
        | _ =>
        {
            let id = ed.request(RequestKind::ClipboardRead);
            Step::Wait(Pending::Request { id, then: Then::Paste })
        }
    }
}

impl Editor
{
    /// Takes the system clipboard after a read and pastes it at the cursor
    pub(crate) fn paste_read(&mut self, answer: Answer) -> Step
    {
        let payload = match answer
        {
            | Answer::Reply(Reply::ClipboardRead(payload)) => payload,
            | _ => None
        };
        match payload
        {
            // Text MEP wrote itself is pasted from the byte-exact internal clipboard.
            | Some(payload) if self.written.as_ref() != Some(&payload) => self.clip = Clip::from_payload(&payload),
            | Some(_) =>
            {}
            | None => self.warn_clipboard()
        }
        let clip = self.clip.clone();
        let cursor = self.cursor();
        self.insert_clip(&clip, cursor);
        Step::Done(true)
    }

    /// Reports once that the system clipboard is unavailable (plan §6.9)
    pub(crate) fn warn_clipboard(&mut self)
    {
        if self.clip_warned == false
        {
            self.clip_warned = true;
            self.error("System clipboard not available; using the internal clipboard");
        }
    }

    /// Inserts clipboard contents at `pos` according to their shape; the cursor does not move
    pub(crate) fn insert_clip(&mut self, clip: &Clip, pos: Pos)
    {
        match clip.shape
        {
            | Shape::Line =>
            {
                self.ensure_start(pos.line);
                let at = self.line_start(pos.line);
                let newline = self.newline();
                let bytes: Vec<u8> = clip.lines.iter().flat_map(|line| line.iter().chain(newline.iter()).copied()).collect();
                self.replace(at..at, &bytes);
            }
            | Shape::Stream =>
            {
                let at = self.prepare(pos);
                let bytes = clip.lines.join(self.newline());
                self.replace(at..at, &bytes);
            }
            | Shape::Box { width } =>
            {
                for (index, row) in clip.lines.iter().enumerate()
                {
                    let at = self.prepare(Pos::new(pos.line + index, pos.col));
                    let mut padded = row.clone();
                    padded.extend(b" ".repeat(width.saturating_sub(cells::width(row))));
                    self.replace(at..at, &padded);
                }
            }
        }
        self.set_cursor(pos);
    }

    /// Inserts the output of a `!` command at `pos`; the result is the command's success
    pub(crate) fn insert_output(&mut self, pos: Pos, answer: Answer) -> Step
    {
        match answer
        {
            | Answer::Reply(Reply::Ran(Ok(ran))) =>
            {
                let clip = crate::commands::edit::stream_clip(&ran.output);
                self.insert_clip(&clip, pos);
                Step::Done(ran.success)
            }
            | Answer::Reply(Reply::Ran(Err(error))) => self.fail(&error.to_string()),
            | _ => Step::Done(false)
        }
    }

    /// Inserts a loaded file at `pos` for `arg arg name paste`
    pub(crate) fn insert_loaded(&mut self, pos: Pos, answer: Answer) -> Step
    {
        match answer
        {
            | Answer::Reply(Reply::Loaded(Ok(loaded))) =>
            {
                let clip = crate::commands::edit::stream_clip(&loaded.bytes);
                self.insert_clip(&clip, pos);
                Step::Done(true)
            }
            | Answer::Reply(Reply::Loaded(Err(error))) => self.fail(&error.to_string()),
            | _ => Step::Done(false)
        }
    }
}
