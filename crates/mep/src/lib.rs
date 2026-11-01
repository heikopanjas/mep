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
pub mod host;
pub mod os;
pub mod process;
pub mod sys;
pub mod tz;

use std::{
    any::Any,
    ffi::OsString,
    io::{self, Write},
    rc::Rc,
    sync::{
        Arc, PoisonError,
        mpsc::{self, Receiver, Sender}
    },
    thread
};

use mep_core::{
    Editor, Event, Startup,
    clip::SystemClipboard,
    env::Env,
    frame::Frame,
    io::{Fs, Request, serve},
    run::{Run, Runner, execute}
};
use mep_ext::Extensions;
use mep_term::{InputGate, Screen, SharedOut};

use crate::{
    host::{Inbox, SharedClipboard, TerminalHost},
    process::interactive_shell
};

/// The operating system pieces the editor runs on
pub struct Platform
{
    /// Environment and clock
    pub env:        Box<dyn Env>,
    /// File system, used by the I/O worker and extensions
    pub fs:         Arc<dyn Fs + Send + Sync>,
    /// System clipboard, used by the I/O worker and extensions
    pub clipboard:  SharedClipboard,
    /// Runs `compile` and `paste` commands, used by the job thread and extensions
    pub runner:     Arc<dyn Runner + Send + Sync>,
    /// Terminal output, shared by the render thread, `shell` and the bell
    pub out:        SharedOut,
    /// Terminal size as (columns, rows)
    pub size:       (u16, u16),
    /// Switches the terminal into editing mode; called after the command line parsed and after `shell`
    pub enter:      Rc<dyn Fn() -> io::Result<()>>,
    /// Restores the terminal; called before `shell` and at the end
    pub leave:      Rc<dyn Fn() -> io::Result<()>>,
    /// Waits for Enter after `shell` (`askrtn`)
    pub wait_enter: Box<dyn Fn() -> io::Result<()>>,
    /// Keeps interrupt signals away from the editor while `shell` runs a program and the guard lives
    pub quiet:      Box<dyn Fn() -> Box<dyn Any>>,
    /// Stops the input thread while `shell` runs a program
    pub gate:       Arc<InputGate>
}

/// Picks the system clipboard (plan §3.3)
///
/// Remote sessions (SSH) use OSC 52, so copies reach the clipboard of the
/// machine the terminal runs on; elsewhere the desktop clipboard from `system`,
/// and OSC 52 when there is none.
pub fn select_clipboard(
    env: &dyn Env, system: &mut dyn FnMut() -> Option<Box<dyn SystemClipboard + Send>>, osc52: Box<dyn SystemClipboard + Send>
) -> Box<dyn SystemClipboard + Send>
{
    let remote = env.var("SSH_TTY").is_some() || env.var("SSH_CONNECTION").is_some();
    match remote
    {
        | true => osc52,
        | false => system().unwrap_or(osc52)
    }
}

/// Serves requests until the editor closes the request channel (plan §3.5)
fn work(requests: Receiver<Request>, events: Sender<Event>, fs: Arc<dyn Fs + Send + Sync>, clipboard: SharedClipboard)
{
    for request in requests
    {
        let completion = serve(request, fs.as_ref(), clipboard.lock().unwrap_or_else(PoisonError::into_inner).as_mut());
        let _ = events.send(Event::Completed(completion));
    }
}

/// Runs background commands one after another until the editor closes the channel (plan §3.5)
fn run_jobs(runs: Receiver<Run>, events: Sender<Event>, runner: Arc<dyn Runner + Send + Sync>)
{
    for run in runs
    {
        let _ = events.send(Event::Completed(execute(run, runner.as_ref())));
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
    let startup = Startup {
        cols: platform.size.0,
        rows: platform.size.1,
        exe,
        files,
        execute: options.execute,
        tools_ini: options.no_tools_ini == false,
        temporary: options.temporary
    };
    let (events, incoming) = mpsc::channel();
    let inbox = Rc::new(Inbox::new(incoming));
    let host = TerminalHost {
        suspend:     platform.leave.clone(),
        resume:      platform.enter.clone(),
        wait_enter:  platform.wait_enter,
        quiet:       platform.quiet,
        gate:        platform.gate,
        out:         platform.out.clone(),
        os:          platform.env.os(),
        interactive: interactive_shell(platform.env.as_ref()),
        cwd:         platform.env.cwd(),
        fs:          platform.fs.clone(),
        clipboard:   platform.clipboard.clone(),
        runner:      platform.runner.clone(),
        inbox:       inbox.clone(),
        extensions:  Extensions::default()
    };
    let (requests, worker_requests) = mpsc::channel();
    let (jobs, job_runs) = mpsc::channel();
    let (frames, render_frames) = mpsc::channel();
    let (worker_events, job_events, render_events) = (events.clone(), events.clone(), events.clone());
    let worker = thread::spawn(move || work(worker_requests, worker_events, platform.fs, platform.clipboard));
    // A compile still running at exit is not waited for.
    thread::spawn(move || run_jobs(job_runs, job_events, platform.runner));
    let screen = Screen::new(Box::new(platform.out.clone()));
    let renderer = thread::spawn(move || render(render_frames, screen, render_events));
    input(events);
    let mut editor = Editor::new(platform.env, startup);
    editor.set_host(Rc::new(host));
    let mut out = platform.out;
    let mut outcome = editor.start();
    loop
    {
        for request in outcome.requests.drain(..)
        {
            let _ = requests.send(request);
        }
        for run in outcome.runs.drain(..)
        {
            let _ = jobs.send(run);
        }
        if outcome.bell == true
        {
            let _ = out.write_all(b"\x07").and_then(|()| out.flush());
        }
        if outcome.quit == true
        {
            break;
        }
        // A render thread that failed has already sent Event::Close, which ends the editor.
        let _ = frames.send((editor.render(), outcome.redraw));
        // While a long operation runs, input is polled and the next slice runs when none waits.
        let event = inbox.next(outcome.busy == false);
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

    #[test]
    fn test_select_clipboard_prefers_osc52_in_remote_sessions()
    {
        let tagged = |text: &str| -> Box<dyn SystemClipboard + Send> {
            Box::new(MemClipboard { content: Some(mep_core::clip::ClipboardPayload { text: text.into(), record: None }), unavailable: false })
        };
        let pick = |vars: &[(&str, &str)], system: bool| {
            let env = FixedEnv { vars: vars.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect(), ..FixedEnv::default() };
            let mut system = || system.then(|| tagged("system"));
            select_clipboard(&env, &mut system, tagged("osc52")).read().unwrap().text
        };
        assert_eq!(pick(&[], true), "system");
        assert_eq!(pick(&[], false), "osc52");
        assert_eq!(pick(&[("SSH_TTY", "/dev/pts/1")], true), "osc52");
        assert_eq!(pick(&[("SSH_CONNECTION", "1 2 3 4")], true), "osc52");
    }

    /// Runs nothing; every command prints its own command line as a compile message
    struct Echo;

    impl Runner for Echo
    {
        fn capture(&self, command: &str, _: &mep_core::Utf8TypedPathBuf, _: bool) -> io::Result<mep_core::run::Ran>
        {
            Ok(mep_core::run::Ran { success: true, output: format!("a.txt:1:{command}\n").into_bytes() })
        }
    }

    #[test]
    fn test_compile_runs_in_the_job_thread_and_rings_the_bell()
    {
        let fs = MemFs::default().with_file("/work/a.txt", b"one\n");
        let out = Shared::default();
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let input: Box<dyn FnOnce(Sender<Event>)> = Box::new(|events| {
            for name in ["alt+a", "alt+a", "m", "shift+f3"]
            {
                events.send(Event::Key(Key::parse(name).unwrap())).unwrap();
            }
            thread::spawn(move || {
                thread::sleep(std::time::Duration::from_millis(300));
                let _ = events.send(Event::Key(Key::parse("f8").unwrap()));
                // Ends the test even if the exit key went astray.
                thread::sleep(std::time::Duration::from_millis(2000));
                let _ = events.send(Event::Close);
            });
        });
        assert_eq!(run(args(&[]), platform(fs, Box::new(out.clone())), input, &mut stdout, &mut stderr), 0);
        let screen = String::from_utf8_lossy(&out.0.lock().unwrap()).into_owned();

        assert!(screen.contains('\u{7}'));
    }

    #[cfg(unix)]
    #[test]
    fn test_shell_hands_the_terminal_to_the_command()
    {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code =
            run(args(&[]), platform(MemFs::default(), Box::new(Shared::default())), keys(&["alt+a", "e", "x", "i", "t", "shift+f9", "f8"]), &mut stdout, &mut stderr);
        assert_eq!(code, 0);
    }

    fn platform(fs: MemFs, out: Box<dyn Write + Send>) -> Platform
    {
        platform_with(Arc::new(fs), out)
    }

    fn platform_with(fs: Arc<dyn Fs + Send + Sync>, out: Box<dyn Write + Send>) -> Platform
    {
        Platform {
            env: Box::new(FixedEnv::default()),
            fs,
            clipboard: Arc::new(Mutex::new(Box::new(MemClipboard::default()))),
            runner: Arc::new(Echo),
            out: SharedOut::new(out),
            size: (40, 6),
            enter: Rc::new(|| Ok(())),
            leave: Rc::new(|| Ok(())),
            wait_enter: Box::new(|| Ok(())),
            quiet: Box::new(|| Box::new(())),
            gate: Arc::new(InputGate::default())
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
        let code = run(args(&["a.txt"]), platform_with(fs.clone(), Box::new(out.clone())), keys(&["X", "f8"]), &mut stdout, &mut stderr);
        assert_eq!(code, 0);
        assert_eq!(fs.file("/work/a.txt").unwrap(), b"xne\n");
        let screen = String::from_utf8_lossy(&out.0.lock().unwrap()).into_owned();
        assert!(screen.contains("/work/a.txt"));
        assert!(stderr.is_empty());
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
        platform.enter = Rc::new(|| Err(io::Error::other("not a tty")));
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
