//! The editor side of C extensions (docs/spec/extensions.md; M guide Chapter 8, MSJ Figs. 6–7)
//!
//! The core never loads libraries. The [`crate::host::Host`] loads them and
//! calls their functions; the functions call back through [`ExtApi`], which
//! the editor implements. Positions are 0-based; file handles are
//! [`crate::file::FileId::handle`] values, 0 for no file.

/// Argument types and behavior flags of an extension function, as in `ext.h`
pub mod flags
{
    pub use crate::cmd::flags::{
        BOXARG, BOXSTR, CURSORFUNC, KEEPMETA, LINEARG, MARKARG, MODIFIES, NOARG, NULLARG, NULLEOL, NULLEOW, NUMARG, STREAMARG, TEXTARG, WINDOWFUNC
    };

    /// The flags an extension may use
    pub const ALL: u32 =
        BOXARG | BOXSTR | CURSORFUNC | KEEPMETA | LINEARG | MARKARG | MODIFIES | NOARG | NULLARG | NULLEOL | NULLEOW | NUMARG | STREAMARG | TEXTARG | WINDOWFUNC;
}

/// The argument passed to an extension function (`ARG`, p. 75)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtArg
{
    /// No argument; the cursor
    None
    {
        /// Line
        y: usize,
        /// Column
        x: usize
    },
    /// `arg` alone; the anchor
    Null
    {
        /// Number of `arg` presses
        count: usize,
        /// Line
        y:     usize,
        /// Column
        x:     usize
    },
    /// Text
    Text
    {
        /// Number of `arg` presses
        count: usize,
        /// Line
        y:     usize,
        /// Column
        x:     usize,
        /// The text
        text:  String
    },
    /// Whole lines, inclusive
    Line
    {
        /// Number of `arg` presses
        count: usize,
        /// First line
        start: usize,
        /// Last line
        end:   usize
    },
    /// A stream; the end is not part of it
    Stream
    {
        /// Number of `arg` presses
        count:   usize,
        /// Start column
        x_start: usize,
        /// Start line
        y_start: usize,
        /// End column
        x_end:   usize,
        /// End line
        y_end:   usize
    },
    /// A box, all edges inclusive
    Box
    {
        /// Number of `arg` presses
        count:  usize,
        /// Left column
        left:   usize,
        /// Top line
        top:    usize,
        /// Right column
        right:  usize,
        /// Bottom line
        bottom: usize
    }
}

/// Type of an extension switch (`swiDesc`, p. 71)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchKind
{
    /// `name:` / `noname:`
    Boolean,
    /// A decimal number
    Numeric,
    /// Text handed to the extension
    Special
}

/// A new value for an extension switch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchValue
{
    /// On or off
    Boolean(bool),
    /// A number
    Numeric(i32),
    /// Text
    Special(String)
}

/// A function of a loaded extension
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtFunction
{
    /// Name used in assignments and macros
    pub name:  String,
    /// Argument types and behavior flags
    pub flags: u32,
    /// Identifies the function to the host
    pub id:    usize
}

/// A switch of a loaded extension
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtSwitch
{
    /// Name used in assignments
    pub name: String,
    /// Type
    pub kind: SwitchKind,
    /// Identifies the switch to the host
    pub id:   usize
}

/// What a loaded extension offers
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extension
{
    /// Identifies the library to the host, for `WhenLoaded`
    pub id:        usize,
    /// Its functions (`cmdTable`)
    pub functions: Vec<ExtFunction>,
    /// Its switches (`swiTable`)
    pub switches:  Vec<ExtSwitch>
}

/// The low-level editor functions extensions call (MSJ Fig. 7; docs/spec/extensions.md)
pub trait ExtApi
{
    /// `AddFile`
    fn add_file(&mut self, name: &str) -> usize;
    /// `BadArg`
    fn bad_arg(&mut self) -> bool;
    /// `CopyBox`
    fn copy_box(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2]);
    /// `CopyLine`
    fn copy_line(&mut self, src: usize, dst: usize, start: usize, end: usize, at: usize);
    /// `CopyStream`
    fn copy_stream(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2]);
    /// `DelBox`
    fn del_box(&mut self, file: usize, area: [usize; 4]);
    /// `DelFile`
    fn del_file(&mut self, file: usize);
    /// `DelLine`
    fn del_line(&mut self, file: usize, start: usize, end: usize);
    /// `DelStream`
    fn del_stream(&mut self, file: usize, area: [usize; 4]);
    /// `DoMessage`
    fn message(&mut self, text: &str) -> usize;
    /// `Display`
    fn display(&mut self);
    /// `fExecute`
    fn execute(&mut self, text: &str) -> bool;
    /// `FileLength`
    fn file_length(&mut self, file: usize) -> usize;
    /// `FileNameToHandle`
    fn file_handle(&mut self, name: &str, short: Option<&str>) -> usize;
    /// `FileRead`
    fn file_read(&mut self, name: &str, file: usize) -> bool;
    /// `FileWrite`
    fn file_write(&mut self, name: &str, file: usize) -> bool;
    /// `GetCursor`: (column, line)
    fn get_cursor(&mut self) -> (usize, usize);
    /// `GetLine`
    fn get_line(&mut self, line: usize, file: usize) -> Vec<u8>;
    /// `KbHook` (true) and `KbUnHook` (false)
    fn keyboard(&mut self, hooked: bool);
    /// `MoveCur`
    fn move_cursor(&mut self, x: usize, y: usize);
    /// `pFileToTop`
    fn to_top(&mut self, file: usize);
    /// `PutLine`
    fn put_line(&mut self, line: usize, text: &[u8], file: usize);
    /// `RemoveFile`
    fn remove_file(&mut self, file: usize);
    /// `ReadChar`
    fn read_char(&mut self) -> u32;
    /// `ReadCmd`
    fn read_command(&mut self) -> Option<String>;
    /// `Replace`
    fn replace_char(&mut self, ch: u8, x: usize, y: usize, file: usize, insert: bool) -> bool;
    /// `SetKey`
    fn set_key(&mut self, function: &str, key: &str) -> bool;
}
