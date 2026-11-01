//! The core thread is never blocked by file or clipboard I/O (IMPLEMENTATION_PLAN.md §3.5, §9.6)
//!
//! Runs the real front end with a file system and a clipboard that block
//! until the test releases them. While they block, Esc must still cancel the
//! waiting command and the screen must show it.
//! `cargo test --release -p mep --test perf -- --ignored perf_`

#![feature(coverage_attribute)]
#![coverage(off)]
#![allow(clippy::bool_comparison)]

use std::{
    ffi::OsString,
    io::{self, Write},
    rc::Rc,
    sync::{Arc, Condvar, Mutex, mpsc::Sender},
    thread,
    time::{Duration, Instant}
};

use mep::{Platform, process::OsRunner, run};
use mep_core::{
    Event, Utf8TypedPathBuf,
    clip::{ClipboardPayload, SystemClipboard},
    env::{FixedEnv, Os},
    io::{FileMeta, Fs},
    keys::Key,
    memfs::MemFs
};
use mep_term::{InputGate, SharedOut};

/// A latch the test opens to let a blocked call return
#[derive(Default)]
struct Latch
{
    open:    Mutex<bool>,
    changed: Condvar
}

impl Latch
{
    fn wait(&self)
    {
        let open = self.open.lock().unwrap();
        drop(self.changed.wait_while(open, |open| *open == false).unwrap());
    }

    fn release(&self)
    {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

/// A file system whose reads block until released
struct BlockingFs
{
    inner: MemFs,
    latch: Arc<Latch>
}

impl Fs for BlockingFs
{
    fn read(&self, path: &Utf8TypedPathBuf) -> io::Result<Vec<u8>>
    {
        self.latch.wait();
        self.inner.read(path)
    }

    fn write_atomic(&self, path: &Utf8TypedPathBuf, data: &[u8]) -> io::Result<()>
    {
        self.inner.write_atomic(path, data)
    }

    fn rename(&self, from: &Utf8TypedPathBuf, to: &Utf8TypedPathBuf) -> io::Result<()>
    {
        self.inner.rename(from, to)
    }

    fn metadata(&self, path: &Utf8TypedPathBuf) -> io::Result<FileMeta>
    {
        self.inner.metadata(path)
    }

    fn read_dir_names(&self, dir: &Utf8TypedPathBuf) -> io::Result<Vec<OsString>>
    {
        self.inner.read_dir_names(dir)
    }

    fn create_dir_all(&self, dir: &Utf8TypedPathBuf) -> io::Result<()>
    {
        self.inner.create_dir_all(dir)
    }
}

/// A clipboard whose reads block until released
struct BlockingClipboard(Arc<Latch>);

impl SystemClipboard for BlockingClipboard
{
    fn read(&mut self) -> Option<ClipboardPayload>
    {
        self.0.wait();
        None
    }

    fn write(&mut self, _: &ClipboardPayload) -> bool
    {
        true
    }
}

/// The terminal output, readable while the editor runs
#[derive(Clone, Default)]
struct Screen(Arc<Mutex<Vec<u8>>>);

impl Write for Screen
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

impl Screen
{
    fn len(&self) -> usize
    {
        self.0.lock().unwrap().len()
    }

    /// Waits until `text` appears after byte `from`; returns how long that took
    fn wait_for(&self, text: &str, from: usize) -> Option<Duration>
    {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5)
        {
            if String::from_utf8_lossy(&self.0.lock().unwrap()[from..]).contains(text)
            {
                return Some(start.elapsed());
            }
            thread::sleep(Duration::from_millis(5));
        }
        None
    }
}

#[test]
#[ignore = "performance test: run with --release"]
fn perf_core_is_not_blocked_by_io()
{
    let (file, clipboard) = (Arc::new(Latch::default()), Arc::new(Latch::default()));
    let screen = Screen::default();
    let platform = Platform {
        env:        Box::new(FixedEnv::default()),
        fs:         Arc::new(BlockingFs { inner: MemFs::default().with_file("/work/big.txt", b"text\n"), latch: file.clone() }),
        clipboard:  Arc::new(Mutex::new(Box::new(BlockingClipboard(clipboard.clone())))),
        runner:     Arc::new(OsRunner(Os::Linux)),
        out:        SharedOut::new(Box::new(screen.clone())),
        size:       (60, 6),
        enter:      Rc::new(|| Ok(())),
        leave:      Rc::new(|| Ok(())),
        wait_enter: Box::new(|| Ok(())),
        quiet:      Box::new(|| Box::new(())),
        gate:       Arc::new(InputGate::default())
    };
    let results = Arc::new(Mutex::new(Vec::new()));
    let (watcher, seen) = (screen.clone(), results.clone());
    let input: Box<dyn FnOnce(Sender<Event>)> = Box::new(move |events| {
        thread::spawn(move || {
            let send = |name: &str| events.send(Event::Key(Key::parse(name).unwrap())).unwrap();
            // The load of big.txt hangs in the worker; Esc still reaches the core.
            let mark = watcher.len();
            send("esc");
            seen.lock().unwrap().push(("load", watcher.wait_for("Operation cancelled", mark)));
            file.release();
            // The clipboard read for paste hangs; Esc still reaches the core.
            send("x");
            let mark = watcher.len();
            send("shift+ins");
            send("esc");
            seen.lock().unwrap().push(("clipboard", watcher.wait_for("Operation cancelled", mark)));
            clipboard.release();
            send("f9");
            send("f8");
        });
    });
    // Without TOOLS.INI the load is the first thing the editor waits for.
    let args = ["mep", "-d", "big.txt"].map(OsString::from).to_vec();
    let code = run(args, platform, input, &mut Vec::new(), &mut Vec::new());
    assert_eq!(code, 0);
    for (what, took) in results.lock().unwrap().iter()
    {
        eprintln!("cancel while the {what} blocks: {took:?}");
        assert!(took.is_some_and(|took| took < Duration::from_millis(100)), "the core did not answer while the {what} blocked");
    }
}
