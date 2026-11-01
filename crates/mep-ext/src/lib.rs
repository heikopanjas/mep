//! C extensions (docs/spec/extensions.md; M guide Chapter 8; MSJ "Programming Extensions")
//!
//! Loads extension libraries, reads their `cmdTable` and `swiTable`, calls
//! their functions with an `ARG` and gives them the low-level editor functions
//! of `c/ext.h` through the [`CALLBACKS`] table. This is the only crate with
//! unsafe code (IMPLEMENTATION_PLAN.md §4.1): everything here crosses the C ABI.
//!
//! While an extension function runs, the editor's [`ExtApi`] is reachable
//! from the callbacks through a thread-local pointer; outside such a call the
//! callbacks do nothing and return 0.

#![cfg_attr(test, feature(coverage_attribute))]
#![allow(clippy::bool_comparison)]

use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_int, c_uint, c_void},
    path::{Path, PathBuf},
    ptr
};

use libloading::Library;
use mep_core::ext::{ExtApi, ExtArg, ExtFunction, ExtSwitch, Extension, SwitchKind, SwitchValue, flags};

/// `LINE` and `COL` of `ext.h`
pub type Coord = i32;

/// `flagType` of `ext.h`: 0 is false, anything else true; the editor passes −1 for true
pub type Flag = c_int;

/// `PFILE` of `ext.h`: an editor file handle, 0 for none
pub type PFile = *mut c_void;

/// Size of the buffer `GetLine` fills, as `BUFLEN` in `ext.h`
pub const BUFLEN: usize = 512;

/// `struct noargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NoArg
{
    pub y: Coord,
    pub x: Coord
}

/// `struct textargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextArg
{
    pub count: c_int,
    pub y:     Coord,
    pub x:     Coord,
    pub text:  *const c_char
}

/// `struct nullargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NullArg
{
    pub count: c_int,
    pub y:     Coord,
    pub x:     Coord
}

/// `struct lineargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LineArg
{
    pub count: c_int,
    pub start: Coord,
    pub end:   Coord
}

/// `struct streamargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StreamArg
{
    pub count:   c_int,
    pub x_start: Coord,
    pub y_start: Coord,
    pub x_end:   Coord,
    pub y_end:   Coord
}

/// `struct boxargType`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BoxArg
{
    pub count:  c_int,
    pub left:   Coord,
    pub top:    Coord,
    pub right:  Coord,
    pub bottom: Coord
}

/// The union in `struct argType`
#[repr(C)]
#[derive(Clone, Copy)]
pub union ArgData
{
    pub noarg:     NoArg,
    pub textarg:   TextArg,
    pub nullarg:   NullArg,
    pub linearg:   LineArg,
    pub streamarg: StreamArg,
    pub boxarg:    BoxArg
}

/// `ARG`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Arg
{
    pub arg_type: c_int,
    pub arg:      ArgData
}

/// An extension function: `flagType func(unsigned int argData, ARG *pArg, flagType fMeta)`
pub type Command = unsafe extern "C" fn(c_uint, *mut Arg, Flag) -> Flag;

/// A `SWI_SPECIAL` switch function: `int func(char *value)`
pub type SwitchFunction = unsafe extern "C" fn(*const c_char) -> c_int;

/// `struct cmdDesc`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CmdDesc
{
    pub name:     *const c_char,
    pub func:     Option<Command>,
    pub reserved: *mut c_void,
    pub flags:    c_uint
}

/// `struct swiDesc`
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SwiDesc
{
    pub name: *const c_char,
    pub act:  *mut c_void,
    pub kind: c_int
}

/// `SWI_BOOLEAN`
pub const SWI_BOOLEAN: c_int = 0;
/// `SWI_NUMERIC`
pub const SWI_NUMERIC: c_int = 1;
/// `SWI_SPECIAL`
pub const SWI_SPECIAL: c_int = 2;

/// `struct MepCallbacks`: the table `exthdr.c` forwards the low-level functions through
#[repr(C)]
#[allow(non_snake_case)]
pub struct Callbacks
{
    pub version:          c_int,
    pub AddFile:          unsafe extern "C" fn(*const c_char) -> PFile,
    pub BadArg:           unsafe extern "C" fn() -> Flag,
    pub CopyBox:          unsafe extern "C" fn(PFile, PFile, Coord, Coord, Coord, Coord, Coord, Coord),
    pub CopyLine:         unsafe extern "C" fn(PFile, PFile, Coord, Coord, Coord),
    pub CopyStream:       unsafe extern "C" fn(PFile, PFile, Coord, Coord, Coord, Coord, Coord, Coord),
    pub DelBox:           unsafe extern "C" fn(PFile, Coord, Coord, Coord, Coord),
    pub DelFile:          unsafe extern "C" fn(PFile),
    pub DelLine:          unsafe extern "C" fn(PFile, Coord, Coord),
    pub DelStream:        unsafe extern "C" fn(PFile, Coord, Coord, Coord, Coord),
    pub DoMessage:        unsafe extern "C" fn(*const c_char) -> c_int,
    pub Display:          unsafe extern "C" fn(),
    pub fExecute:         unsafe extern "C" fn(*const c_char) -> Flag,
    pub FileLength:       unsafe extern "C" fn(PFile) -> Coord,
    pub FileNameToHandle: unsafe extern "C" fn(*const c_char, *const c_char) -> PFile,
    pub FileRead:         unsafe extern "C" fn(*const c_char, PFile) -> Flag,
    pub FileWrite:        unsafe extern "C" fn(*const c_char, PFile) -> Flag,
    pub GetCursor:        unsafe extern "C" fn(*mut Coord, *mut Coord),
    pub GetLine:          unsafe extern "C" fn(Coord, *mut c_char, PFile) -> c_int,
    pub KbHook:           unsafe extern "C" fn(),
    pub KbUnHook:         unsafe extern "C" fn(),
    pub MoveCur:          unsafe extern "C" fn(Coord, Coord),
    pub pFileToTop:       unsafe extern "C" fn(PFile),
    pub PutLine:          unsafe extern "C" fn(Coord, *const c_char, PFile),
    pub RemoveFile:       unsafe extern "C" fn(PFile),
    pub ReadChar:         unsafe extern "C" fn() -> c_int,
    pub ReadCmd:          unsafe extern "C" fn() -> *const c_char,
    pub Replace:          unsafe extern "C" fn(c_char, Coord, Coord, PFile, Flag) -> Flag,
    pub SetKey:           unsafe extern "C" fn(*const c_char, *const c_char) -> Flag
}

/// `MEP_CALLBACKS_VERSION` in `ext.h`
pub const CALLBACKS_VERSION: c_int = 1;

/// The low-level editor functions handed to every loaded extension
pub static CALLBACKS: Callbacks = Callbacks {
    version:          CALLBACKS_VERSION,
    AddFile:          add_file,
    BadArg:           bad_arg,
    CopyBox:          copy_box,
    CopyLine:         copy_line,
    CopyStream:       copy_stream,
    DelBox:           del_box,
    DelFile:          del_file,
    DelLine:          del_line,
    DelStream:        del_stream,
    DoMessage:        do_message,
    Display:          display,
    fExecute:         execute,
    FileLength:       file_length,
    FileNameToHandle: file_name_to_handle,
    FileRead:         file_read,
    FileWrite:        file_write,
    GetCursor:        get_cursor,
    GetLine:          get_line,
    KbHook:           kb_hook,
    KbUnHook:         kb_unhook,
    MoveCur:          move_cur,
    pFileToTop:       file_to_top,
    PutLine:          put_line,
    RemoveFile:       remove_file,
    ReadChar:         read_char,
    ReadCmd:          read_cmd,
    Replace:          replace,
    SetKey:           set_key
};

thread_local! {
    /// The editor while an extension runs
    static API: Cell<Option<*mut dyn ExtApi>> = const { Cell::new(None) };
    /// The name `ReadCmd` returned last; it stays valid until the next call
    static COMMAND: RefCell<CString> = RefCell::new(CString::default());
}

/// Makes `api` reachable from the callbacks while `work` runs; nested calls restore the outer editor
fn enter<T>(api: &mut dyn ExtApi, work: impl FnOnce() -> T) -> T
{
    // SAFETY: only the lifetime is erased; the pointer is removed again before `api` goes out of scope.
    let api: *mut (dyn ExtApi + 'static) = unsafe { std::mem::transmute::<*mut dyn ExtApi, *mut (dyn ExtApi + 'static)>(api) };
    let outer = API.replace(Some(api));
    let result = work();
    API.set(outer);
    result
}

/// Runs a callback against the editor; `none` outside an extension call
fn with_api<T>(none: T, work: impl FnOnce(&mut dyn ExtApi) -> T) -> T
{
    match API.get()
    {
        // SAFETY: the pointer was set by `enter` and the editor outlives the extension call.
        | Some(api) => work(unsafe { &mut *api }),
        | None => none
    }
}

/// Reads a C string; null is empty
///
/// # Safety
///
/// `text` is null or points to a zero-terminated string.
unsafe fn text(text: *const c_char) -> String
{
    match text.is_null()
    {
        | true => String::new(),
        // SAFETY: the caller passes a zero-terminated string.
        | false => unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    }
}

/// Turns a C coordinate into a position; negative values are 0
fn at(value: Coord) -> usize
{
    usize::try_from(value).unwrap_or(0)
}

/// Turns a position into a C coordinate
fn coord(value: usize) -> Coord
{
    Coord::try_from(value).unwrap_or(Coord::MAX)
}

/// Turns a C flag into a `bool`
fn truth(flag: Flag) -> bool
{
    flag != 0
}

/// Turns a `bool` into a C flag: −1 or 0, as the 1988 editor did (p. 74)
fn flag(value: bool) -> Flag
{
    -Flag::from(value)
}

/// Turns an editor file handle into a `PFILE`
fn pfile(handle: usize) -> PFile
{
    ptr::without_provenance_mut(handle)
}

/// Turns a `PFILE` into an editor file handle
fn handle(file: PFile) -> usize
{
    file.addr()
}

unsafe extern "C" fn add_file(name: *const c_char) -> PFile
{
    // SAFETY: extensions pass zero-terminated strings.
    let name = unsafe { text(name) };
    pfile(with_api(0, |api| api.add_file(&name)))
}

unsafe extern "C" fn bad_arg() -> Flag
{
    flag(with_api(false, |api| api.bad_arg()))
}

unsafe extern "C" fn copy_box(src: PFile, dst: PFile, left: Coord, top: Coord, right: Coord, bottom: Coord, x: Coord, y: Coord)
{
    with_api((), |api| api.copy_box(handle(src), handle(dst), [at(left), at(top), at(right), at(bottom)], [at(x), at(y)]));
}

unsafe extern "C" fn copy_line(src: PFile, dst: PFile, start: Coord, end: Coord, y: Coord)
{
    with_api((), |api| api.copy_line(handle(src), handle(dst), at(start), at(end), at(y)));
}

unsafe extern "C" fn copy_stream(src: PFile, dst: PFile, x_start: Coord, y_start: Coord, x_end: Coord, y_end: Coord, x: Coord, y: Coord)
{
    with_api((), |api| api.copy_stream(handle(src), handle(dst), [at(x_start), at(y_start), at(x_end), at(y_end)], [at(x), at(y)]));
}

unsafe extern "C" fn del_box(file: PFile, left: Coord, top: Coord, right: Coord, bottom: Coord)
{
    with_api((), |api| api.del_box(handle(file), [at(left), at(top), at(right), at(bottom)]));
}

unsafe extern "C" fn del_file(file: PFile)
{
    with_api((), |api| api.del_file(handle(file)));
}

unsafe extern "C" fn del_line(file: PFile, start: Coord, end: Coord)
{
    with_api((), |api| api.del_line(handle(file), at(start), at(end)));
}

unsafe extern "C" fn del_stream(file: PFile, x_start: Coord, y_start: Coord, x_end: Coord, y_end: Coord)
{
    with_api((), |api| api.del_stream(handle(file), [at(x_start), at(y_start), at(x_end), at(y_end)]));
}

unsafe extern "C" fn do_message(message: *const c_char) -> c_int
{
    // SAFETY: extensions pass zero-terminated strings.
    let message = unsafe { text(message) };
    c_int::try_from(with_api(0, |api| api.message(&message))).unwrap_or(c_int::MAX)
}

unsafe extern "C" fn display()
{
    with_api((), |api| api.display());
}

unsafe extern "C" fn execute(macro_text: *const c_char) -> Flag
{
    // SAFETY: extensions pass zero-terminated strings.
    let macro_text = unsafe { text(macro_text) };
    flag(with_api(false, |api| api.execute(&macro_text)))
}

unsafe extern "C" fn file_length(file: PFile) -> Coord
{
    coord(with_api(0, |api| api.file_length(handle(file))))
}

unsafe extern "C" fn file_name_to_handle(name: *const c_char, short: *const c_char) -> PFile
{
    // SAFETY: extensions pass zero-terminated strings or null.
    let (name, short) = unsafe { (text(name), (short.is_null() == false).then(|| text(short))) };
    pfile(with_api(0, |api| api.file_handle(&name, short.as_deref())))
}

unsafe extern "C" fn file_read(name: *const c_char, file: PFile) -> Flag
{
    // SAFETY: extensions pass zero-terminated strings.
    let name = unsafe { text(name) };
    flag(with_api(false, |api| api.file_read(&name, handle(file))))
}

unsafe extern "C" fn file_write(name: *const c_char, file: PFile) -> Flag
{
    // SAFETY: extensions pass zero-terminated strings.
    let name = unsafe { text(name) };
    flag(with_api(false, |api| api.file_write(&name, handle(file))))
}

unsafe extern "C" fn get_cursor(x: *mut Coord, y: *mut Coord)
{
    let (col, line) = with_api((0, 0), |api| api.get_cursor());
    // SAFETY: extensions pass pointers to their variables or null.
    unsafe {
        if let Some(x) = x.as_mut()
        {
            *x = coord(col);
        }
        if let Some(y) = y.as_mut()
        {
            *y = coord(line);
        }
    }
}

unsafe extern "C" fn get_line(line: Coord, buf: *mut c_char, file: PFile) -> c_int
{
    let mut bytes = with_api(Vec::new(), |api| api.get_line(at(line), handle(file)));
    bytes.truncate(BUFLEN - 1);
    if buf.is_null() == false
    {
        // SAFETY: extensions pass a buffer of `BUFLEN` bytes (p. 79).
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buf, bytes.len());
            *buf.add(bytes.len()) = 0;
        }
    }
    c_int::try_from(bytes.len()).unwrap_or(c_int::MAX)
}

unsafe extern "C" fn kb_hook()
{
    with_api((), |api| api.keyboard(true));
}

unsafe extern "C" fn kb_unhook()
{
    with_api((), |api| api.keyboard(false));
}

unsafe extern "C" fn move_cur(x: Coord, y: Coord)
{
    with_api((), |api| api.move_cursor(at(x), at(y)));
}

unsafe extern "C" fn file_to_top(file: PFile)
{
    with_api((), |api| api.to_top(handle(file)));
}

unsafe extern "C" fn put_line(line: Coord, buf: *const c_char, file: PFile)
{
    // SAFETY: extensions pass zero-terminated strings.
    let bytes = match buf.is_null()
    {
        | true => Vec::new(),
        | false => unsafe { CStr::from_ptr(buf) }.to_bytes().to_vec()
    };
    with_api((), |api| api.put_line(at(line), &bytes, handle(file)));
}

unsafe extern "C" fn remove_file(file: PFile)
{
    with_api((), |api| api.remove_file(handle(file)));
}

unsafe extern "C" fn read_char() -> c_int
{
    c_int::try_from(with_api(0, |api| api.read_char())).unwrap_or(0)
}

unsafe extern "C" fn read_cmd() -> *const c_char
{
    match with_api(None, |api| api.read_command())
    {
        | Some(name) => COMMAND.with(|command| {
            *command.borrow_mut() = CString::new(name).unwrap_or_default();
            command.borrow().as_ptr()
        }),
        | None => ptr::null()
    }
}

unsafe extern "C" fn replace(ch: c_char, x: Coord, y: Coord, file: PFile, insert: Flag) -> Flag
{
    flag(with_api(false, |api| api.replace_char(ch as u8, at(x), at(y), handle(file), truth(insert))))
}

unsafe extern "C" fn set_key(name: *const c_char, key: *const c_char) -> Flag
{
    // SAFETY: extensions pass zero-terminated strings.
    let (name, key) = unsafe { (text(name), text(key)) };
    flag(with_api(false, |api| api.set_key(&name, &key)))
}

/// The tables of one library, read by [`Extensions::register`]
#[derive(Debug, Clone, Copy)]
pub struct Tables
{
    /// `cmdTable`
    pub commands: *const CmdDesc,
    /// `swiTable`
    pub switches: *const SwiDesc,
    /// `WhenLoaded`
    pub init:     unsafe extern "C" fn()
}

/// The loaded extensions
#[derive(Debug, Default)]
pub struct Extensions
{
    libraries: RefCell<Vec<Library>>,
    inits:     RefCell<Vec<unsafe extern "C" fn()>>,
    functions: RefCell<Vec<Command>>,
    /// The `act` pointers of the switches; the editor sends values of the declared type
    switches:  RefCell<Vec<*mut c_void>>
}

impl Extensions
{
    /// Loads a library, fills its `MepCallbacks` and registers its tables
    ///
    /// # Errors
    ///
    /// Returns why the library cannot be loaded or lacks a required symbol.
    // ORACLE: EXT-1
    pub fn load(&self, path: &Path) -> Result<Extension, String>
    {
        let library = open(path)?;
        // SAFETY: only the addresses are read here.
        let symbol = |name: &[u8]| unsafe { library.get::<*mut c_void>(name) }.ok().map(|symbol| *symbol);
        let (Some(callbacks), Some(init), Some(commands), Some(switches)) = (symbol(b"MepCallbacks"), symbol(b"WhenLoaded"), symbol(b"cmdTable"), symbol(b"swiTable"))
        else
        {
            return Err("not an extension: it needs MepCallbacks (exthdr.c), WhenLoaded, cmdTable and swiTable".into());
        };
        // SAFETY: the symbols have the types `ext.h` declares; the library stays loaded until the editor ends.
        let tables = unsafe {
            *callbacks.cast::<*const Callbacks>() = &raw const CALLBACKS;
            Tables { commands: commands.cast(), switches: switches.cast(), init: std::mem::transmute::<*mut c_void, unsafe extern "C" fn()>(init) }
        };
        self.libraries.borrow_mut().push(library);
        // SAFETY: the tables come from a library written against `ext.h`.
        Ok(unsafe { self.register(tables) })
    }

    /// Reads the tables of a library and keeps its functions and switches
    ///
    /// # Safety
    ///
    /// The tables follow `ext.h`: both end with an entry whose name is null, names are
    /// zero-terminated, switch pointers fit their type, and everything lives as long as `self`.
    pub unsafe fn register(&self, tables: Tables) -> Extension
    {
        let mut inits = self.inits.borrow_mut();
        let mut extension = Extension { id: inits.len(), ..Extension::default() };
        inits.push(tables.init);
        let mut functions = self.functions.borrow_mut();
        let mut entry = tables.commands;
        // SAFETY: the caller guarantees a terminated table.
        while let Some(desc) = unsafe { entry.as_ref() }.filter(|desc| desc.name.is_null() == false)
        {
            if let Some(func) = desc.func
            {
                // SAFETY: the name is zero-terminated.
                let name = unsafe { text(desc.name) };
                extension.functions.push(ExtFunction { name, flags: desc.flags & flags::ALL, id: functions.len() });
                functions.push(func);
            }
            // SAFETY: the table goes on up to its terminating entry.
            entry = unsafe { entry.add(1) };
        }
        let mut switches = self.switches.borrow_mut();
        let mut entry = tables.switches;
        // SAFETY: the caller guarantees a terminated table.
        while let Some(desc) = unsafe { entry.as_ref() }.filter(|desc| desc.name.is_null() == false)
        {
            let kind = match desc.kind
            {
                | SWI_BOOLEAN => Some(SwitchKind::Boolean),
                | SWI_NUMERIC => Some(SwitchKind::Numeric),
                | SWI_SPECIAL => Some(SwitchKind::Special),
                | _ => None
            };
            if let Some(kind) = kind.filter(|_| desc.act.is_null() == false)
            {
                // SAFETY: the name is zero-terminated.
                let name = unsafe { text(desc.name) };
                extension.switches.push(ExtSwitch { name, kind, id: switches.len() });
                switches.push(desc.act);
            }
            // SAFETY: the table goes on up to its terminating entry.
            entry = unsafe { entry.add(1) };
        }
        extension
    }

    /// Runs `WhenLoaded` of a registered library
    pub fn init(&self, library: usize, api: &mut dyn ExtApi)
    {
        let init = self.inits.borrow()[library];
        // SAFETY: `WhenLoaded` takes nothing (p. 74).
        enter(api, || unsafe { init() });
    }

    /// Calls an extension function with its argument (p. 74)
    pub fn call(&self, id: usize, arg: &ExtArg, key: u32, meta: bool, api: &mut dyn ExtApi) -> bool
    {
        let func = self.functions.borrow()[id];
        // The text must outlive the call; NUL bytes inside cannot reach C.
        let text = match arg
        {
            | ExtArg::Text { text, .. } => CString::new(text.replace('\0', "")).unwrap_or_default(),
            | _ => CString::default()
        };
        let mut arg = c_arg(arg, &text);
        // SAFETY: the function follows `ext.h`; `arg` and `text` live during the call.
        truth(enter(api, || unsafe { func(key, &raw mut arg, flag(meta)) }))
    }

    /// Sets an extension switch; `false` if a switch function rejects the value
    pub fn set_switch(&self, id: usize, value: &SwitchValue, api: &mut dyn ExtApi) -> bool
    {
        let act = self.switches.borrow()[id];
        match value
        {
            | SwitchValue::Boolean(on) =>
            {
                // SAFETY: a Boolean switch points to an `int` (p. 71).
                unsafe { *act.cast::<c_int>() = flag(*on) };
                true
            }
            | SwitchValue::Numeric(number) =>
            {
                // SAFETY: a numeric switch points to an `int` (p. 71).
                unsafe { *act.cast::<c_int>() = *number };
                true
            }
            | SwitchValue::Special(value) =>
            {
                let value = CString::new(value.replace('\0', "")).unwrap_or_default();
                // SAFETY: a special switch points to a function taking the text (p. 71).
                let func = unsafe { std::mem::transmute::<*mut c_void, SwitchFunction>(act) };
                // SAFETY: the function follows `ext.h`; the text lives during the call.
                truth(enter(api, || unsafe { func(value.as_ptr()) }))
            }
        }
    }
}

/// Builds the C argument; `text` holds the text of a `TEXTARG`
fn c_arg(arg: &ExtArg, text: &CString) -> Arg
{
    let count = |count: &usize| c_int::try_from(*count).unwrap_or(c_int::MAX);
    match arg
    {
        | ExtArg::None { y, x } => Arg { arg_type: flags::NOARG as c_int, arg: ArgData { noarg: NoArg { y: coord(*y), x: coord(*x) } } },
        | ExtArg::Null { count: n, y, x } =>
            Arg { arg_type: flags::NULLARG as c_int, arg: ArgData { nullarg: NullArg { count: count(n), y: coord(*y), x: coord(*x) } } },
        | ExtArg::Text { count: n, y, x, .. } => Arg {
            arg_type: flags::TEXTARG as c_int,
            arg:      ArgData { textarg: TextArg { count: count(n), y: coord(*y), x: coord(*x), text: text.as_ptr() } }
        },
        | ExtArg::Line { count: n, start, end } =>
            Arg { arg_type: flags::LINEARG as c_int, arg: ArgData { linearg: LineArg { count: count(n), start: coord(*start), end: coord(*end) } } },
        | ExtArg::Stream { count: n, x_start, y_start, x_end, y_end } => Arg {
            arg_type: flags::STREAMARG as c_int,
            arg:      ArgData {
                streamarg: StreamArg { count: count(n), x_start: coord(*x_start), y_start: coord(*y_start), x_end: coord(*x_end), y_end: coord(*y_end) }
            }
        },
        | ExtArg::Box { count: n, left, top, right, bottom } => Arg {
            arg_type: flags::BOXARG as c_int,
            arg:      ArgData { boxarg: BoxArg { count: count(n), left: coord(*left), top: coord(*top), right: coord(*right), bottom: coord(*bottom) } }
        }
    }
}

/// Opens a library; a name without an extension is also tried with the platform's library extension
fn open(path: &Path) -> Result<Library, String>
{
    let candidates: Vec<PathBuf> = match path.extension()
    {
        | Some(_) => vec![path.to_path_buf()],
        | None => vec![path.to_path_buf(), path.with_extension(std::env::consts::DLL_EXTENSION)]
    };
    let mut error = String::new();
    for candidate in candidates
    {
        // SAFETY: loading runs the library's initializers; the user asked for this library with `load:`.
        match unsafe { Library::new(&candidate) }
        {
            | Ok(library) => return Ok(library),
            | Err(failure) => error = failure.to_string()
        }
    }
    Err(error)
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    /// Records every callback and answers with fixed values
    #[derive(Default)]
    struct Fake
    {
        calls: Vec<String>
    }

    impl ExtApi for Fake
    {
        fn add_file(&mut self, name: &str) -> usize
        {
            self.calls.push(format!("add {name}"));
            5
        }

        fn bad_arg(&mut self) -> bool
        {
            self.calls.push("badarg".into());
            false
        }

        fn copy_box(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2])
        {
            self.calls.push(format!("copybox {src} {dst} {from:?} {to:?}"));
        }

        fn copy_line(&mut self, src: usize, dst: usize, start: usize, end: usize, at: usize)
        {
            self.calls.push(format!("copyline {src} {dst} {start} {end} {at}"));
        }

        fn copy_stream(&mut self, src: usize, dst: usize, from: [usize; 4], to: [usize; 2])
        {
            self.calls.push(format!("copystream {src} {dst} {from:?} {to:?}"));
        }

        fn del_box(&mut self, file: usize, area: [usize; 4])
        {
            self.calls.push(format!("delbox {file} {area:?}"));
        }

        fn del_file(&mut self, file: usize)
        {
            self.calls.push(format!("delfile {file}"));
        }

        fn del_line(&mut self, file: usize, start: usize, end: usize)
        {
            self.calls.push(format!("delline {file} {start} {end}"));
        }

        fn del_stream(&mut self, file: usize, area: [usize; 4])
        {
            self.calls.push(format!("delstream {file} {area:?}"));
        }

        fn message(&mut self, text: &str) -> usize
        {
            self.calls.push(format!("message {text}"));
            text.len()
        }

        fn display(&mut self)
        {
            self.calls.push("display".into());
        }

        fn execute(&mut self, text: &str) -> bool
        {
            self.calls.push(format!("execute {text}"));
            true
        }

        fn file_length(&mut self, file: usize) -> usize
        {
            file * 10
        }

        fn file_handle(&mut self, name: &str, short: Option<&str>) -> usize
        {
            self.calls.push(format!("handle {name} {short:?}"));
            3
        }

        fn file_read(&mut self, name: &str, file: usize) -> bool
        {
            self.calls.push(format!("read {name} {file}"));
            true
        }

        fn file_write(&mut self, name: &str, file: usize) -> bool
        {
            self.calls.push(format!("write {name} {file}"));
            false
        }

        fn get_cursor(&mut self) -> (usize, usize)
        {
            (4, 9)
        }

        fn get_line(&mut self, line: usize, file: usize) -> Vec<u8>
        {
            match file
            {
                | 1 => b"x".repeat(BUFLEN + 3),
                | _ => format!("line {line}").into_bytes()
            }
        }

        fn keyboard(&mut self, hooked: bool)
        {
            self.calls.push(format!("keyboard {hooked}"));
        }

        fn move_cursor(&mut self, x: usize, y: usize)
        {
            self.calls.push(format!("move {x} {y}"));
        }

        fn to_top(&mut self, file: usize)
        {
            self.calls.push(format!("top {file}"));
        }

        fn put_line(&mut self, line: usize, text: &[u8], file: usize)
        {
            self.calls.push(format!("putline {line} {} {file}", String::from_utf8_lossy(text)));
        }

        fn remove_file(&mut self, file: usize)
        {
            self.calls.push(format!("remove {file}"));
        }

        fn read_char(&mut self) -> u32
        {
            u32::from('q')
        }

        fn read_command(&mut self) -> Option<String>
        {
            self.calls.push("readcmd".into());
            (self.calls.len() % 2 == 1).then(|| "psearch".to_string())
        }

        fn replace_char(&mut self, ch: u8, x: usize, y: usize, file: usize, insert: bool) -> bool
        {
            self.calls.push(format!("replace {} {x} {y} {file} {insert}", ch as char));
            true
        }

        fn set_key(&mut self, function: &str, key: &str) -> bool
        {
            self.calls.push(format!("setkey {function} {key}"));
            true
        }
    }

    fn c(text: &str) -> CString
    {
        CString::new(text).unwrap()
    }

    #[test]
    fn test_callbacks_reach_the_editor()
    {
        let mut fake = Fake::default();
        let (name, other) = (c("a.txt"), c("b"));
        let file = pfile(2);
        let results = enter(&mut fake, || unsafe {
            let calls = &CALLBACKS;
            let mut buf = [0 as c_char; BUFLEN];
            let (mut x, mut y) = (0, 0);
            (calls.GetCursor)(&raw mut x, &raw mut y);
            (calls.GetCursor)(ptr::null_mut(), ptr::null_mut());
            let length = (calls.GetLine)(1, buf.as_mut_ptr(), file);
            let line = CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
            let long = (calls.GetLine)(0, buf.as_mut_ptr(), pfile(1));
            let unbuffered = (calls.GetLine)(0, ptr::null_mut(), file);
            (calls.CopyBox)(file, pfile(3), 1, 2, 3, 4, 5, 6);
            (calls.CopyLine)(ptr::null_mut(), file, 0, -1, 2);
            (calls.CopyStream)(file, file, 1, 2, 3, 4, 5, 6);
            (calls.DelBox)(file, 1, 2, 3, 4);
            (calls.DelFile)(file);
            (calls.DelLine)(file, 1, 2);
            (calls.DelStream)(file, 1, 2, 3, 4);
            (calls.Display)();
            (calls.KbHook)();
            (calls.KbUnHook)();
            (calls.MoveCur)(7, 8);
            (calls.pFileToTop)(file);
            (calls.PutLine)(3, name.as_ptr(), file);
            (calls.PutLine)(4, ptr::null(), file);
            (calls.RemoveFile)(file);
            let first = (calls.ReadCmd)();
            let second = CStr::from_ptr((calls.ReadCmd)()).to_string_lossy().into_owned();
            vec![
                format!("{x} {y} {length} {line} {long} {unbuffered}"),
                format!("{:?}", (calls.AddFile)(name.as_ptr()).addr()),
                format!("{}", (calls.BadArg)()),
                format!("{}", (calls.DoMessage)(name.as_ptr())),
                format!("{}", (calls.fExecute)(name.as_ptr())),
                format!("{}", (calls.FileLength)(file)),
                format!("{:?}", (calls.FileNameToHandle)(name.as_ptr(), other.as_ptr()).addr()),
                format!("{:?}", (calls.FileNameToHandle)(ptr::null(), ptr::null()).addr()),
                format!("{}", (calls.FileRead)(name.as_ptr(), file)),
                format!("{}", (calls.FileWrite)(name.as_ptr(), file)),
                format!("{}", (calls.ReadChar)()),
                format!("{} {second}", first.is_null()),
                format!("{}", (calls.Replace)(b'z' as c_char, 1, 2, file, -1)),
                format!("{}", (calls.SetKey)(name.as_ptr(), other.as_ptr())),
                format!("{}", calls.version),
            ]
        });
        assert_eq!(results, ["4 9 6 line 1 511 6", "5", "0", "5", "-1", "20", "3", "3", "-1", "0", "113", "true psearch", "-1", "-1", "1"]);
        assert_eq!(fake.calls, [
            "copybox 2 3 [1, 2, 3, 4] [5, 6]", "copyline 0 2 0 0 2", "copystream 2 2 [1, 2, 3, 4] [5, 6]", "delbox 2 [1, 2, 3, 4]", "delfile 2", "delline 2 1 2",
            "delstream 2 [1, 2, 3, 4]", "display", "keyboard true", "keyboard false", "move 7 8", "top 2", "putline 3 a.txt 2", "putline 4  2", "remove 2", "readcmd",
            "readcmd", "add a.txt", "badarg", "message a.txt", "execute a.txt", "handle a.txt Some(\"b\")", "handle  None", "read a.txt 2", "write a.txt 2",
            "replace z 1 2 2 true", "setkey a.txt b"
        ]);
    }

    #[test]
    fn test_callbacks_outside_an_extension_call_do_nothing()
    {
        unsafe {
            let calls = &CALLBACKS;
            let (mut x, mut y) = (5, 5);
            (calls.GetCursor)(&raw mut x, &raw mut y);
            assert_eq!((x, y), (0, 0));
            assert!((calls.AddFile)(ptr::null()).is_null());
            assert_eq!((calls.ReadChar)(), 0);
            assert!((calls.ReadCmd)().is_null());
            assert_eq!((calls.FileLength)(pfile(1)), 0);
        }
        assert_eq!((at(-3), coord(usize::MAX), flag(true), flag(false)), (0, Coord::MAX, -1, 0));
    }

    static mut NUMBER: c_int = 0;
    static mut BOOLEAN: c_int = 0;

    unsafe extern "C" fn special(value: *const c_char) -> c_int
    {
        let value = unsafe { text(value) };
        with_api(0, |api| api.message(&value));
        c_int::from(value != "bad")
    }

    unsafe extern "C" fn command(key: c_uint, arg: *mut Arg, meta: Flag) -> Flag
    {
        let arg = unsafe { *arg };
        let fields = unsafe {
            match arg.arg_type as u32
            {
                | flags::NOARG => format!("noarg {} {}", arg.arg.noarg.y, arg.arg.noarg.x),
                | flags::NULLARG => format!("nullarg {} {} {}", arg.arg.nullarg.count, arg.arg.nullarg.y, arg.arg.nullarg.x),
                | flags::TEXTARG => format!("textarg {} {} {} {}", arg.arg.textarg.count, arg.arg.textarg.y, arg.arg.textarg.x, text(arg.arg.textarg.text)),
                | flags::LINEARG => format!("linearg {} {} {}", arg.arg.linearg.count, arg.arg.linearg.start, arg.arg.linearg.end),
                | flags::STREAMARG =>
                {
                    let stream = arg.arg.streamarg;
                    format!("streamarg {} {} {} {} {}", stream.count, stream.x_start, stream.y_start, stream.x_end, stream.y_end)
                }
                | _ =>
                {
                    let block = arg.arg.boxarg;
                    format!("boxarg {} {} {} {} {}", block.count, block.left, block.top, block.right, block.bottom)
                }
            }
        };
        with_api(0, |api| api.message(&format!("{fields} key {key} meta {meta}")));
        flag(arg.arg_type as u32 != flags::BOXARG)
    }

    unsafe extern "C" fn when_loaded()
    {
        with_api(false, |api| api.set_key("probe", "alt+p"));
    }

    #[test]
    fn test_register_call_switches_and_init()
    {
        let names = [c("probe"), c("skipped"), c("flag"), c("number"), c("text"), c("odd"), c("nothing")];
        let commands = [
            CmdDesc { name: names[0].as_ptr(), func: Some(command), reserved: ptr::null_mut(), flags: flags::NOARG | 1 << 16 },
            CmdDesc { name: names[1].as_ptr(), func: None, reserved: ptr::null_mut(), flags: 0 },
            CmdDesc { name: ptr::null(), func: None, reserved: ptr::null_mut(), flags: 0 }
        ];
        let switches = [
            SwiDesc { name: names[2].as_ptr(), act: (&raw mut BOOLEAN).cast(), kind: SWI_BOOLEAN },
            SwiDesc { name: names[3].as_ptr(), act: (&raw mut NUMBER).cast(), kind: SWI_NUMERIC },
            SwiDesc { name: names[4].as_ptr(), act: special as *mut c_void, kind: SWI_SPECIAL },
            SwiDesc { name: names[5].as_ptr(), act: (&raw mut NUMBER).cast(), kind: 9 },
            SwiDesc { name: names[6].as_ptr(), act: ptr::null_mut(), kind: SWI_NUMERIC },
            SwiDesc { name: ptr::null(), act: ptr::null_mut(), kind: 0 }
        ];
        let extensions = Extensions::default();
        let extension = unsafe { extensions.register(Tables { commands: commands.as_ptr(), switches: switches.as_ptr(), init: when_loaded }) };
        assert_eq!(extension.id, 0);
        assert_eq!(extension.functions, [ExtFunction { name: "probe".into(), flags: flags::NOARG, id: 0 }]);
        let kinds: Vec<(String, SwitchKind)> = extension.switches.iter().map(|switch| (switch.name.clone(), switch.kind)).collect();
        assert_eq!(kinds, [("flag".into(), SwitchKind::Boolean), ("number".into(), SwitchKind::Numeric), ("text".into(), SwitchKind::Special)]);
        let mut fake = Fake::default();
        extensions.init(0, &mut fake);
        for arg in [
            ExtArg::None { y: 1, x: 2 },
            ExtArg::Null { count: 1, y: 3, x: 4 },
            ExtArg::Text { count: 2, y: 5, x: 6, text: "so\0rt".into() },
            ExtArg::Line { count: 1, start: 7, end: 8 },
            ExtArg::Stream { count: 1, x_start: 1, y_start: 2, x_end: 3, y_end: 4 }
        ]
        {
            assert!(extensions.call(0, &arg, 97, true, &mut fake));
        }
        assert!(extensions.call(0, &ExtArg::Box { count: 3, left: 1, top: 2, right: 3, bottom: 4 }, 0, false, &mut fake) == false);
        assert!(extensions.set_switch(0, &SwitchValue::Boolean(true), &mut fake));
        assert!(extensions.set_switch(1, &SwitchValue::Numeric(42), &mut fake));
        assert!(extensions.set_switch(2, &SwitchValue::Special("SORT <".into()), &mut fake));
        assert!(extensions.set_switch(2, &SwitchValue::Special("bad".into()), &mut fake) == false);
        assert_eq!(unsafe { (BOOLEAN, NUMBER) }, (-1, 42));
        assert!(extensions.set_switch(0, &SwitchValue::Boolean(false), &mut fake));
        assert_eq!(unsafe { BOOLEAN }, 0);
        assert_eq!(fake.calls, [
            "setkey probe alt+p", "message noarg 1 2 key 97 meta -1", "message nullarg 1 3 4 key 97 meta -1", "message textarg 2 5 6 sort key 97 meta -1",
            "message linearg 1 7 8 key 97 meta -1", "message streamarg 1 1 2 3 4 key 97 meta -1", "message boxarg 3 1 2 3 4 key 0 meta 0", "message SORT <",
            "message bad"
        ]);
    }

    #[test]
    fn test_load_reports_missing_libraries()
    {
        let extensions = Extensions::default();
        let dir = std::env::temp_dir().join("mep-ext-missing");
        assert!(extensions.load(&dir.join("nothing.so")).is_err());
        assert!(extensions.load(&dir.join("nothing")).is_err());
    }
}
