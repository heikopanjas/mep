//! Terminal setup through crossterm (excluded from coverage, plan §9.1)
//!
//! Raw mode keeps the terminal from turning Ctrl+C, Ctrl+Z, Ctrl+\, Ctrl+S and
//! Ctrl+Q into signals or flow control; MEP binds several of them.

use std::{
    io::{self, stdout},
    sync::OnceLock,
    time::Duration
};

use crossterm::{
    cursor::SetCursorStyle,
    event::{self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}
};

use crate::InputGate;

/// Enters raw mode and the alternate screen, and asks for unambiguous key codes where supported
pub fn enter() -> io::Result<()>
{
    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    // The query waits for an answer the first time; returning from `shell` must not wait again.
    static ENHANCED: OnceLock<bool> = OnceLock::new();
    if *ENHANCED.get_or_init(|| terminal::supports_keyboard_enhancement().unwrap_or(false))
    {
        execute!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS))?;
    }
    Ok(())
}

/// Restores the terminal
pub fn leave() -> io::Result<()>
{
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    execute!(stdout(), SetCursorStyle::DefaultUserShape, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()
}

/// Restores the terminal before a panic message is printed
pub fn install_panic_hook()
{
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = leave();
        previous(info);
    }));
}

/// Waits for the next terminal event, stopping at the gate's checkpoint while a program has the terminal
pub fn read(gate: &InputGate) -> io::Result<event::Event>
{
    loop
    {
        gate.checkpoint();
        if event::poll(Duration::from_millis(50))?
        {
            return event::read();
        }
    }
}

/// Waits until Enter is pressed, with the terminal in its normal mode
pub fn wait_enter() -> io::Result<()>
{
    io::stdin().read_line(&mut String::new()).map(|_| ())
}

/// Returns the terminal size as (columns, rows)
pub fn size() -> io::Result<(u16, u16)>
{
    terminal::size()
}
