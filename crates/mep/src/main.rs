//! MEP entry point: builds the real platform and runs the editor (excluded from coverage, plan §9.1)

#![allow(clippy::bool_comparison)]

use std::{io, thread};

use mep::{
    Platform,
    os::{OsEnv, OsFs},
    run
};
use mep_clip::{Adapter, system::ClipboardRs};
use mep_core::clip::{MemClipboard, SystemClipboard};
use mep_term::{Screen, forward_input, sys};

fn main()
{
    let clipboard: Box<dyn SystemClipboard + Send> = match ClipboardRs::new()
    {
        | Some(system) => Box::new(Adapter::new(system)),
        | None => Box::new(MemClipboard { content: None, unavailable: true })
    };
    sys::install_panic_hook();
    let platform = Platform {
        env: Box::new(OsEnv),
        fs: Box::new(OsFs),
        clipboard,
        screen: Screen::new(Box::new(io::stdout())),
        size: sys::size().unwrap_or((80, 25)),
        enter: Box::new(sys::enter),
        leave: Box::new(sys::leave)
    };
    let input = Box::new(|events| {
        thread::spawn(move || forward_input(&mut sys::read, &events));
    });
    let code = run(std::env::args_os().collect(), platform, input, &mut io::stdout(), &mut io::stderr());
    std::process::exit(code);
}
