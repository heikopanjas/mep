//! Running commands on the terminal (docs/spec/commands/shell.md; M guide p. 108, p. 61)

use crate::{
    cells,
    editor::{After, Arg, Editor, Step}
};

/// `shell` (shell.md)
pub(crate) fn shell(ed: &mut Editor, arg: Arg) -> Step
{
    let commands: Vec<String> = match arg
    {
        | Arg::None =>
        {
            let file = ed.file();
            if ed.meta == false && file.modified == true && file.name.path().is_some()
            {
                return ed.save(ed.current, After::Shell);
            }
            return ed.run_shell(&[]);
        }
        | Arg::Line { first, last, .. } =>
            (first..=last.min(ed.last_line())).map(|line| ed.shown_line(line)).map(|line| String::from_utf8_lossy(&line).into_owned()).collect(),
        | Arg::Box { top, bottom, left, right, .. } => (top..=bottom)
            .map(|line| {
                let shown = ed.shown_line(line);
                String::from_utf8_lossy(&shown[cells::box_range(&shown, left, right)]).into_owned()
            })
            .collect(),
        | other => vec![other.into_text()]
    };
    let commands: Vec<String> = commands.iter().map(|command| command.trim_end().to_string()).filter(|command| command.is_empty() == false).collect();
    match commands.is_empty()
    {
        | true => ed.fail("Invalid argument"),
        | false => ed.run_shell(&commands)
    }
}

impl Editor
{
    /// Runs commands on the terminal, or the interactive shell when `commands` is empty, then repaints
    pub(crate) fn run_shell(&mut self, commands: &[String]) -> Step
    {
        let host = self.host.clone();
        let success = host.shell(commands, self.switches.askrtn);
        self.redraw = true;
        Step::Done(success)
    }
}
