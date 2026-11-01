//! MEP entry point: builds the real platform and runs the editor (excluded from coverage, plan §9.1)

#![allow(clippy::bool_comparison)]

use std::{io, thread};

use mep::{
    Platform,
    os::{OsEnv, OsFs},
    run, select_clipboard
};
use mep_clip::{Adapter, osc52::Osc52, system::ClipboardRs};
use mep_core::clip::SystemClipboard;
use mep_term::{Screen, SharedOut, forward_input, sys};

fn main()
{
    let out = SharedOut::new(Box::new(io::stdout()));
    let mut system = || ClipboardRs::new().map(|system| Box::new(Adapter::new(system)) as Box<dyn SystemClipboard + Send>);
    let clipboard = select_clipboard(&OsEnv, &mut system, Box::new(Osc52::new(Box::new(out.clone()))));
    sys::install_panic_hook();
    let platform = Platform {
        env: Box::new(OsEnv),
        fs: Box::new(OsFs),
        clipboard,
        screen: Screen::new(Box::new(out)),
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
