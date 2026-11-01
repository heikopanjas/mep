//! Work the core asks of the front end and waits for (IMPLEMENTATION_PLAN.md §8 M4)
//!
//! `shell` hands the terminal to other programs and extensions call back into
//! the editor, so neither can run in a worker thread; the core calls the host
//! and waits. The host also serves requests at once for extensions, whose
//! callbacks cannot wait for a completion event.

use std::path::Path;

use crate::{
    ext::{ExtApi, ExtArg, Extension, SwitchValue},
    io::{Completion, Request},
    keys::Key,
    run::Run
};

/// The front end services the core waits for
pub trait Host
{
    /// Runs command lines on the terminal one after another, or the interactive shell when `commands` is empty
    ///
    /// With `wait`, asks for Enter afterwards. Returns `true` if every command started and succeeded.
    fn shell(&self, commands: &[String], wait: bool) -> bool;

    /// Serves a request at once; `None` if this host cannot
    fn serve(&self, request: Request) -> Option<Completion>;

    /// Runs a command line at once; `None` if this host cannot
    fn run(&self, run: Run) -> Option<Completion>;

    /// Waits for the next key; `None` if there is none to wait for
    fn read_key(&self) -> Option<Key>;

    /// Restarts (`true`) or stops (`false`) the editor's keyboard reading
    fn keyboard(&self, hooked: bool);

    /// Loads an extension library and reads its tables
    ///
    /// # Errors
    ///
    /// Returns why the library cannot be loaded.
    fn load(&self, path: &Path) -> Result<Extension, String>;

    /// Runs `WhenLoaded` of a loaded library, after its functions are known
    fn init(&self, library: usize, api: &mut dyn ExtApi);

    /// Calls an extension function
    fn call(&self, id: usize, arg: &ExtArg, key: u32, meta: bool, api: &mut dyn ExtApi) -> bool;

    /// Sets an extension switch; `false` if the extension rejects the value
    fn set_switch(&self, id: usize, value: &SwitchValue, api: &mut dyn ExtApi) -> bool;
}

/// Host without a terminal or extensions: nothing can run
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHost;

impl Host for NoHost
{
    fn shell(&self, _: &[String], _: bool) -> bool
    {
        false
    }

    fn serve(&self, _: Request) -> Option<Completion>
    {
        None
    }

    fn run(&self, _: Run) -> Option<Completion>
    {
        None
    }

    fn read_key(&self) -> Option<Key>
    {
        None
    }

    fn keyboard(&self, _: bool)
    {
    }

    fn load(&self, _: &Path) -> Result<Extension, String>
    {
        Err("extensions are not available".into())
    }

    fn init(&self, _: usize, _: &mut dyn ExtApi)
    {
    }

    fn call(&self, _: usize, _: &ExtArg, _: u32, _: bool, _: &mut dyn ExtApi) -> bool
    {
        false
    }

    fn set_switch(&self, _: usize, _: &SwitchValue, _: &mut dyn ExtApi) -> bool
    {
        false
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;
    use crate::{Editor, Startup, env::FixedEnv, io::RequestKind};

    #[test]
    fn test_no_host_runs_nothing()
    {
        let host = NoHost;
        assert!(host.shell(&["true".to_string()], false) == false);
        assert!(host.serve(Request { id: 1, kind: RequestKind::ClipboardRead }).is_none());
        assert!(host.run(Run { id: 2, command: "true".into(), cwd: "/".into(), stdout_only: false }).is_none());
        assert!(host.read_key().is_none());
        host.keyboard(false);
        assert_eq!(host.load(Path::new("x.so")), Err("extensions are not available".to_string()));
        let startup =
            Startup { cols: 40, rows: 8, exe: "mep".into(), files: Vec::new(), execute: Vec::new(), tools_ini: false, temporary: true };
        let mut editor = Editor::new(Box::new(FixedEnv::default()), startup);
        host.init(0, &mut editor);
        assert!(host.call(0, &ExtArg::None { y: 0, x: 0 }, 0, false, &mut editor) == false);
        assert!(host.set_switch(0, &SwitchValue::Numeric(1), &mut editor) == false);
    }
}
