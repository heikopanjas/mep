//! Terminal setup through crossterm (excluded from coverage, plan §9.1)
//!
//! Raw mode keeps the terminal from turning Ctrl+C, Ctrl+Z, Ctrl+\, Ctrl+S and
//! Ctrl+Q into signals or flow control; MEP binds several of them.

use std::io::{self, stdout};

use crossterm::{
    cursor::SetCursorStyle,
    event::{self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen}
};

/// Enters raw mode and the alternate screen, and asks for unambiguous key codes where supported
pub fn enter() -> io::Result<()>
{
    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    if terminal::supports_keyboard_enhancement().unwrap_or(false)
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

/// Waits for the next terminal event
pub fn read() -> io::Result<event::Event>
{
    event::read()
}

/// Returns the terminal size as (columns, rows)
pub fn size() -> io::Result<(u16, u16)>
{
    terminal::size()
}
