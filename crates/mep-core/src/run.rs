//! Background commands for `compile` and `paste` (IMPLEMENTATION_PLAN.md §3.5, §8 M4)
//!
//! The core never starts processes. It emits a [`Run`]; a job thread runs it
//! with [`execute`] and a [`Runner`] and sends back a [`Completion`].

use std::{
    io,
    path::{Path, PathBuf}
};

use crate::io::{Completion, Reply, RequestId};

/// A command line to run in the background
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run
{
    /// Matches the completion to the run
    pub id:          RequestId,
    /// Command line for the system shell
    pub command:     String,
    /// Working directory
    pub cwd:         PathBuf,
    /// Capture standard output only and discard standard error; otherwise both through one pipe
    pub stdout_only: bool
}

/// Result of a finished command
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran
{
    /// The command exited successfully
    pub success: bool,
    /// The captured output
    pub output:  Vec<u8>
}

/// Starts processes for the job thread
pub trait Runner
{
    /// Runs a command line through the system shell and waits for it
    ///
    /// # Errors
    ///
    /// Returns the operating system error if the shell cannot be started.
    fn capture(&self, command: &str, cwd: &Path, stdout_only: bool) -> io::Result<Ran>;
}

/// Runs `run` with `runner`
pub fn execute(run: Run, runner: &dyn Runner) -> Completion
{
    Completion { id: run.id, reply: Reply::Ran(runner.capture(&run.command, &run.cwd, run.stdout_only)) }
}
