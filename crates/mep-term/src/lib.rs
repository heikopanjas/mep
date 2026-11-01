//! Terminal front end (IMPLEMENTATION_PLAN.md §7)
//!
//! Key conversion, colors and frame diffing are pure and tested here; the
//! calls that switch the terminal into raw mode live in `sys.rs`.
//!
//! Without the kitty keyboard protocol, terminals send some keys alike:
//! Ctrl+M arrives as Enter, Ctrl+[ as Esc, Ctrl+H as Backspace, Ctrl+I as Tab,
//! and Alt+key as an Esc prefix that crossterm turns back into Alt.
//!
//! Windows reports AltGr as Ctrl+Alt, so a character other than an ASCII
//! letter or digit with both modifiers is typed text, not a key combination.

#![cfg_attr(test, feature(coverage_attribute))]
#![allow(clippy::bool_comparison)]

pub mod sys;

use std::{
    io::{self, Write},
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc::Sender},
    time::Duration
};

use crossterm::{
    Command,
    cursor::{MoveTo, SetCursorStyle, Show},
    event::{self, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType}
};
use mep_core::{
    Event,
    frame::{CursorShape, Frame},
    keys::{Key, KeyCode}
};

/// Converts a crossterm key event; `None` for releases and keys MEP does not know
pub fn convert_key(event: KeyEvent) -> Option<Key>
{
    require!(event.kind != KeyEventKind::Release, None);
    let mods = event.modifiers;
    let mut ctrl = mods.contains(KeyModifiers::CONTROL);
    let mut alt = mods.contains(KeyModifiers::ALT);
    // Command on macOS, the Windows key elsewhere; only the kitty keyboard protocol reports it.
    let cmd = mods.contains(KeyModifiers::SUPER);
    let mut shift = mods.contains(KeyModifiers::SHIFT);
    let keypad = event.state.contains(KeyEventState::KEYPAD);
    let code = match event.code
    {
        | event::KeyCode::Char('+') if keypad == true => KeyCode::NumPlus,
        | event::KeyCode::Char('-') if keypad == true => KeyCode::NumMinus,
        | event::KeyCode::Char(ch) if keypad == true && ch.is_ascii_digit() => KeyCode::NumDigit(ch as u8 - b'0'),
        | event::KeyCode::Char(ch) =>
        {
            // The character already includes Shift; Ctrl, Alt and Cmd combine with the lowercase letter.
            shift = false;
            // Windows reports AltGr as Ctrl+Alt; a character other than a letter or digit is what AltGr typed.
            if ctrl && alt && ch.is_ascii_alphanumeric() == false
            {
                ctrl = false;
                alt = false;
            }
            KeyCode::Char(
                if ctrl || alt || cmd
                {
                    ch.to_ascii_lowercase()
                }
                else
                {
                    ch
                }
            )
        }
        | event::KeyCode::F(number @ 1..=12) => KeyCode::F(number),
        | event::KeyCode::Home => KeyCode::Home,
        | event::KeyCode::End => KeyCode::End,
        | event::KeyCode::Left => KeyCode::Left,
        | event::KeyCode::Right => KeyCode::Right,
        | event::KeyCode::Up => KeyCode::Up,
        | event::KeyCode::Down => KeyCode::Down,
        | event::KeyCode::PageUp => KeyCode::PgUp,
        | event::KeyCode::PageDown => KeyCode::PgDn,
        | event::KeyCode::Insert => KeyCode::Ins,
        | event::KeyCode::Delete => KeyCode::Del,
        | event::KeyCode::Backspace => KeyCode::Bksp,
        | event::KeyCode::Tab => KeyCode::Tab,
        | event::KeyCode::BackTab =>
        {
            shift = true;
            KeyCode::Tab
        }
        | event::KeyCode::Esc => KeyCode::Esc,
        | event::KeyCode::Enter => KeyCode::Enter,
        | _ => return None
    };
    Some(Key { code, ctrl, alt, shift, cmd })
}

/// Converts a crossterm event; `None` for events MEP ignores (mouse, focus, paste)
pub fn convert_event(event: event::Event) -> Option<Event>
{
    match event
    {
        | event::Event::Key(key) => convert_key(key).map(Event::Key),
        | event::Event::Resize(cols, rows) => Some(Event::Resize(cols, rows)),
        | _ => None
    }
}

/// Reads terminal events with `read` and forwards the converted ones until reading or sending fails
pub fn forward_input(read: &mut dyn FnMut() -> io::Result<event::Event>, events: &Sender<Event>)
{
    while let Ok(event) = read()
    {
        if let Some(event) = convert_event(event) &&
            events.send(event).is_err()
        {
            return;
        }
    }
}

/// The 16 DOS colors in attribute order (M guide Table 7.1 p. 58)
const COLORS: [Color; 16] = [
    Color::Black,
    Color::DarkBlue,
    Color::DarkGreen,
    Color::DarkCyan,
    Color::DarkRed,
    Color::DarkMagenta,
    Color::DarkYellow,
    Color::Grey,
    Color::DarkGrey,
    Color::Blue,
    Color::Green,
    Color::Cyan,
    Color::Red,
    Color::Magenta,
    Color::Yellow,
    Color::White
];

/// Returns the foreground and background colors of a DOS attribute
///
/// Background values 8–F meant blinking text; they are drawn as bright backgrounds (plan §6.4).
pub fn colors(attr: u8) -> (Color, Color)
{
    (COLORS[usize::from(attr & 0x0f)], COLORS[usize::from(attr >> 4)])
}

/// Returns the terminal cursor style for a cursor shape
pub fn cursor_style(shape: CursorShape) -> SetCursorStyle
{
    match shape
    {
        | CursorShape::Block => SetCursorStyle::SteadyBlock,
        | CursorShape::Bar => SetCursorStyle::SteadyBar,
        | CursorShape::Underline => SetCursorStyle::SteadyUnderScore
    }
}

/// Appends a command's escape sequence to a buffer; writing to memory cannot fail
fn emit(buffer: &mut Vec<u8>, command: impl Command)
{
    let _ = queue!(buffer, command);
}

/// Terminal output shared by the render thread and the OSC 52 clipboard
///
/// Each `write_all` holds the lock for the whole buffer, so a frame and a
/// clipboard sequence never mix.
#[derive(Clone)]
pub struct SharedOut(Arc<Mutex<Box<dyn Write + Send>>>);

impl SharedOut
{
    /// Shares `out`
    pub fn new(out: Box<dyn Write + Send>) -> Self
    {
        Self(Arc::new(Mutex::new(out)))
    }

    /// Takes the output for itself until the guard is dropped; frames wait meanwhile
    pub fn lock(&self) -> MutexGuard<'_, Box<dyn Write + Send>>
    {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Lets another thread stop the input thread from reading keys, so a child process gets them
///
/// The input thread calls [`InputGate::checkpoint`] between reads.
#[derive(Debug, Default)]
pub struct InputGate
{
    state:   Mutex<GateState>,
    changed: Condvar
}

#[derive(Debug, Default)]
struct GateState
{
    paused:  bool,
    parked:  bool,
    /// An input thread has reached a checkpoint; without one there is nothing to wait for
    reading: bool
}

/// How long [`InputGate::pause`] waits for the input thread; a thread that ended never parks
const PARK_TIMEOUT: Duration = Duration::from_secs(1);

impl InputGate
{
    fn state(&self) -> MutexGuard<'_, GateState>
    {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Stops the input thread at its next checkpoint and waits until it got there
    pub fn pause(&self)
    {
        let mut state = self.state();
        state.paused = true;
        let _ = self.changed.wait_timeout_while(state, PARK_TIMEOUT, |state| state.reading == true && state.parked == false);
    }

    /// Lets the input thread read again
    pub fn resume(&self)
    {
        self.state().paused = false;
        self.changed.notify_all();
    }

    /// Called by the input thread before each read; waits while paused
    pub fn checkpoint(&self)
    {
        let mut state = self.state();
        state.reading = true;
        while state.paused == true
        {
            state.parked = true;
            self.changed.notify_all();
            state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
        state.parked = false;
    }
}

impl Write for SharedOut
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        self.lock().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
    {
        self.lock().write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()>
    {
        self.lock().flush()
    }
}

/// Draws frames on a terminal, writing only the cells that changed
pub struct Screen
{
    out:      Box<dyn Write + Send>,
    previous: Option<Frame>
}

impl Screen
{
    /// Creates a screen that writes to `out`
    pub fn new(out: Box<dyn Write + Send>) -> Self
    {
        Self { out, previous: None }
    }

    /// Draws a frame; `redraw` repaints every cell (`setwindow`)
    ///
    /// # Errors
    ///
    /// Returns the writer's error.
    pub fn draw(&mut self, frame: &Frame, redraw: bool) -> io::Result<()>
    {
        let previous = self.previous.take().filter(|previous| redraw == false && (previous.cols(), previous.rows()) == (frame.cols(), frame.rows()));
        let mut buffer = Vec::new();
        if previous.is_none()
        {
            emit(&mut buffer, Clear(ClearType::All));
        }
        let mut attr = None;
        // Where the terminal cursor stands after the last print; adjacent cells need no move.
        let mut at = None;
        for row in 0..frame.rows()
        {
            for col in 0..frame.cols()
            {
                let cell = frame.cell(col, row);
                let changed = previous.as_ref().is_none_or(|previous| previous.cell(col, row) != cell);
                // Continuation cells belong to the wide character before them.
                if let Some(cell) = cell.filter(|cell| changed && cell.text.is_empty() == false)
                {
                    if at != Some((col, row))
                    {
                        emit(&mut buffer, MoveTo(col, row));
                    }
                    let wide = frame.cell(col + 1, row).is_some_and(|next| next.text.is_empty());
                    at = Some((col + 1 + u16::from(wide), row));
                    if attr != Some(cell.attr)
                    {
                        let (fg, bg) = colors(cell.attr);
                        emit(&mut buffer, SetForegroundColor(fg));
                        emit(&mut buffer, SetBackgroundColor(bg));
                        attr = Some(cell.attr);
                    }
                    emit(&mut buffer, Print(&cell.text));
                }
            }
        }
        emit(&mut buffer, cursor_style(frame.shape));
        emit(&mut buffer, MoveTo(frame.cursor.0, frame.cursor.1));
        emit(&mut buffer, Show);
        self.previous = Some(frame.clone());
        self.out.write_all(&buffer)?;
        self.out.flush()
    }
}

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

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use std::sync::{Arc, Mutex, mpsc};

    use crossterm::event::KeyEvent;

    use super::*;

    fn key(code: event::KeyCode, modifiers: KeyModifiers) -> KeyEvent
    {
        KeyEvent::new(code, modifiers)
    }

    fn name(event: KeyEvent) -> Option<String>
    {
        convert_key(event).map(|key| key.to_string())
    }

    #[test]
    fn test_character_keys()
    {
        assert_eq!(name(key(event::KeyCode::Char('a'), KeyModifiers::NONE)).unwrap(), "a");
        assert_eq!(name(key(event::KeyCode::Char('A'), KeyModifiers::SHIFT)).unwrap(), "A");
        assert_eq!(name(key(event::KeyCode::Char('A'), KeyModifiers::CONTROL | KeyModifiers::SHIFT)).unwrap(), "ctrl+a");
        assert_eq!(name(key(event::KeyCode::Char('X'), KeyModifiers::ALT)).unwrap(), "alt+x");
        assert_eq!(name(key(event::KeyCode::Char('='), KeyModifiers::ALT)).unwrap(), "alt+=");
        assert_eq!(name(key(event::KeyCode::Char('+'), KeyModifiers::NONE)).unwrap(), "+");
        assert_eq!(name(key(event::KeyCode::Char('-'), KeyModifiers::NONE)).unwrap(), "-");
        assert_eq!(name(key(event::KeyCode::Char('C'), KeyModifiers::SUPER | KeyModifiers::SHIFT)).unwrap(), "cmd+c");
        assert_eq!(name(key(event::KeyCode::F(3), KeyModifiers::SUPER)).unwrap(), "cmd+f3");
    }

    #[test]
    fn test_altgr_characters_are_typed()
    {
        let altgr = KeyModifiers::CONTROL | KeyModifiers::ALT;
        assert_eq!(name(key(event::KeyCode::Char('{'), altgr)).unwrap(), "{");
        assert_eq!(name(key(event::KeyCode::Char('|'), altgr | KeyModifiers::SHIFT)).unwrap(), "|");
        assert_eq!(name(key(event::KeyCode::Char('€'), altgr)).unwrap(), "€");
        assert_eq!(name(key(event::KeyCode::Char('A'), altgr | KeyModifiers::SHIFT)).unwrap(), "ctrl+alt+a");
        assert_eq!(name(key(event::KeyCode::Char('7'), altgr)).unwrap(), "ctrl+alt+7");
    }

    #[test]
    fn test_named_and_function_keys()
    {
        let cases = [
            (event::KeyCode::F(1), "f1"),
            (event::KeyCode::F(12), "f12"),
            (event::KeyCode::Home, "home"),
            (event::KeyCode::End, "end"),
            (event::KeyCode::Left, "left"),
            (event::KeyCode::Right, "right"),
            (event::KeyCode::Up, "up"),
            (event::KeyCode::Down, "down"),
            (event::KeyCode::PageUp, "pgup"),
            (event::KeyCode::PageDown, "pgdn"),
            (event::KeyCode::Insert, "ins"),
            (event::KeyCode::Delete, "del"),
            (event::KeyCode::Backspace, "bksp"),
            (event::KeyCode::Tab, "tab"),
            (event::KeyCode::BackTab, "shift+tab"),
            (event::KeyCode::Esc, "esc"),
            (event::KeyCode::Enter, "enter")
        ];
        for (code, want) in cases
        {
            assert_eq!(name(key(code, KeyModifiers::NONE)).unwrap(), want);
        }
        assert_eq!(name(key(event::KeyCode::PageDown, KeyModifiers::CONTROL)).unwrap(), "ctrl+pgdn");
        assert_eq!(name(key(event::KeyCode::F(3), KeyModifiers::SHIFT)).unwrap(), "shift+f3");
        assert_eq!(name(key(event::KeyCode::Backspace, KeyModifiers::ALT)).unwrap(), "alt+bksp");
    }

    #[test]
    fn test_keypad_keys()
    {
        let keypad = |ch| KeyEvent::new_with_kind_and_state(event::KeyCode::Char(ch), KeyModifiers::NONE, KeyEventKind::Press, KeyEventState::KEYPAD);
        assert_eq!(name(keypad('+')).unwrap(), "num+");
        assert_eq!(name(keypad('-')).unwrap(), "num-");
        assert_eq!(name(keypad('4')).unwrap(), "num4");
        assert_eq!(name(keypad('*')).unwrap(), "*");
    }

    #[test]
    fn test_ignored_keys()
    {
        assert_eq!(convert_key(key(event::KeyCode::F(13), KeyModifiers::NONE)), None);
        assert_eq!(convert_key(key(event::KeyCode::CapsLock, KeyModifiers::NONE)), None);
        let release = KeyEvent::new_with_kind(event::KeyCode::Char('a'), KeyModifiers::NONE, KeyEventKind::Release);
        assert_eq!(convert_key(release), None);
    }

    #[test]
    fn test_events()
    {
        assert!(matches!(convert_event(event::Event::Resize(80, 24)), Some(Event::Resize(80, 24))));
        assert!(matches!(convert_event(event::Event::Key(key(event::KeyCode::Esc, KeyModifiers::NONE))), Some(Event::Key(_))));
        assert!(convert_event(event::Event::FocusGained).is_none());
    }

    #[test]
    fn test_forward_input_until_read_fails()
    {
        let (sender, receiver) = mpsc::channel();
        let mut script = vec![Err(io::Error::other("closed")), Ok(event::Event::FocusLost), Ok(event::Event::Resize(1, 2))];
        forward_input(&mut || script.pop().unwrap(), &sender);
        assert!(matches!(receiver.try_recv(), Ok(Event::Resize(1, 2))));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn test_forward_input_stops_when_the_editor_is_gone()
    {
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        let mut reads = 0;
        forward_input(
            &mut || {
                reads += 1;
                Ok(event::Event::Resize(1, 1))
            },
            &sender
        );
        assert_eq!(reads, 1);
    }

    #[test]
    fn test_colors()
    {
        assert_eq!(colors(0x07), (Color::Grey, Color::Black));
        assert_eq!(colors(0x70), (Color::Black, Color::Grey));
        assert_eq!(colors(0x0e), (Color::Yellow, Color::Black));
        assert_eq!(colors(0xf1), (Color::DarkBlue, Color::White));
    }

    #[test]
    fn test_cursor_styles()
    {
        assert_eq!(cursor_style(CursorShape::Block), SetCursorStyle::SteadyBlock);
        assert_eq!(cursor_style(CursorShape::Bar), SetCursorStyle::SteadyBar);
        assert_eq!(cursor_style(CursorShape::Underline), SetCursorStyle::SteadyUnderScore);
    }

    fn frame(text: &str) -> Frame
    {
        let mut frame = Frame::new(4, 1, 0x07);
        frame.put_str(0, 0, text, 0x07);
        frame
    }

    /// A writer whose bytes the test can read afterwards
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

    impl Shared
    {
        fn take(&self) -> String
        {
            String::from_utf8(std::mem::take(&mut *self.0.lock().unwrap())).unwrap()
        }
    }

    #[test]
    fn test_input_gate_parks_the_reader_until_resumed()
    {
        let gate = Arc::new(InputGate::default());
        // Without a reader there is nothing to wait for.
        gate.pause();
        gate.resume();
        gate.checkpoint();
        let (reads, counted) = std::sync::mpsc::channel();
        let reader = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                for _ in 0..2
                {
                    gate.checkpoint();
                    reads.send(()).unwrap();
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
        };
        counted.recv().unwrap();
        gate.pause();
        assert!(gate.state().parked);
        assert!(counted.try_recv().is_err());
        gate.resume();
        counted.recv().unwrap();
        reader.join().unwrap();
        // A reader that ended never parks; the pause gives up after the timeout.
        gate.pause();
        gate.resume();
    }

    #[test]
    fn test_shared_out_forwards_to_the_writer()
    {
        let inner = Shared::default();
        let mut out = SharedOut::new(Box::new(inner.clone()));
        let mut other = out.clone();
        out.write_all(b"ab").unwrap();
        assert_eq!(other.write(b"c").unwrap(), 1);
        other.flush().unwrap();
        assert_eq!(inner.take(), "abc");
    }

    #[test]
    fn test_first_draw_paints_everything_then_only_changes()
    {
        let out = Shared::default();
        let mut screen = Screen::new(Box::new(out.clone()));
        screen.draw(&frame("ab"), false).unwrap();
        let first = out.take();
        assert!(first.starts_with("\u{1b}[2J"));
        assert!(first.contains('a') && first.contains('b'));
        let mut changed = frame("ab");
        changed.put(1, 0, "日", 2, 0x70);
        changed.cursor = (3, 0);
        changed.shape = CursorShape::Bar;
        screen.draw(&changed, false).unwrap();
        let second = out.take();
        assert!(second.contains("\u{1b}[1;2H"));
        assert!(second.contains('日'));
        assert!(second.contains('a') == false);
        assert!(second.contains("\u{1b}[6 q"));
    }

    #[test]
    fn test_redraw_and_size_change_paint_everything()
    {
        let out = Shared::default();
        let mut screen = Screen::new(Box::new(out.clone()));
        screen.draw(&frame("ab"), false).unwrap();
        screen.draw(&frame("ab"), true).unwrap();
        screen.draw(&Frame::new(2, 1, 0x07), false).unwrap();
        let text = out.take();
        assert_eq!(text.matches("\u{1b}[2J").count(), 3);
    }

    struct Failing
    {
        write: bool
    }

    impl Write for Failing
    {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>
        {
            match self.write
            {
                | true => Ok(buf.len()),
                | false => Err(io::Error::other("write"))
            }
        }

        fn flush(&mut self) -> io::Result<()>
        {
            Err(io::Error::other("flush"))
        }
    }

    #[test]
    fn test_write_and_flush_errors_are_returned()
    {
        assert_eq!(Screen::new(Box::new(Failing { write: false })).draw(&frame("a"), false).unwrap_err().to_string(), "write");
        assert_eq!(Screen::new(Box::new(Failing { write: true })).draw(&frame("a"), false).unwrap_err().to_string(), "flush");
        assert!(Shared::default().flush().is_ok());
    }
}
