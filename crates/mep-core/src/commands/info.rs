//! The information file and the `information` command (docs/spec/commands/information.md; M guide p. 38, p. 97)

use typed_path::Utf8TypedPathBuf;

use crate::{
    edit::text_length,
    editor::{Answer, Arg, Editor, Pending, Step, Switch, Then},
    file::{File, FileId, FileName},
    history::{self, Entry},
    ini,
    io::{Reply, RequestKind},
    text::Text
};

/// `information`: lists the files in memory and the remembered files (information.md)
// ORACLE: INFO-1
pub(crate) fn information(ed: &mut Editor, _: Arg) -> Step
{
    let name = FileName::Pseudo("information".into());
    let mut text = String::new();
    for (_, file) in ed.files.iter().filter(|(_, file)| file.name != name)
    {
        text.push_str(&format!("{}  Length={}\n", file.name.display(), text_length(&file.text)));
    }
    for entry in ed.history.iter().filter(|entry| ed.files.find(&FileName::Disk(entry.path.clone())).is_none())
    {
        text.push_str(&format!("{}\n", entry.path));
    }
    let id = ed.fill_pseudo("information", text.into_bytes());
    ed.switch(Switch::To(id))
}

impl Editor
{
    /// Replaces the contents of a pseudo-file, creating it if needed
    pub(crate) fn fill_pseudo(&mut self, name: &str, bytes: Vec<u8>) -> FileId
    {
        let name = FileName::Pseudo(name.into());
        let id = match self.files.find(&name)
        {
            | Some(id) => id,
            | None => self.files.add(File::new(name, self.platform_crlf()))
        };
        let file = self.file_at(id);
        file.text = Text::from(bytes);
        file.modified = false;
        let state = self.state_of(id);
        state.undo.clear();
        state.redo.clear();
        id
    }

    /// Returns the information file: `<config dir>/<exe>.tmp`
    fn history_path(&self) -> Option<Utf8TypedPathBuf>
    {
        ini::config_dir(self.env.as_ref()).map(|dir| dir.join(format!("{}.tmp", self.exe)))
    }

    /// Reads the information file at startup
    pub(crate) fn load_history(&mut self) -> Step
    {
        let Some(path) = self.history_path()
        else
        {
            return Step::Done(true);
        };
        let id = self.request(RequestKind::Load(path));
        Step::Wait(Pending::Request { id, then: Then::History })
    }

    /// Keeps the remembered files; without file arguments, reopens the first one
    pub(crate) fn history_loaded(&mut self, answer: Answer) -> Step
    {
        if let Answer::Reply(Reply::Loaded(Ok(loaded))) = answer
        {
            self.history = history::parse(&String::from_utf8_lossy(&loaded.bytes), self.env.os());
        }
        match (self.file_list.is_empty(), self.history.first())
        {
            | (true, Some(entry)) =>
            {
                let name = FileName::Disk(entry.path.clone());
                let id = self.request(RequestKind::Load(entry.path.clone()));
                Step::Wait(Pending::Request { id, then: Then::Reopen { name } })
            }
            | _ => Step::Done(true)
        }
    }

    /// Opens the last edited file; a file that is gone leaves the untitled buffer
    pub(crate) fn reopen(&mut self, name: FileName, answer: Answer) -> Step
    {
        match answer
        {
            | Answer::Reply(Reply::Loaded(Ok(loaded))) => self.install(File::loaded(name, loaded)),
            | _ => Step::Done(true)
        }
    }

    /// Gives a newly opened file the position remembered in the information file, if it still lies in the file
    pub(crate) fn remember(&mut self, id: FileId, name: &FileName)
    {
        let lines = self.files.get(id).map_or(0, |file| file.text.line_count());
        let found = self.history.iter().find(|entry| Some(&entry.path) == name.path()).map(|entry| entry.view);
        if let Some(view) = found &&
            view.cursor.line < lines
        {
            self.state_of(id).view = view;
        }
    }

    /// Quits and writes the information file unless the session is temporary or `tmpsav` is 0
    pub(crate) fn leave(&mut self)
    {
        self.quit = true;
        let wanted = self.temporary == false && self.switches.tmpsav > 0;
        let Some(path) = self.history_path().filter(|_| wanted)
        else
        {
            return;
        };
        let current = self.current;
        let mut open: Vec<_> = self.files.iter().filter_map(|(id, file)| file.name.path().map(|path| (id, path.clone()))).collect();
        // A stable sort keeps the other files in order behind the current one.
        open.sort_by_key(|(id, _)| *id != current);
        let recent = open.into_iter().map(|(id, path)| Entry { path, view: self.shown_view(id) }).collect();
        let entries = history::merge(recent, &self.history, self.switches.tmpsav);
        self.request(RequestKind::WriteInfo { path, data: history::format(&entries).into_bytes() });
    }

    /// Returns a file's cursor and window position: the current window's, another window's, or the remembered one
    fn shown_view(&self, id: FileId) -> crate::view::View
    {
        match id == self.current
        {
            | true => self.view(),
            | false => self
                .windows
                .iter()
                .find(|window| window.file == id)
                .map(|window| window.view)
                .unwrap_or_else(|| self.states.get(&id).map(|state| state.view).unwrap_or_default())
        }
    }
}
