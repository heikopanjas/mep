//! Binary wiring

#![cfg_attr(test, feature(coverage_attribute))]
#![allow(clippy::bool_comparison)]

/// Early-return guard macro for precondition checks
///
/// Returns the given expression when the condition is false.
#[macro_export]
macro_rules! require {
    ($cond:expr, $ret:expr) => {
        if ($cond) == false
        {
            return $ret;
        }
    };
}

pub mod cli;
pub mod os;
pub mod tz;

use std::{
    ffi::OsString,
    io::Write,
    sync::mpsc::{self, Receiver, Sender},
    thread
};

use mep_core::{
    Editor, Event, Startup,
    clip::SystemClipboard,
    env::Env,
    frame::Frame,
    io::{Fs, Request, serve}
};
use mep_term::Screen;

/// The operating system pieces the editor runs on
pub struct Platform
{
    /// Environment and clock
    pub env:       Box<dyn Env>,
    /// File system, used by the I/O worker
    pub fs:        Box<dyn Fs + Send>,
    /// System clipboard, used by the I/O worker
    pub clipboard: Box<dyn SystemClipboard + Send>,
    /// Terminal output, used by the render thread
    pub screen:    Screen,
    /// Terminal size as (columns, rows)
    pub size:      (u16, u16),
    /// Switches the terminal into editing mode; called only after the command line parsed
    pub enter:     Box<dyn FnOnce() -> std::io::Result<()>>,
    /// Restores the terminal
    pub leave:     Box<dyn FnOnce() -> std::io::Result<()>>
}

/// Serves requests until the editor closes the request channel (plan §3.5)
fn work(requests: Receiver<Request>, events: Sender<Event>, fs: Box<dyn Fs + Send>, mut clipboard: Box<dyn SystemClipboard + Send>)
{
    for request in requests
    {
        let _ = events.send(Event::Completed(serve(request, fs.as_ref(), clipboard.as_mut())));
    }
}

/// Draws the newest frame until the editor closes the frame channel
///
/// A write error ends drawing and tells the editor to close, so it does not wait for input forever.
fn render(frames: Receiver<(Frame, bool)>, mut screen: Screen, events: Sender<Event>) -> std::io::Result<()>
{
    while let Ok((mut frame, mut redraw)) = frames.recv()
    {
        // A slow terminal only gets the newest frame.
        for (newer, newer_redraw) in frames.try_iter()
        {
            frame = newer;
            redraw |= newer_redraw;
        }
        screen.draw(&frame, redraw).inspect_err(|_| {
            let _ = events.send(Event::Close);
        })?;
    }
    Ok(())
}

/// Runs the editor and returns the process exit code
///
/// `args` includes the program name. `input` receives the event sender and
/// starts whatever feeds keys and resizes into it.
pub fn run(args: Vec<OsString>, platform: Platform, input: Box<dyn FnOnce(Sender<Event>)>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32
{
    let exe = cli::exe_name(args.first().map(OsString::as_os_str));
    let options = match cli::parse(args)
    {
        | Ok(options) => options,
        | Err(exit) =>
        {
            let _ = match exit.stdout
            {
                | true => stdout.write_all(exit.text.as_bytes()),
                | false => stderr.write_all(exit.text.as_bytes())
            };
            return exit.code;
        }
    };
    if let Err(error) = (platform.enter)()
    {
        let _ = writeln!(stderr, "mep: cannot use the terminal: {error}");
        return 1;
    }
    let files = options.files.iter().map(|path| path.to_string_lossy().into_owned()).collect();
    let startup = Startup { cols: platform.size.0, rows: platform.size.1, exe, files, execute: options.execute, tools_ini: options.no_tools_ini == false };
    let (events, incoming) = mpsc::channel();
    let (requests, worker_requests) = mpsc::channel();
    let (frames, render_frames) = mpsc::channel();
    let (worker_events, render_events) = (events.clone(), events.clone());
    let worker = thread::spawn(move || work(worker_requests, worker_events, platform.fs, platform.clipboard));
    let renderer = thread::spawn(move || render(render_frames, platform.screen, render_events));
    input(events);
    let mut editor = Editor::new(platform.env, startup);
    let mut outcome = editor.start();
    loop
    {
        for request in outcome.requests.drain(..)
        {
            let _ = requests.send(request);
        }
        if outcome.quit == true
        {
            break;
        }
        // A render thread that failed has already sent Event::Close, which ends the editor.
        let _ = frames.send((editor.render(), outcome.redraw));
        // While a long operation runs, input is polled and the next slice runs when none waits.
        let event = match outcome.busy
        {
            | true => incoming.try_recv().ok(),
            | false => incoming.recv().ok()
        };
        outcome = match event
        {
            | Some(event) => editor.handle(event),
            | None => editor.tick()
        };
    }
    drop(frames);
    drop(requests);
    let _ = worker.join();
    let rendered = renderer.join().unwrap_or(Ok(()));
    let _ = (platform.leave)();
    match rendered
    {
        | Ok(()) => 0,
        | Err(error) =>
        {
            let _ = writeln!(stderr, "mep: {error}");
            1
        }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::{
        io,
        sync::{Arc, Mutex}
    };

    use mep_core::{
        clip::MemClipboard,
        env::FixedEnv,
        keys::{Key, KeyCode},
        memfs::MemFs
    };

    use super::*;

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

    struct Broken;

    impl Write for Broken
    {
        fn write(&mut self, _: &[u8]) -> io::Result<usize>
        {
            Err(io::Error::other("terminal gone"))
        }

        fn flush(&mut self) -> io::Result<()>
        {
            Ok(())
        }
    }

    /// A file system the test can still read after `run` took it
    struct SharedFs(Arc<MemFs>);

    impl Fs for SharedFs
    {
        fn read(&self, path: &std::path::Path) -> io::Result<Vec<u8>>
        {
            self.0.read(path)
        }

        fn write_atomic(&self, path: &std::path::Path, data: &[u8]) -> io::Result<()>
        {
            self.0.write_atomic(path, data)
        }

        fn rename(&self, from: &std::path::Path, to: &std::path::Path) -> io::Result<()>
        {
            self.0.rename(from, to)
        }

        fn metadata(&self, path: &std::path::Path) -> io::Result<mep_core::io::FileMeta>
        {
            self.0.metadata(path)
        }

        fn read_dir_names(&self, dir: &std::path::Path) -> io::Result<Vec<OsString>>
        {
            self.0.read_dir_names(dir)
        }
    }

    fn platform(fs: MemFs, out: Box<dyn Write + Send>) -> Platform
    {
        platform_with(Box::new(fs), out)
    }

    fn platform_with(fs: Box<dyn Fs + Send>, out: Box<dyn Write + Send>) -> Platform
    {
        Platform {
            env: Box::new(FixedEnv::default()),
            fs,
            clipboard: Box::new(MemClipboard::default()),
            screen: Screen::new(out),
            size: (40, 6),
            enter: Box::new(|| Ok(())),
            leave: Box::new(|| Ok(()))
        }
    }

    fn keys(names: &[&str]) -> Box<dyn FnOnce(Sender<Event>)>
    {
        let keys: Vec<Key> = names.iter().map(|name| Key::parse(name).unwrap()).collect();
        Box::new(move |events| {
            for key in keys
            {
                events.send(Event::Key(key)).unwrap();
            }
            // Keep the channel open like a terminal that is still connected.
            std::mem::forget(events);
        })
    }

    fn args(list: &[&str]) -> Vec<OsString>
    {
        std::iter::once("mep").chain(list.iter().copied()).map(OsString::from).collect()
    }

    #[test]
    fn test_edit_save_and_exit()
    {
        let fs = Arc::new(MemFs::default().with_file("/work/a.txt", b"one\n"));
        let out = Shared::default();
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = run(args(&["a.txt"]), platform_with(Box::new(SharedFs(fs.clone())), Box::new(out.clone())), keys(&["X", "f8"]), &mut stdout, &mut stderr);
        assert_eq!(code, 0);
        assert_eq!(fs.file("/work/a.txt").unwrap(), b"xne\n");
        let screen = String::from_utf8_lossy(&out.0.lock().unwrap()).into_owned();
        assert!(screen.contains("/work/a.txt"));
        assert!(stderr.is_empty());
        let shared = SharedFs(fs);
        assert!(shared.rename(std::path::Path::new("/work/a.txt"), std::path::Path::new("/work/b.txt")).is_ok());
        assert_eq!(shared.read_dir_names(std::path::Path::new("/work")).unwrap(), vec![OsString::from("b.txt")]);
    }

    #[test]
    fn test_busy_search_polls_input()
    {
        let mut text = "x\n".repeat(25_000);
        text.push_str("needle\n");
        let fs = MemFs::default().with_file("/work/big.txt", text.as_bytes());
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let input: Box<dyn FnOnce(Sender<Event>)> = Box::new(|events| {
            let typed = |name: &str| Event::Key(Key::parse(name).unwrap());
            for event in [typed("alt+a"), typed("n"), typed("e"), typed("e"), typed("d"), typed("l"), typed("e"), typed("f3"), typed("f9"), typed("f8")]
            {
                events.send(event).unwrap();
            }
            std::mem::forget(events);
        });
        let code = run(args(&["big.txt"]), platform(fs, Box::new(Shared::default())), input, &mut stdout, &mut stderr);
        assert_eq!(code, 0);
    }

    #[test]
    fn test_busy_search_runs_slices_without_input()
    {
        let fs = MemFs::default().with_file("/work/big.txt", "x\n".repeat(25_000).as_bytes());
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let input: Box<dyn FnOnce(Sender<Event>)> = Box::new(|events| {
            let search = [Event::Key(Key::parse("alt+a").unwrap()), Event::Key(Key::plain(KeyCode::Char('q'))), Event::Key(Key::parse("f3").unwrap())];
            for event in search
            {
                events.send(event).unwrap();
            }
            thread::spawn(move || {
                thread::sleep(std::time::Duration::from_millis(200));
                let _ = events.send(Event::Key(Key::parse("f9").unwrap()));
                let _ = events.send(Event::Key(Key::parse("f8").unwrap()));
            });
        });
        assert_eq!(run(args(&["big.txt"]), platform(fs, Box::new(Shared::default())), input, &mut stdout, &mut stderr), 0);
    }

    #[test]
    fn test_redraw_frames_are_merged()
    {
        let (frames, receiver) = mpsc::channel();
        let out = Shared::default();
        frames.send((Frame::new(2, 1, 7), false)).unwrap();
        frames.send((Frame::new(2, 1, 7), true)).unwrap();
        drop(frames);
        let (events, _incoming) = mpsc::channel();
        render(receiver, Screen::new(Box::new(out.clone())), events).unwrap();
        assert!(String::from_utf8_lossy(&out.0.lock().unwrap()).contains("\u{1b}[2J"));
    }

    #[test]
    fn test_terminal_write_error_ends_the_editor()
    {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = run(args(&[]), platform(MemFs::default(), Box::new(Broken)), keys(&["x", "x", "x"]), &mut stdout, &mut stderr);
        assert_eq!(code, 1);
        assert_eq!(String::from_utf8(stderr).unwrap(), "mep: terminal gone\n");
    }

    #[test]
    fn test_terminal_setup_failure_is_reported()
    {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let mut platform = platform(MemFs::default(), Box::new(Shared::default()));
        platform.enter = Box::new(|| Err(io::Error::other("not a tty")));
        assert_eq!(run(args(&[]), platform, keys(&[]), &mut stdout, &mut stderr), 1);
        assert_eq!(String::from_utf8(stderr).unwrap(), "mep: cannot use the terminal: not a tty\n");
    }

    #[test]
    fn test_help_version_and_usage_errors()
    {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        assert_eq!(run(args(&["--version"]), platform(MemFs::default(), Box::new(Shared::default())), keys(&[]), &mut stdout, &mut stderr), 0);
        assert!(String::from_utf8(stdout).unwrap().starts_with("mep "));
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        assert_eq!(run(args(&["--bogus"]), platform(MemFs::default(), Box::new(Shared::default())), keys(&[]), &mut stdout, &mut stderr), 2);
        assert!(String::from_utf8(stderr).unwrap().contains("Usage:"));
        assert!(Shared::default().flush().is_ok());
        assert!(Broken.flush().is_ok());
    }
}
