//! Keeps Ctrl+C and friends away from the editor while a `shell` command runs (excluded from coverage, plan §9.1)
//!
//! In cooked mode the terminal sends interrupt signals to every process in the
//! foreground, the editor included. While the guard lives, the editor handles
//! them by doing nothing; the child process keeps the default behavior.

/// Undoes [`ignore_interrupts`] when dropped
pub struct Interrupts(#[cfg(unix)] Vec<signal_hook::SigId>);

/// Makes the editor survive Ctrl+C, Ctrl+\ and Ctrl+Z until the guard is dropped
#[cfg(unix)]
pub fn ignore_interrupts() -> Interrupts
{
    use std::sync::{Arc, atomic::AtomicBool};

    let flag = Arc::new(AtomicBool::new(false));
    let signals = [signal_hook::consts::SIGINT, signal_hook::consts::SIGQUIT, signal_hook::consts::SIGTSTP];
    Interrupts(signals.iter().filter_map(|signal| signal_hook::flag::register(*signal, flag.clone()).ok()).collect())
}

#[cfg(unix)]
impl Drop for Interrupts
{
    fn drop(&mut self)
    {
        for id in self.0.drain(..)
        {
            signal_hook::low_level::unregister(id);
        }
    }
}

/// Console control handler that swallows Ctrl+C and Ctrl+Break; child processes do not inherit it
#[cfg(windows)]
unsafe extern "system" fn swallow(_: u32) -> windows_sys::core::BOOL
{
    1
}

/// Makes the editor survive Ctrl+C and Ctrl+Break until the guard is dropped
#[cfg(windows)]
pub fn ignore_interrupts() -> Interrupts
{
    // SAFETY: registers a handler function that lives for the whole program.
    unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(swallow), 1) };
    Interrupts()
}

#[cfg(windows)]
impl Drop for Interrupts
{
    fn drop(&mut self)
    {
        // SAFETY: removes the handler registered by `ignore_interrupts`.
        unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(swallow), 0) };
    }
}
