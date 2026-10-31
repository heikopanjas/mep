//! Command table (IMPLEMENTATION_PLAN.md §6.5, §6.7; M guide p. 72; docs/spec/commands)

use crate::{
    commands::{clipboard, edit, files, misc, movement, search},
    editor::{Arg, Editor, Step}
};

/// Argument and behavior flags of a command (M guide p. 72, MSJ Fig. 6)
pub(crate) mod flags
{
    /// Accepts no argument
    pub const NOARG: u32 = 1 << 0;
    /// Accepts `arg` without text or movement
    pub const NULLARG: u32 = 1 << 1;
    /// Accepts typed text
    pub const TEXTARG: u32 = 1 << 2;
    /// Accepts a number (converted to lines unless `COUNT`)
    pub const NUMARG: u32 = 1 << 3;
    /// Accepts whole lines
    pub const LINEARG: u32 = 1 << 4;
    /// Accepts a stream; multi-line selections arrive as a stream if `BOXARG` and `LINEARG` do not take them
    pub const STREAMARG: u32 = 1 << 5;
    /// Accepts a box
    pub const BOXARG: u32 = 1 << 6;
    /// A one-line selection arrives as text
    pub const BOXSTR: u32 = 1 << 7;
    /// A nullarg arrives as text from the anchor to the end of the line
    pub const NULLEOL: u32 = 1 << 8;
    /// A nullarg arrives as text from the anchor to the next blank
    pub const NULLEOW: u32 = 1 << 9;
    /// Leaves meta set
    pub const KEEPMETA: u32 = 1 << 10;
    /// Cursor movement: extends a selection and takes no argument
    pub const CURSORFUNC: u32 = 1 << 11;
    /// Window movement
    pub const WINDOWFUNC: u32 = 1 << 12;
    /// Changes the file
    pub const MODIFIES: u32 = 1 << 13;
    /// A number is passed as a count, not converted to lines
    pub const COUNT: u32 = 1 << 14;
    /// Edits the text argument while one is being typed
    pub const TEXTEDIT: u32 = 1 << 15;
    /// Handles the argument state itself
    pub const RAW: u32 = 1 << 16;
}

use flags::*;

/// A command implementation
pub(crate) type Func = fn(&mut Editor, Arg) -> Step;

/// A built-in function
#[derive(Debug)]
pub(crate) struct Desc
{
    pub(crate) name:  &'static str,
    pub(crate) flags: u32,
    pub(crate) func:  Func
}

const MOVE: u32 = CURSORFUNC;
const WINDOW: u32 = NOARG | NULLARG | NUMARG | COUNT | WINDOWFUNC;
const BLOCK: u32 = NOARG | NULLARG | LINEARG | BOXARG | STREAMARG | NUMARG | MODIFIES;
const SEARCH: u32 = NOARG | NULLARG | NULLEOW | TEXTARG | BOXSTR;
const INSERT: u32 = NOARG | MODIFIES;

/// All built-in functions, sorted by name
const TABLE: &[Desc] = &[
    Desc { name: "arg", flags: RAW | KEEPMETA, func: misc::arg },
    Desc { name: "argcompile", flags: RAW, func: misc::not_available },
    Desc { name: "assign", flags: NULLARG | NULLEOL | TEXTARG | BOXSTR | LINEARG | BOXARG, func: misc::assign },
    Desc { name: "backtab", flags: MOVE, func: movement::backtab },
    Desc { name: "begline", flags: MOVE | TEXTEDIT, func: movement::begline },
    Desc { name: "cancel", flags: RAW, func: misc::cancel },
    Desc { name: "cdelete", flags: NOARG | MODIFIES | TEXTEDIT, func: edit::cdelete },
    Desc { name: "compile", flags: RAW, func: misc::not_available },
    Desc { name: "copy", flags: NOARG | NULLARG | BOXARG | LINEARG | STREAMARG | TEXTARG | NUMARG, func: clipboard::copy },
    Desc { name: "curdate", flags: INSERT, func: misc::curdate },
    Desc { name: "curday", flags: INSERT, func: misc::curday },
    Desc { name: "curfile", flags: INSERT, func: misc::curfile },
    Desc { name: "curfileext", flags: INSERT, func: misc::curfileext },
    Desc { name: "curfilenam", flags: INSERT, func: misc::curfilenam },
    Desc { name: "curtime", flags: INSERT, func: misc::curtime },
    Desc { name: "curuser", flags: INSERT, func: misc::curuser },
    Desc { name: "down", flags: MOVE, func: movement::down },
    Desc { name: "emacscdel", flags: NOARG | MODIFIES | TEXTEDIT, func: edit::emacscdel },
    Desc { name: "emacsnewl", flags: NOARG | MODIFIES, func: edit::emacsnewl },
    Desc { name: "endline", flags: MOVE | TEXTEDIT, func: movement::endline },
    Desc { name: "execute", flags: NULLARG | NULLEOL | TEXTARG | STREAMARG | LINEARG, func: misc::execute },
    Desc { name: "exit", flags: NOARG | NULLARG, func: files::exit },
    Desc { name: "graphic", flags: NOARG | MODIFIES | TEXTEDIT, func: edit::graphic },
    Desc { name: "help", flags: RAW, func: misc::not_available },
    Desc { name: "home", flags: MOVE, func: movement::home },
    Desc { name: "information", flags: RAW, func: misc::not_available },
    Desc { name: "initialize", flags: NOARG | NULLARG | NULLEOW | TEXTARG | BOXSTR, func: misc::initialize },
    Desc { name: "insertmode", flags: NOARG, func: edit::insertmode },
    Desc { name: "lasttext", flags: RAW | KEEPMETA, func: misc::lasttext },
    Desc { name: "ldelete", flags: BLOCK, func: edit::ldelete },
    Desc { name: "left", flags: MOVE | TEXTEDIT, func: movement::left },
    Desc { name: "linsert", flags: BLOCK, func: edit::linsert },
    Desc { name: "mark", flags: RAW, func: misc::not_available },
    Desc { name: "meta", flags: RAW | KEEPMETA, func: misc::meta },
    Desc { name: "mlines", flags: WINDOW, func: movement::mlines },
    Desc { name: "mpage", flags: WINDOW | BOXSTR, func: movement::mpage },
    Desc { name: "mpara", flags: MOVE, func: movement::mpara },
    Desc { name: "msearch", flags: SEARCH, func: search::msearch },
    Desc { name: "mword", flags: MOVE, func: movement::mword },
    Desc { name: "newline", flags: MOVE, func: movement::newline },
    Desc { name: "paste", flags: NOARG | NULLARG | NULLEOL | TEXTARG | BOXSTR | MODIFIES, func: clipboard::paste },
    Desc { name: "pbal", flags: NOARG | NULLARG | MODIFIES, func: edit::pbal },
    Desc { name: "plines", flags: WINDOW, func: movement::plines },
    Desc { name: "ppage", flags: WINDOW | BOXSTR, func: movement::ppage },
    Desc { name: "ppara", flags: MOVE, func: movement::ppara },
    Desc { name: "psearch", flags: SEARCH, func: search::psearch },
    Desc { name: "pword", flags: MOVE, func: movement::pword },
    Desc { name: "qreplace", flags: RAW, func: misc::not_available },
    Desc { name: "quote", flags: NOARG | MODIFIES | TEXTEDIT, func: edit::quote },
    Desc { name: "refresh", flags: NOARG | NULLARG, func: files::refresh },
    Desc { name: "replace", flags: RAW, func: misc::not_available },
    Desc { name: "restcur", flags: MOVE, func: movement::restcur },
    Desc { name: "right", flags: MOVE | TEXTEDIT, func: movement::right },
    Desc { name: "savecur", flags: MOVE, func: movement::savecur },
    Desc { name: "sdelete", flags: NOARG | NULLARG | STREAMARG | MODIFIES | TEXTEDIT, func: edit::sdelete },
    Desc { name: "setfile", flags: NOARG | NULLARG | TEXTARG | BOXSTR, func: files::setfile },
    Desc { name: "setwindow", flags: NOARG | NULLARG | WINDOWFUNC, func: movement::setwindow },
    Desc { name: "shell", flags: RAW, func: misc::not_available },
    Desc { name: "sinsert", flags: NOARG | NULLARG | STREAMARG | MODIFIES | TEXTEDIT, func: edit::sinsert },
    Desc { name: "tab", flags: MOVE, func: movement::tab },
    Desc { name: "unassigned", flags: RAW | KEEPMETA, func: misc::unassigned },
    Desc { name: "undo", flags: NOARG, func: edit::undo },
    Desc { name: "up", flags: MOVE, func: movement::up },
    Desc { name: "window", flags: RAW, func: misc::not_available }
];

/// Returns the built-in function named `name` (lowercase)
pub(crate) fn find(name: &str) -> Option<&'static Desc>
{
    TABLE.binary_search_by(|desc| desc.name.cmp(name)).ok().map(|index| &TABLE[index])
}

/// Returns `true` if `name` is a built-in function
pub fn is_function(name: &str) -> bool
{
    find(&name.to_lowercase()).is_some()
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_table_is_sorted_and_complete()
    {
        assert!(TABLE.windows(2).all(|pair| pair[0].name < pair[1].name));
        for name in ["arg", "window", "graphic", "unassigned", "curfileext"]
        {
            assert!(is_function(name), "{name}");
        }
        assert!(is_function("Psearch"));
        assert!(is_function("bogus") == false);
    }
}
