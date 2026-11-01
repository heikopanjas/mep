//! Child processes for `compile`, `paste` and `shell` (IMPLEMENTATION_PLAN.md §3.4, §8 M4)

use std::{
    io::{self, Read},
    process::{Command, Stdio}
};

use mep_core::{
    Utf8TypedPathBuf,
    env::{Env, Os},
    run::{Ran, Runner}
};

/// Returns the command that runs a command line through the system shell
pub fn shell_command(os: Os, line: &str) -> Command
{
    match os
    {
        | Os::Windows =>
        {
            let mut command = Command::new("cmd");
            command.arg("/C");
            raw_arg(&mut command, line);
            command
        }
        | _ =>
        {
            let mut command = Command::new("sh");
            command.arg("-c").arg(line);
            command
        }
    }
}

/// Passes a command line to `cmd` unquoted, as it expects it
#[cfg(windows)]
fn raw_arg(command: &mut Command, line: &str)
{
    std::os::windows::process::CommandExt::raw_arg(command, line);
}

/// Passes a command line to `cmd` unquoted, as it expects it
#[cfg(not(windows))]
fn raw_arg(command: &mut Command, line: &str)
{
    command.arg(line);
}

/// Returns the interactive shell: `$SHELL` or `/bin/sh`, `%COMSPEC%` or `cmd.exe` on Windows
pub fn interactive_shell(env: &dyn Env) -> String
{
    let (variable, fallback) = match env.os()
    {
        | Os::Windows => ("COMSPEC", "cmd.exe"),
        | _ => ("SHELL", "/bin/sh")
    };
    env.var(variable).map_or_else(|| fallback.to_string(), |value| value.to_string_lossy().into_owned())
}

/// Runs command lines in the background with the output captured
#[derive(Debug, Clone, Copy)]
pub struct OsRunner(pub Os);

impl Runner for OsRunner
{
    fn capture(&self, line: &str, cwd: &Utf8TypedPathBuf, stdout_only: bool) -> io::Result<Ran>
    {
        let mut command = shell_command(self.0, line);
        command.current_dir(cwd.as_str()).stdin(Stdio::null());
        if stdout_only == true
        {
            let output = command.stderr(Stdio::null()).output()?;
            return Ok(Ran { success: output.status.success(), output: output.stdout });
        }
        // One pipe for both streams keeps the order the program wrote them in.
        io::pipe().and_then(|(reader, writer)| writer.try_clone().map(|copy| (reader, copy, writer))).and_then(move |(mut reader, copy, writer)| {
            command.stdout(copy).stderr(writer);
            let spawned = command.spawn();
            // The command still holds the write ends; without dropping them the read never ends.
            drop(command);
            let mut output = Vec::new();
            spawned.and_then(|mut child| reader.read_to_end(&mut output).and_then(|_| child.wait())).map(|status| Ran { success: status.success(), output })
        })
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use mep_core::env::FixedEnv;

    use super::*;

    #[test]
    fn test_shell_command_lines()
    {
        let unix = shell_command(Os::Linux, "echo hi");
        assert_eq!(unix.get_program(), "sh");
        assert_eq!(unix.get_args().collect::<Vec<_>>(), ["-c", "echo hi"]);
        let windows = shell_command(Os::Windows, "dir");
        assert_eq!(windows.get_program(), "cmd");
        assert_eq!(windows.get_args().collect::<Vec<_>>(), ["/C", "dir"]);
    }

    #[test]
    fn test_interactive_shell_from_the_environment()
    {
        let env =
            |os, vars: &[(&str, &str)]| FixedEnv { os, vars: vars.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect(), ..FixedEnv::default() };
        assert_eq!(interactive_shell(&env(Os::Linux, &[("SHELL", "/bin/zsh")])), "/bin/zsh");
        assert_eq!(interactive_shell(&env(Os::MacOs, &[])), "/bin/sh");
        assert_eq!(interactive_shell(&env(Os::Windows, &[("COMSPEC", "C:\\cmd.exe")])), "C:\\cmd.exe");
        assert_eq!(interactive_shell(&env(Os::Windows, &[])), "cmd.exe");
    }

    #[cfg(unix)]
    #[test]
    fn test_runner_captures_both_streams_in_order()
    {
        let dir = tempfile::TempDir::new().unwrap();
        let runner = OsRunner(Os::Linux);
        let cwd = Os::Linux.path(&dir.path().to_string_lossy());
        let missing = cwd.join("missing");
        let ran = runner.capture("echo one; echo two >&2; pwd", &cwd, false).unwrap();
        let text = String::from_utf8(ran.output).unwrap();
        assert!(ran.success);
        assert!(text.starts_with("one\ntwo\n"), "{text}");
        let stdout = runner.capture("echo out; echo err >&2; exit 3", &cwd, true).unwrap();
        assert_eq!((stdout.success, stdout.output.as_slice()), (false, b"out\n".as_slice()));
        assert!(runner.capture("true", &missing, false).is_err());
        assert!(runner.capture("true", &missing, true).is_err());
    }
}
