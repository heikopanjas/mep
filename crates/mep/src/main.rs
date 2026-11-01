//! MEP entry point: builds the real platform and runs the editor (excluded from coverage, plan §9.1)

#![allow(clippy::bool_comparison)]

use std::{
    io,
    rc::Rc,
    sync::{Arc, Mutex},
    thread
};

use mep::{
    Platform,
    os::{OsEnv, OsFs},
    process::OsRunner,
    run, select_clipboard, sys
};
use mep_clip::{Adapter, osc52::Osc52, system::ClipboardRs};
use mep_core::{clip::SystemClipboard, env::Env};
use mep_term::{InputGate, SharedOut, forward_input, sys as term};

fn main()
{
    let out = SharedOut::new(Box::new(io::stdout()));
    let mut system = || ClipboardRs::new().map(|system| Box::new(Adapter::new(system)) as Box<dyn SystemClipboard + Send>);
    let clipboard = select_clipboard(&OsEnv, &mut system, Box::new(Osc52::new(Box::new(out.clone()))));
    let gate = Arc::new(InputGate::default());
    term::install_panic_hook();
    let platform = Platform {
        env: Box::new(OsEnv),
        fs: Arc::new(OsFs),
        clipboard: Arc::new(Mutex::new(clipboard)),
        runner: Arc::new(OsRunner(OsEnv.os())),
        out,
        size: term::size().unwrap_or((80, 25)),
        enter: Rc::new(term::enter),
        leave: Rc::new(term::leave),
        wait_enter: Box::new(term::wait_enter),
        quiet: Box::new(|| Box::new(sys::ignore_interrupts())),
        gate: gate.clone()
    };
    let input = Box::new(move |events| {
        thread::spawn(move || forward_input(&mut || term::read(&gate), &events));
    });
    let code = run(std::env::args_os().collect(), platform, input, &mut io::stdout(), &mut io::stderr());
    std::process::exit(code);
}
