//! Extensions: loading, functions, switches and the callbacks (docs/spec/extensions.md)

use anyhow::{Result, anyhow};

use crate::{
    cells,
    clip::{Clip, Shape},
    cmd::{self, Desc},
    commands::edit::{delete_box, lines_range},
    edit::text_length,
    editor::{Arg, Editor, Pending, Step, Window, key_char},
    ext::{ExtApi, ExtArg, SwitchKind, SwitchValue, flags},
    file::{File, FileId, FileName},
    io::{Reply, Request, RequestKind},
    view::Pos
};

/// Calls the extension function the dispatcher found under the current function name
pub(crate) fn call(ed: &mut Editor, arg: Arg) -> Step
{
    let id = ed.ext_functions.get(ed.function).map(|(_, id)| *id).expect("an extension function is registered under its name");
    let cursor = ed.cursor();
    let arg = match arg
    {
        | Arg::Null { count, pos } => ExtArg::Null { count, y: pos.line, x: pos.col },
        | Arg::Text { count, text, pos } => ExtArg::Text { count, y: pos.line, x: pos.col, text },
        | Arg::Line { count, first, last } => ExtArg::Line { count, start: first, end: last },
        | Arg::Stream { count, start, end } => ExtArg::Stream { count, x_start: start.col, y_start: start.line, x_end: end.col, y_end: end.line },
        | Arg::Box { count, top, bottom, left, right } => ExtArg::Box { count, left, top, right: right - 1, bottom },
        | _ => ExtArg::None { y: cursor.line, x: cursor.col }
    };
    let (key, meta, host) = (ed.typed.map_or(0, u32::from), ed.meta, ed.host.clone());
    Step::Done(host.call(id, &arg, key, meta, ed))
}

impl Editor
{
    /// Returns `true` if `name` is a built-in or extension function
    pub(crate) fn is_function(&self, name: &str) -> bool
    {
        cmd::is_function(name) || self.ext_functions.contains_key(name.to_lowercase().as_str())
    }

    /// Returns the built-in or extension function `name` (lowercase)
    pub(crate) fn function_desc(&self, name: &str) -> Option<&'static Desc>
    {
        cmd::find(name).or_else(|| self.ext_functions.get(name).map(|(desc, _)| *desc))
    }

    /// Loads an extension (`load:`), registers its functions and switches and runs its `WhenLoaded`
    // ORACLE: EXT-1, EXT-2
    pub(crate) fn load_extension(&mut self, value: &str) -> Result<()>
    {
        let name = FileName::parse(value, &self.env.cwd());
        let path = name.path().ok_or_else(|| anyhow!("{value}: cannot load extension: not a file name"))?.clone();
        require!(self.loaded.contains(&path) == false, Ok(()));
        let host = self.host.clone();
        let extension = host.load(&path).map_err(|error| anyhow!("{value}: cannot load extension: {error}"))?;
        self.loaded.push(path);
        for function in extension.functions.into_iter().filter(|function| cmd::find(&function.name.to_lowercase()).is_none())
        {
            let name: &'static str = Box::leak(function.name.to_lowercase().into_boxed_str());
            // Extensions stay loaded until the editor ends, so their table entries live as long.
            let desc: &'static Desc = Box::leak(Box::new(Desc { name, flags: function.flags & flags::ALL, func: call }));
            self.ext_functions.insert(name, (desc, function.id));
        }
        for switch in extension.switches
        {
            self.ext_switches.insert(switch.name.to_lowercase(), (switch.kind, switch.id));
        }
        host.init(extension.id, self);
        Ok(())
    }

    /// Sets an extension switch; `None` if `name` is not one
    pub(crate) fn set_ext_switch(&mut self, name: &str, value: &str) -> Option<Result<()>>
    {
        let lower = name.to_lowercase();
        let negated = lower.strip_prefix("no").and_then(|plain| self.ext_switches.get(plain)).filter(|(kind, _)| *kind == SwitchKind::Boolean);
        let (kind, id, on) = match (self.ext_switches.get(&lower), negated)
        {
            | (Some((kind, id)), _) => (*kind, *id, true),
            | (None, Some((kind, id))) => (*kind, *id, false),
            | (None, None) => return None
        };
        let value = value.trim();
        let parsed = match kind
        {
            | SwitchKind::Boolean if value.is_empty() => Ok(SwitchValue::Boolean(on)),
            | SwitchKind::Boolean => Err(anyhow!("{name}: a Boolean switch takes no value")),
            | SwitchKind::Numeric => value.parse().map(SwitchValue::Numeric).map_err(|_| anyhow!("{name}: expected a number")),
            | SwitchKind::Special => Ok(SwitchValue::Special(value.to_string()))
        };
        let host = self.host.clone();
        Some(parsed.and_then(|value| match host.set_switch(id, &value, self)
        {
            | true => Ok(()),
            | false => Err(anyhow!("{name}: value rejected by the extension"))
        }))
    }

    /// Returns the open file of an extension handle
    fn file_of(&self, handle: usize) -> Option<FileId>
    {
        FileId::from_handle(handle).filter(|id| self.files.get(*id).is_some())
    }

    /// Runs `work` with the file of `handle` as the current file; the windows stay as they were
    fn in_file(&mut self, handle: usize, work: &mut dyn FnMut(&mut Self))
    {
        if let Some(id) = self.file_of(handle)
        {
            let (current, window) = (self.current, self.windows[self.active]);
            self.current = id;
            work(self);
            self.current = current;
            self.windows[self.active] = window;
        }
    }

    /// Returns rows of the displayed lines of a file
    fn rows_of(&mut self, handle: usize, lines: std::ops::RangeInclusive<usize>) -> Vec<Vec<u8>>
    {
        let mut rows = Vec::new();
        self.in_file(handle, &mut |ed| rows = lines.clone().map(|line| ed.shown_line(line)).collect());
        rows
    }

    /// Inserts clipboard-shaped contents into a file
    fn insert_into(&mut self, handle: usize, clip: &Clip, pos: Pos)
    {
        self.in_file(handle, &mut |ed| ed.insert_clip(clip, pos));
    }

    /// Serves the request a command waits for at once, for `fExecute`
    fn serve_now(&mut self, id: crate::io::RequestId) -> bool
    {
        let host = self.host.clone();
        let completion = match self.requests.iter().position(|request| request.id == id)
        {
            | Some(index) => host.serve(self.requests.remove(index)),
            | None => self.runs.iter().position(|run| run.id == id).and_then(|index| host.run(self.runs.remove(index)))
        };
        completion.map(|completion| self.completed(completion)).is_some()
    }

    /// Takes the next key: typed ahead or from the host
    fn next_key(&mut self) -> Option<crate::keys::Key>
    {
        self.typeahead.pop_front().or_else(|| self.host.read_key())
    }
}

impl ExtApi for Editor
{
    fn add_file(&mut self, name: &str) -> usize
    {
        let name = FileName::parse(name, &self.env.cwd());
        let id = match self.files.find(&name)
        {
            | Some(id) => id,
            | None => self.files.add(File::new(name, self.platform_crlf()))
        };
        id.handle()
    }

    fn bad_arg(&mut self) -> bool
    {
        self.error("Invalid argument");
        false
    }

    fn copy_box(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2])
    {
        let [left, top, right, bottom] = from;
        let width = right + 1 - left;
        let rows = self.rows_of(src, top..=bottom).into_iter().map(|row| row[cells::box_range(&row, left, right + 1)].to_vec()).collect();
        self.insert_into(dst, &Clip { shape: Shape::Box { width }, lines: rows }, Pos::new(to[1], to[0]));
    }

    fn copy_line(&mut self, src: usize, dst: usize, start: usize, end: usize, at: usize)
    {
        let lines = match src
        {
            | 0 => vec![Vec::new()],
            | _ =>
            {
                let length = self.file_length(src);
                match start < length
                {
                    | true => self.rows_of(src, start..=end.min(length - 1)),
                    | false => return
                }
            }
        };
        self.insert_into(dst, &Clip { shape: Shape::Line, lines }, Pos::new(at, 0));
    }

    fn copy_stream(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2])
    {
        let [x_start, y_start, x_end, y_end] = from;
        let mut lines = self.rows_of(src, y_start..=y_end);
        let count = lines.len();
        // A stream that ends before it starts has no lines.
        if count == 0
        {
            return;
        }
        let end = cells::byte_at(&lines[count - 1], x_end);
        lines[count - 1].truncate(end);
        let start = cells::byte_at(&lines[0], x_start).min(lines[0].len());
        lines[0].drain(..start);
        self.insert_into(dst, &Clip { shape: Shape::Stream, lines }, Pos::new(to[1], to[0]));
    }

    fn del_box(&mut self, file: usize, area: [usize; 4])
    {
        let [left, top, right, bottom] = area;
        self.in_file(file, &mut |ed| drop(delete_box(ed, top, bottom.min(ed.last_line()), left, right + 1)));
    }

    fn del_file(&mut self, file: usize)
    {
        self.in_file(file, &mut |ed| {
            let end = ed.file().text.len();
            ed.replace(0..end, b"");
        });
    }

    fn del_line(&mut self, file: usize, start: usize, end: usize)
    {
        self.in_file(file, &mut |ed| {
            if start < ed.line_count()
            {
                let range = lines_range(ed, start, end.min(ed.last_line()));
                ed.replace(range, b"");
            }
        });
    }

    fn del_stream(&mut self, file: usize, area: [usize; 4])
    {
        let [x_start, y_start, x_end, y_end] = area;
        self.in_file(file, &mut |ed| {
            let from = ed.clamped_offset(Pos::new(y_start, x_start));
            let to = ed.clamped_offset(Pos::new(y_end, x_end));
            ed.replace(from..to.max(from), b"");
        });
    }

    fn message(&mut self, text: &str) -> usize
    {
        self.info(text);
        text.chars().count()
    }

    // ORACLE: EXT-3
    fn display(&mut self)
    {
        self.redraw = true;
    }

    /// Runs macro text to the end; waiting I/O is served at once, questions to the user fail
    ///
    /// `replace` asks before it works, so only a search can be in progress.
    fn execute(&mut self, text: &str) -> bool
    {
        let depth = self.frames.len();
        if let Err(error) = self.run_text(text)
        {
            self.error(&error.to_string());
            return false;
        }
        loop
        {
            match self.pending.take()
            {
                | Some(Pending::Request { id, then }) =>
                {
                    self.pending = Some(Pending::Request { id, then });
                    if self.serve_now(id) == false
                    {
                        self.pending = None;
                        self.result = false;
                    }
                }
                | Some(Pending::Search(search)) =>
                {
                    let step = self.search_step(*search);
                    self.run_step(step);
                }
                | Some(_) => self.result = false,
                | None if self.frames.len() > depth => self.step_macro(),
                | None => break
            }
        }
        self.result
    }

    fn file_length(&mut self, file: usize) -> usize
    {
        self.file_of(file).and_then(|id| self.files.get(id)).map_or(0, |file| text_length(&file.text))
    }

    fn file_handle(&mut self, name: &str, short: Option<&str>) -> usize
    {
        if name.is_empty() == true
        {
            return self.current.handle();
        }
        let wanted = FileName::parse(name, &self.env.cwd());
        let found = self.files.find(&wanted).or_else(|| {
            let short = short.filter(|short| short.is_empty() == false)?;
            self.files.iter().find(|(_, file)| file.name.display().ends_with(short)).map(|(id, _)| id)
        });
        found.map_or(0, FileId::handle)
    }

    fn file_read(&mut self, name: &str, file: usize) -> bool
    {
        require!(self.file_of(file).is_some(), false);
        let Some(path) = FileName::parse(name, &self.env.cwd()).path().cloned()
        else
        {
            // A pseudo-file has nothing on disk (MSJ WhenLoaded).
            return true;
        };
        let host = self.host.clone();
        let Some(Reply::Loaded(Ok(loaded))) = host.serve(Request { id: 0, kind: RequestKind::Load(path) }).map(|completion| completion.reply)
        else
        {
            return false;
        };
        self.in_file(file, &mut |ed| {
            let end = ed.file().text.len();
            ed.replace(0..end, &loaded.bytes);
        });
        true
    }

    fn file_write(&mut self, name: &str, file: usize) -> bool
    {
        let path = FileName::parse(name, &self.env.cwd()).path().cloned();
        let data = self.file_of(file).and_then(|id| self.files.get(id)).map(|file| file.text.to_bytes());
        let (Some(path), Some(data)) = (path, data)
        else
        {
            return false;
        };
        let host = self.host.clone();
        matches!(host.serve(Request { id: 0, kind: RequestKind::Save { path, data, backup: false } }).map(|completion| completion.reply), Some(Reply::Saved(Ok(()))))
    }

    fn get_cursor(&mut self) -> (usize, usize)
    {
        let cursor = self.cursor();
        (cursor.col, cursor.line)
    }

    fn get_line(&mut self, line: usize, file: usize) -> Vec<u8>
    {
        self.rows_of(file, line..=line).pop().unwrap_or_default()
    }

    fn keyboard(&mut self, hooked: bool)
    {
        self.host.keyboard(hooked);
    }

    fn move_cursor(&mut self, x: usize, y: usize)
    {
        self.set_cursor(Pos::new(y, x));
    }

    fn to_top(&mut self, file: usize)
    {
        if let Some(id) = self.file_of(file)
        {
            self.make_current(id);
        }
    }

    fn put_line(&mut self, line: usize, text: &[u8], file: usize)
    {
        self.in_file(file, &mut |ed| {
            ed.ensure_line(line);
            ed.set_line(line, text);
        });
    }

    fn remove_file(&mut self, file: usize)
    {
        match self.file_of(file)
        {
            | Some(id) if id == self.current =>
            {
                self.discard(id);
            }
            | Some(id) =>
            {
                self.files.remove(id);
                self.states.remove(&id);
                self.previous = self.previous.filter(|previous| *previous != id);
                let (current, view) = (self.current, self.view());
                for window in self.windows.iter_mut().filter(|window| window.file == id)
                {
                    *window = Window { file: current, view };
                }
            }
            | None =>
            {}
        }
    }

    fn read_char(&mut self) -> u32
    {
        self.next_key().and_then(key_char).map_or(0, u32::from)
    }

    // ORACLE: EXT-4
    fn read_command(&mut self) -> Option<String>
    {
        self.next_key().map(|key| self.keymap.lookup(key).to_string())
    }

    fn replace_char(&mut self, ch: u8, x: usize, y: usize, file: usize, insert: bool) -> bool
    {
        let mut done = false;
        self.in_file(file, &mut |ed| {
            let pos = Pos::new(y, x);
            let at = ed.prepare(pos);
            let line = ed.shown_line(y);
            let replaced = cells::glyphs(&line).into_iter().find(|glyph| glyph.col == x && insert == false).map_or(0, |glyph| glyph.bytes.len());
            ed.replace(at..at + replaced, &[ch]);
            done = true;
        });
        done
    }

    fn set_key(&mut self, function: &str, key: &str) -> bool
    {
        self.apply(&format!("{function}:{key}")).is_ok()
    }
}
