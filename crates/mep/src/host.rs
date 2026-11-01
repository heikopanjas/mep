//! The services the core waits for: the terminal for `shell`, I/O at once and extensions (IMPLEMENTATION_PLAN.md §8 M4)

use std::{
    any::Any,
    cell::RefCell,
    collections::VecDeque,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex, PoisonError, mpsc::Receiver}
};

use mep_core::{
    Event,
    clip::SystemClipboard,
    env::Os,
    ext::{ExtApi, ExtArg, Extension, SwitchValue},
    host::Host,
    io::{Completion, Fs, Request, serve},
    keys::Key,
    run::{Run, Runner, execute}
};
use mep_ext::Extensions;
use mep_term::{InputGate, SharedOut};

use crate::process::shell_command;

/// The system clipboard, shared by the I/O worker and the host
pub type SharedClipboard = Arc<Mutex<Box<dyn SystemClipboard + Send>>>;

/// The editor's events; an extension reading keys sets the other events aside for the main loop
pub struct Inbox
{
    incoming: Receiver<Event>,
    deferred: RefCell<VecDeque<Event>>
}

impl Inbox
{
    /// Wraps the event channel
    pub fn new(incoming: Receiver<Event>) -> Self
    {
        Self { incoming, deferred: RefCell::default() }
    }

    /// Returns the next event, set-aside ones first; with `wait`, waits for one
    pub fn next(&self, wait: bool) -> Option<Event>
    {
        let deferred = self.deferred.borrow_mut().pop_front();
        deferred.or_else(|| match wait
        {
            | true => self.incoming.recv().ok(),
            | false => self.incoming.try_recv().ok()
        })
    }

    /// Waits for a key; `None` when the terminal closed
    pub fn key(&self) -> Option<Key>
    {
        loop
        {
            match self.incoming.recv()
            {
                | Ok(Event::Key(key)) => return Some(key),
                | Ok(Event::Close) =>
                {
                    self.deferred.borrow_mut().push_back(Event::Close);
                    return None;
                }
                | Ok(other) => self.deferred.borrow_mut().push_back(other),
                | Err(_) => return None
            }
        }
    }
}

/// Hands the terminal to programs, serves I/O at once and runs extensions
pub struct TerminalHost
{
    /// Leaves full-screen mode
    pub suspend:     Rc<dyn Fn() -> io::Result<()>>,
    /// Enters full-screen mode again
    pub resume:      Rc<dyn Fn() -> io::Result<()>>,
    /// Waits until Enter is pressed
    pub wait_enter:  Box<dyn Fn() -> io::Result<()>>,
    /// Keeps interrupt signals away from the editor while the returned guard lives
    pub quiet:       Box<dyn Fn() -> Box<dyn Any>>,
    /// Stops the input thread
    pub gate:        Arc<InputGate>,
    /// The terminal output, held while programs run so no frame is drawn
    pub out:         SharedOut,
    /// Operating system, for the shell command line
    pub os:          Os,
    /// The interactive shell
    pub interactive: String,
    /// Working directory
    pub cwd:         PathBuf,
    /// The file system, for extension callbacks
    pub fs:          Arc<dyn Fs + Send + Sync>,
    /// The system clipboard, for extension callbacks
    pub clipboard:   SharedClipboard,
    /// Runs command lines, for extension callbacks
    pub runner:      Arc<dyn Runner + Send + Sync>,
    /// The editor's events, for `ReadChar` and `ReadCmd`
    pub inbox:       Rc<Inbox>,
    /// The loaded extensions
    pub extensions:  Extensions
}

impl Host for TerminalHost
{
    fn shell(&self, commands: &[String], wait: bool) -> bool
    {
        self.gate.pause();
        let mut out = self.out.lock();
        let _ = (self.suspend)();
        let quiet = (self.quiet)();
        let mut runs: Vec<Command> = commands.iter().map(|line| shell_command(self.os, line)).collect();
        if runs.is_empty() == true
        {
            runs.push(Command::new(&self.interactive));
        }
        // Every command runs, even after one failed.
        let results: Vec<bool> = runs.iter_mut().map(|command| command.current_dir(&self.cwd).status().is_ok_and(|status| status.success())).collect();
        drop(quiet);
        if wait == true
        {
            let _ = write!(out, "Press Enter to return to the editor").and_then(|()| out.flush());
            let _ = (self.wait_enter)();
        }
        let _ = (self.resume)();
        drop(out);
        self.gate.resume();
        results.into_iter().all(|success| success)
    }

    fn serve(&self, request: Request) -> Option<Completion>
    {
        let mut clipboard = self.clipboard.lock().unwrap_or_else(PoisonError::into_inner);
        Some(serve(request, self.fs.as_ref(), clipboard.as_mut()))
    }

    fn run(&self, run: Run) -> Option<Completion>
    {
        Some(execute(run, self.runner.as_ref()))
    }

    fn read_key(&self) -> Option<Key>
    {
        self.inbox.key()
    }

    fn keyboard(&self, hooked: bool)
    {
        match hooked
        {
            | true => self.gate.resume(),
            | false => self.gate.pause()
        }
    }

    fn load(&self, path: &Path) -> Result<Extension, String>
    {
        self.extensions.load(path)
    }

    fn init(&self, library: usize, api: &mut dyn ExtApi)
    {
        self.extensions.init(library, api);
    }

    fn call(&self, id: usize, arg: &ExtArg, key: u32, meta: bool, api: &mut dyn ExtApi) -> bool
    {
        self.extensions.call(id, arg, key, meta, api)
    }

    fn set_switch(&self, id: usize, value: &SwitchValue, api: &mut dyn ExtApi) -> bool
    {
        self.extensions.set_switch(id, value, api)
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::sync::mpsc;

    use mep_core::{
        clip::MemClipboard,
        io::{Reply, RequestKind},
        keys::KeyCode,
        memfs::MemFs
    };

    use super::*;
    use crate::process::OsRunner;

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared
    {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>
        {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()>
        {
            Ok(())
        }
    }

    fn host(calls: &Rc<RefCell<Vec<&'static str>>>, out: &Shared, events: Receiver<Event>) -> TerminalHost
    {
        let log = |name: &'static str| {
            let calls = calls.clone();
            move || {
                calls.borrow_mut().push(name);
                Ok(())
            }
        };
        TerminalHost {
            suspend:     Rc::new(log("suspend")),
            resume:      Rc::new(log("resume")),
            wait_enter:  Box::new(log("enter")),
            quiet:       Box::new(|| Box::new(())),
            gate:        Arc::new(InputGate::default()),
            out:         SharedOut::new(Box::new(out.clone())),
            os:          Os::Linux,
            interactive: "true".into(),
            cwd:         std::env::temp_dir(),
            fs:          Arc::new(MemFs::default().with_file("/a.txt", b"a")),
            clipboard:   Arc::new(Mutex::new(Box::new(MemClipboard::default()))),
            runner:      Arc::new(OsRunner(Os::Linux)),
            inbox:       Rc::new(Inbox::new(events)),
            extensions:  Extensions::default()
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_shell_runs_commands_and_waits()
    {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let out = Shared::default();
        let (_events, incoming) = mpsc::channel();
        let host = host(&calls, &out, incoming);
        assert!(host.shell(&["exit 0".into(), "true".into()], true));
        assert_eq!(*calls.borrow(), ["suspend", "enter", "resume"]);
        assert_eq!(out.0.lock().unwrap().as_slice(), b"Press Enter to return to the editor");
        assert!(host.shell(&["exit 1".into(), "true".into()], false) == false);
        assert!(host.shell(&[], false));
        let missing = TerminalHost { interactive: "/no/such/shell".into(), ..host };
        assert!(missing.shell(&[], false) == false);
    }

    #[test]
    fn test_serves_requests_and_reads_keys()
    {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let (events, incoming) = mpsc::channel();
        let host = host(&calls, &Shared::default(), incoming);
        let loaded = host.serve(Request { id: 4, kind: RequestKind::Load("/a.txt".into()) }).unwrap();
        assert!(matches!(loaded.reply, Reply::Loaded(Ok(ref file)) if file.bytes == b"a"));
        let ran = host.run(Run { id: 5, command: "exit 0".into(), cwd: std::env::temp_dir(), stdout_only: true }).unwrap();
        assert_eq!(ran.id, 5);
        let key = Key::plain(KeyCode::Char('k'));
        events.send(Event::Resize(1, 2)).unwrap();
        events.send(Event::Key(key)).unwrap();
        assert_eq!(host.read_key(), Some(key));
        events.send(Event::Close).unwrap();
        assert_eq!(host.read_key(), None);
        assert!(matches!(host.inbox.next(false), Some(Event::Resize(1, 2))));
        assert!(matches!(host.inbox.next(true), Some(Event::Close)));
        assert!(host.inbox.next(false).is_none());
        drop(events);
        assert_eq!(host.read_key(), None);
        assert!(host.inbox.next(true).is_none());
        host.keyboard(false);
        host.keyboard(true);
        assert!(host.load(Path::new("/no/such/library.so")).is_err());
    }
}
