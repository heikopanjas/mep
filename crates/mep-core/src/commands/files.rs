//! File functions (docs/spec/commands: setfile, refresh, exit; IMPLEMENTATION_PLAN.md §6.10)

use std::io::ErrorKind;

use crate::{
    clip::Clip,
    edit::clip_text,
    editor::{After, Answer, Arg, Editor, Pending, Step, Switch, Then, Window},
    file::{File, FileId, FileName},
    io::{Reply, RequestKind},
    switches::Backup,
    text::Text
};

/// `setfile` (setfile.md)
// ORACLE: CMD-SETFILE-1, CMD-SETFILE-2
pub(crate) fn setfile(ed: &mut Editor, arg: Arg) -> Step
{
    let cwd = ed.env.cwd();
    match arg
    {
        | Arg::Null { count, .. } if count >= 2 => ed.save(ed.current, After::Nothing),
        | Arg::Text { count, text, .. } if count >= 2 => match FileName::parse(&text, &cwd)
        {
            | FileName::Disk(path) => ed.write_copy(path),
            | FileName::Pseudo(_) => ed.fail(&format!("{text}: cannot save to a pseudo-file"))
        },
        | Arg::Null { pos, .. } =>
        {
            let shown = ed.shown_line(pos.line);
            let rest = &shown[crate::cells::byte_at(&shown, pos.col)..];
            let word = String::from_utf8_lossy(rest.split(|byte| *byte == b' ').next().unwrap_or_default()).into_owned();
            match word.is_empty()
            {
                | true => ed.fail("Invalid argument"),
                | false => ed.switch(Switch::Open(FileName::parse(&word, &cwd)))
            }
        }
        | Arg::Text { text, .. } if text.contains(['*', '?']) => ed.fail("Wildcard file lists are not available in this version"),
        | Arg::Text { text, .. } => ed.switch(Switch::Open(FileName::parse(&text, &cwd))),
        | _ => match ed.previous
        {
            | Some(previous) => ed.switch(Switch::To(previous)),
            | None => ed.fail("No previous file")
        }
    }
}

/// `refresh` (refresh.md)
pub(crate) fn refresh(ed: &mut Editor, arg: Arg) -> Step
{
    let file = ed.current;
    let name = ed.file().name.display();
    let (prompt, then) = match arg
    {
        | Arg::Null { .. } => (format!("Discard {name}? (y/n)"), Then::Discard { file }),
        | _ => (format!("Reread {name}? (y/n)"), Then::Reread { file })
    };
    Step::Wait(Pending::Confirm { prompt, then })
}

/// `exit` (exit.md)
// ORACLE: CMD-EXIT-1
pub(crate) fn exit(ed: &mut Editor, arg: Arg) -> Step
{
    let advance = matches!(arg, Arg::None);
    let file = ed.file();
    if ed.meta == false && file.modified == true
    {
        match file.name
        {
            | FileName::Disk(_) => return ed.save(ed.current, After::Exit { advance }),
            | FileName::Pseudo(ref name) if name == "untitled" => return ed.fail("No file name: use arg arg name setfile, or meta exit"),
            | FileName::Pseudo(_) =>
            {}
        }
    }
    ed.exit_continue(advance)
}

impl Editor
{
    /// Switches files, saving the current one first per `autosave` unless meta is set
    pub(crate) fn switch(&mut self, target: Switch) -> Step
    {
        let file = self.file();
        if self.meta == false && self.switches.autosave == true && file.modified == true && file.name.path().is_some()
        {
            return self.save(self.current, After::Switch(target));
        }
        self.switch_now(target)
    }

    /// Switches without saving
    fn switch_now(&mut self, target: Switch) -> Step
    {
        match target
        {
            | Switch::To(id) =>
            {
                self.make_current(id);
                Step::Done(true)
            }
            | Switch::Open(name) => self.open(name)
        }
    }

    /// Makes a file current; an empty untitled start buffer is dropped instead of becoming the previous file
    pub(crate) fn make_current(&mut self, id: FileId)
    {
        let old = self.current;
        if old == id
        {
            return;
        }
        let file = self.file();
        // An untitled buffer that was never changed is empty and not worth keeping.
        let scratch = file.name == FileName::Pseudo("untitled".into()) && file.modified == false && self.shown(old) == 1;
        self.state().view = self.view();
        if scratch == true
        {
            self.files.remove(old);
            self.states.remove(&old);
        }
        else
        {
            self.previous = Some(old);
        }
        self.current = id;
        self.windows[self.active].file = id;
        *self.view_mut() = self.state().view;
        // A jump to a marker in another file lands on the marker.
        if let Some(pos) = self.goto.take()
        {
            self.set_cursor(pos);
        }
    }

    /// Opens a file by name: switches if it is open, otherwise loads it (setfile.md)
    pub(crate) fn open(&mut self, name: FileName) -> Step
    {
        if name == FileName::Pseudo("clipboard".into())
        {
            let id = self.request(RequestKind::ClipboardRead);
            return Step::Wait(Pending::Request { id, then: Then::ShowClipboard });
        }
        if let Some(id) = self.files.find(&name)
        {
            self.make_current(id);
            return Step::Done(true);
        }
        match name.path()
        {
            | Some(path) =>
            {
                let id = self.request(RequestKind::Load(path.to_path_buf()));
                Step::Wait(Pending::Request { id, then: Then::Open { name } })
            }
            | None => self.install(File::new(name, self.platform_crlf()))
        }
    }

    /// Returns `true` if new files use CRLF on this system
    pub(crate) fn platform_crlf(&self) -> bool
    {
        self.platform_newline() == "\r\n"
    }

    /// Adds a file and makes it current
    pub(crate) fn install(&mut self, file: File) -> Step
    {
        let (readonly, name) = (file.readonly, file.name.clone());
        let id = self.files.add(file);
        self.remember(id, &name);
        self.make_current(id);
        if readonly == true
        {
            self.info("File is read-only");
        }
        Step::Done(true)
    }

    /// Saves a file under its own name, then continues with `after`
    pub(crate) fn save(&mut self, file: FileId, after: After) -> Step
    {
        let Some(path) = self.files.get(file).and_then(|file| file.name.path()).map(|path| path.to_path_buf())
        else
        {
            return self.fail("No file name: use arg arg name setfile");
        };
        let data = self.files.get(file).map(|file| file.text.save_bytes(self.switches.entab_mode(), self.switches.trailspace)).unwrap_or_default();
        let id = self.request(RequestKind::Save { path: path.clone(), data, backup: self.switches.backup == Backup::Bak });
        Step::Wait(Pending::Request { id, then: Then::Saved { file, path, after } })
    }

    /// Writes the current file's contents to another file; the current file is unchanged
    fn write_copy(&mut self, path: std::path::PathBuf) -> Step
    {
        let data = self.file().text.save_bytes(self.switches.entab_mode(), self.switches.trailspace);
        let id = self.request(RequestKind::Save { path: path.clone(), data, backup: self.switches.backup == Backup::Bak });
        Step::Wait(Pending::Request { id, then: Then::Saved { file: self.current, path, after: After::Copied } })
    }

    /// Continues `exit` after saving: advance to the next file or quit
    pub(crate) fn exit_continue(&mut self, advance: bool) -> Step
    {
        if advance == true && self.next_file < self.file_list.len()
        {
            let name = FileName::parse(&self.file_list[self.next_file], &self.env.cwd());
            self.next_file += 1;
            return self.open(name);
        }
        let current = self.current;
        let others = self.files.iter().filter(|(id, file)| *id != current && file.modified == true).count();
        if self.switches.askexit == true || others > 0
        {
            let prompt = match others
            {
                | 0 => "Exit the editor? (y/n)".to_string(),
                | count => format!("{count} other modified files will be lost. Exit? (y/n)")
            };
            return Step::Wait(Pending::Confirm { prompt, then: Then::Quit });
        }
        self.leave();
        Step::Done(true)
    }

    /// Continues a suspended file command
    pub(crate) fn file_continue(&mut self, then: Then, answer: Answer) -> Step
    {
        match (then, answer)
        {
            | (Then::Open { name }, Answer::Reply(Reply::Loaded(Ok(loaded)))) => self.install(File::loaded(name, loaded)),
            | (Then::Open { name }, Answer::Reply(Reply::Loaded(Err(error)))) if error.kind() == ErrorKind::NotFound =>
            {
                let prompt = format!("{} does not exist. Create? (y/n)", name.display());
                Step::Wait(Pending::Confirm { prompt, then: Then::Create { name } })
            }
            | (Then::Open { name }, Answer::Reply(Reply::Loaded(Err(error)))) => self.fail(&format!("{}: {error}", name.display())),
            | (Then::Create { name }, Answer::Yes) => self.install(File::new(name, self.platform_crlf())),
            | (Then::ShowClipboard, answer) => self.show_clipboard(answer),
            | (Then::Saved { file, path, after }, Answer::Reply(Reply::Saved(Ok(())))) => self.saved(file, &path, after),
            | (Then::Saved { file, after, .. }, Answer::Reply(Reply::Saved(Err(error)))) if error.kind() == ErrorKind::PermissionDenied =>
                match self.switches.readonly.is_empty()
                {
                    | true =>
                        Step::Wait(Pending::Prompt { prompt: "File is read-only. Save as: ".into(), edit: Default::default(), then: Then::SaveAs { file, after } }),
                    | false => self.fail("readonly command: not available in this version")
                },
            | (Then::Saved { path, .. }, Answer::Reply(Reply::Saved(Err(error)))) => self.fail(&format!("{}: {error}", path.display())),
            | (Then::SaveAs { file, after }, Answer::Name(name)) =>
            {
                let name = FileName::parse(&name, &self.env.cwd());
                let target = self.file_at(file);
                target.name = name;
                target.readonly = false;
                self.save(file, after)
            }
            | (Then::Reread { file }, Answer::Yes) => self.reread(file),
            | (Then::Reread { file }, Answer::Reply(Reply::Loaded(Ok(loaded)))) =>
            {
                let target = self.file_at(file);
                target.text = Text::from(loaded.bytes);
                target.modified = false;
                target.readonly = loaded.readonly;
                self.states.entry(file).or_default().undo.clear();
                Step::Done(true)
            }
            | (Then::Reread { .. }, Answer::Reply(Reply::Loaded(Err(error)))) => self.fail(&error.to_string()),
            | (Then::Discard { file }, Answer::Yes) => self.discard(file),
            | (Then::Quit, Answer::Yes) =>
            {
                self.leave();
                Step::Done(true)
            }
            | _ => Step::Done(false)
        }
    }

    /// Finishes a save: clears the modified state for the file's own name and continues
    fn saved(&mut self, file: FileId, path: &std::path::Path, after: After) -> Step
    {
        if let After::Copied = after
        {
            return Step::Done(true);
        }
        let saved = self.file_at(file);
        saved.modified = false;
        saved.text.clear_edited();
        self.info(&format!("Saved {}", path.display()));
        match after
        {
            | After::Switch(target) => self.switch_now(target),
            | After::Exit { advance } => self.exit_continue(advance),
            | After::Compile(command) => self.start_compile(command),
            | After::Shell => self.run_shell(&[]),
            | _ => Step::Done(true)
        }
    }

    /// Rereads a file from disk after confirmation
    fn reread(&mut self, file: FileId) -> Step
    {
        let name = self.files.get(file).map(|file| file.name.clone());
        match name
        {
            | Some(FileName::Disk(path)) =>
            {
                let id = self.request(RequestKind::Load(path));
                Step::Wait(Pending::Request { id, then: Then::Reread { file } })
            }
            | Some(FileName::Pseudo(name)) if name == "clipboard" => self.open(FileName::Pseudo(name)),
            | _ => self.fail("No file on disk to reread")
        }
    }

    /// Discards a file and switches to the previous one or a new untitled buffer
    pub(crate) fn discard(&mut self, file: FileId) -> Step
    {
        let next = match self.previous
        {
            | Some(id) => id,
            | None => self.files.add(File::new(FileName::Pseudo("untitled".into()), self.platform_crlf()))
        };
        self.files.remove(file);
        self.states.remove(&file);
        self.current = next;
        self.previous = None;
        let view = self.state().view;
        for window in self.windows.iter_mut().filter(|window| window.file == file)
        {
            *window = Window { file: next, view };
        }
        Step::Done(true)
    }

    /// Fills `<clipboard>` with the system clipboard and switches to it
    fn show_clipboard(&mut self, answer: Answer) -> Step
    {
        match answer
        {
            | Answer::Reply(Reply::ClipboardRead(Some(payload))) if self.written.as_ref() != Some(&payload) => self.clip = Clip::from_payload(&payload),
            | Answer::Reply(Reply::ClipboardRead(None)) => self.warn_clipboard(),
            | _ =>
            {}
        }
        let name = FileName::Pseudo("clipboard".into());
        let text = Text::from(clip_text(&self.clip));
        let id = match self.files.find(&name)
        {
            | Some(id) => id,
            | None => self.files.add(File::new(name, false))
        };
        let file = self.file_at(id);
        file.text = text;
        file.modified = false;
        self.make_current(id);
        Step::Done(true)
    }
}
