//! File system boundary and worker requests (IMPLEMENTATION_PLAN.md §3.5, §4.2)
//!
//! The core never touches the file system. It emits [`Request`]s; a worker
//! runs them with [`serve`] against an [`Fs`] and sends back a [`Completion`].

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf}
};

use crate::{
    clip::{ClipboardPayload, SystemClipboard},
    ini::is_tools_ini
};

/// What [`Fs::metadata`] reports about a path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMeta
{
    /// The path exists
    pub exists:   bool,
    /// The file exists and cannot be written
    pub readonly: bool
}

/// All file system access of the editor
pub trait Fs
{
    /// Reads a whole file
    ///
    /// # Errors
    ///
    /// Returns the operating system error, `NotFound` for a missing file.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Replaces a file through a temporary file in the same directory and a rename
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()>;

    /// Renames a file, replacing an existing target
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Reports whether a path exists and is read-only
    ///
    /// # Errors
    ///
    /// Returns the operating system error; a missing path is not an error.
    fn metadata(&self, path: &Path) -> io::Result<FileMeta>;

    /// Lists the names in a directory
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn read_dir_names(&self, dir: &Path) -> io::Result<Vec<OsString>>;

    /// Creates a directory and its missing parents
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn create_dir_all(&self, dir: &Path) -> io::Result<()>;
}

/// Identifies a request so its completion can be matched
pub type RequestId = u64;

/// Work for a worker thread
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request
{
    /// Matches the completion to the request
    pub id:   RequestId,
    /// What to do
    pub kind: RequestKind
}

/// The operations a worker performs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind
{
    /// Read a file and whether it is read-only
    Load(PathBuf),
    /// Write a file; with `backup`, first rename the old version to `.bak`
    Save
    {
        /// Target file
        path:   PathBuf,
        /// Complete new contents
        data:   Vec<u8>,
        /// Keep the old version as `.bak` (`backup:bak`)
        backup: bool
    },
    /// Find and read TOOLS.INI in the first of these directories that has one
    FindIni(Vec<PathBuf>),
    /// Write the information file, creating its directory when needed
    WriteInfo
    {
        /// The information file
        path: PathBuf,
        /// Complete new contents
        data: Vec<u8>
    },
    /// Read the system clipboard
    ClipboardRead,
    /// Write the system clipboard
    ClipboardWrite(ClipboardPayload)
}

/// A loaded file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded
{
    /// The file's bytes, unchanged
    pub bytes:    Vec<u8>,
    /// The file cannot be written
    pub readonly: bool
}

/// Result of a request
#[derive(Debug)]
pub enum Reply
{
    /// Result of [`RequestKind::Load`]
    Loaded(io::Result<Loaded>),
    /// Result of [`RequestKind::Save`] and [`RequestKind::WriteInfo`]
    Saved(io::Result<()>),
    /// Result of [`RequestKind::FindIni`]: the file and its text, `None` if there is none
    Ini(io::Result<Option<(PathBuf, String)>>),
    /// Result of [`RequestKind::ClipboardRead`]; `None` if the clipboard is unavailable
    ClipboardRead(Option<ClipboardPayload>),
    /// Result of [`RequestKind::ClipboardWrite`]
    ClipboardWritten(bool)
}

/// A worker's answer to a request
#[derive(Debug)]
pub struct Completion
{
    /// The request's id
    pub id:    RequestId,
    /// The result
    pub reply: Reply
}

/// Runs `request` against the file system and the system clipboard
pub fn serve(request: Request, fs: &dyn Fs, clipboard: &mut dyn SystemClipboard) -> Completion
{
    let reply = match request.kind
    {
        | RequestKind::Load(path) => Reply::Loaded(load(fs, &path)),
        | RequestKind::Save { path, data, backup } => Reply::Saved(save(fs, &path, &data, backup)),
        | RequestKind::FindIni(dirs) => Reply::Ini(find_ini(fs, &dirs)),
        | RequestKind::WriteInfo { path, data } => Reply::Saved(write_info(fs, &path, &data)),
        | RequestKind::ClipboardRead => Reply::ClipboardRead(clipboard.read()),
        | RequestKind::ClipboardWrite(payload) => Reply::ClipboardWritten(clipboard.write(&payload))
    };
    Completion { id: request.id, reply }
}

/// Finds TOOLS.INI, matching the name without regard to case; unreadable directories are skipped
fn find_ini(fs: &dyn Fs, dirs: &[PathBuf]) -> io::Result<Option<(PathBuf, String)>>
{
    for dir in dirs
    {
        let names = fs.read_dir_names(dir).unwrap_or_default();
        if let Some(name) = names.iter().find(|name| is_tools_ini(&name.to_string_lossy()))
        {
            let path = dir.join(name);
            let text = String::from_utf8_lossy(&fs.read(&path)?).into_owned();
            return Ok(Some((path, text)));
        }
    }
    Ok(None)
}

fn write_info(fs: &dyn Fs, path: &Path, data: &[u8]) -> io::Result<()>
{
    fs.create_dir_all(path.parent().unwrap_or(Path::new("")))?;
    fs.write_atomic(path, data)
}

fn load(fs: &dyn Fs, path: &Path) -> io::Result<Loaded>
{
    let bytes = fs.read(path)?;
    let readonly = fs.metadata(path)?.readonly;
    Ok(Loaded { bytes, readonly })
}

fn save(fs: &dyn Fs, path: &Path, data: &[u8], backup: bool) -> io::Result<()>
{
    let meta = fs.metadata(path)?;
    // A rename would replace a read-only file on most systems, so refuse here.
    require!(meta.readonly == false, Err(io::Error::new(io::ErrorKind::PermissionDenied, "file is read-only")));
    if backup == true && meta.exists == true
    {
        fs.rename(path, &backup_path(path))?;
    }
    fs.write_atomic(path, data)
}

/// Returns the backup name for `path`: its extension replaced by `bak`
// ORACLE: FILE-BAK-1
pub fn backup_path(path: &Path) -> PathBuf
{
    path.with_extension("bak")
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;
    use crate::{
        clip::MemClipboard,
        memfs::{MemFs, Op}
    };

    fn run(kind: RequestKind, fs: &MemFs) -> Reply
    {
        serve(Request { id: 3, kind }, fs, &mut MemClipboard::default()).reply
    }

    #[test]
    fn test_find_ini_searches_directories_in_order()
    {
        let fs = MemFs::default().with_file("/b/Tools.Ini", b"[mep]\n").with_file("/c/tools.ini", b"x");
        let dirs = vec![PathBuf::from("/a"), PathBuf::from("/b"), PathBuf::from("/c")];
        match run(RequestKind::FindIni(dirs), &fs)
        {
            | Reply::Ini(Ok(Some((path, text)))) => assert_eq!((path, text.as_str()), (PathBuf::from("/b/Tools.Ini"), "[mep]\n")),
            | other => panic!("{other:?}")
        }
        assert!(matches!(run(RequestKind::FindIni(vec![PathBuf::from("/a")]), &fs), Reply::Ini(Ok(None))));
        fs.fail(Op::Read, "/b/Tools.Ini");
        assert!(matches!(run(RequestKind::FindIni(vec![PathBuf::from("/b")]), &fs), Reply::Ini(Err(_))));
    }

    #[test]
    fn test_clipboard_requests()
    {
        let mut clipboard = MemClipboard::default();
        let payload = ClipboardPayload { text: "t".into(), record: None };
        let fs = MemFs::default();
        let written = serve(Request { id: 1, kind: RequestKind::ClipboardWrite(payload.clone()) }, &fs, &mut clipboard);
        assert!(matches!(written.reply, Reply::ClipboardWritten(true)));
        let read = serve(Request { id: 2, kind: RequestKind::ClipboardRead }, &fs, &mut clipboard);
        assert!(matches!(read.reply, Reply::ClipboardRead(Some(found)) if found == payload));
    }

    fn load(fs: &MemFs, path: &str) -> io::Result<Loaded>
    {
        match serve(Request { id: 7, kind: RequestKind::Load(PathBuf::from(path)) }, fs, &mut MemClipboard::default())
        {
            | Completion { id: 7, reply: Reply::Loaded(result) } => result,
            | other => panic!("unexpected completion {other:?}")
        }
    }

    fn save(fs: &MemFs, path: &str, data: &[u8], backup: bool) -> io::Result<()>
    {
        let kind = RequestKind::Save { path: PathBuf::from(path), data: data.to_vec(), backup };
        match serve(Request { id: 8, kind }, fs, &mut MemClipboard::default())
        {
            | Completion { id: 8, reply: Reply::Saved(result) } => result,
            | other => panic!("unexpected completion {other:?}")
        }
    }

    #[test]
    fn test_load_returns_bytes_and_readonly_state()
    {
        let fs = MemFs::default().with_file("/a.txt", b"a\r\n");
        assert_eq!(load(&fs, "/a.txt").unwrap(), Loaded { bytes: b"a\r\n".to_vec(), readonly: false });
        fs.set_readonly("/a.txt", true);
        assert!(load(&fs, "/a.txt").unwrap().readonly);
    }

    #[test]
    fn test_load_missing_file_is_not_found()
    {
        assert_eq!(load(&MemFs::default(), "/none").unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_load_read_and_metadata_failures()
    {
        let fs = MemFs::default().with_file("/a", b"x");
        fs.fail(Op::Read, "/a");
        assert!(load(&fs, "/a").is_err());
        let fs = MemFs::default().with_file("/a", b"x");
        fs.fail(Op::Metadata, "/a");
        assert!(load(&fs, "/a").is_err());
    }

    #[test]
    fn test_save_writes_file()
    {
        let fs = MemFs::default();
        save(&fs, "/new.c", b"int x;\n", false).unwrap();
        assert_eq!(fs.file("/new.c").unwrap(), b"int x;\n");
    }

    #[test]
    fn test_save_with_backup_renames_old_version()
    {
        let fs = MemFs::default().with_file("/src/main.c", b"old");
        save(&fs, "/src/main.c", b"new", true).unwrap();
        assert_eq!(fs.file("/src/main.c").unwrap(), b"new");
        assert_eq!(fs.file("/src/main.bak").unwrap(), b"old");
    }

    #[test]
    fn test_save_with_backup_of_new_file_skips_backup()
    {
        let fs = MemFs::default();
        save(&fs, "/n.c", b"x", true).unwrap();
        assert_eq!(fs.file("/n.bak"), None);
    }

    #[test]
    fn test_save_without_backup_keeps_no_old_version()
    {
        let fs = MemFs::default().with_file("/a.c", b"old");
        save(&fs, "/a.c", b"new", false).unwrap();
        assert_eq!(fs.file("/a.bak"), None);
    }

    #[test]
    fn test_save_refuses_readonly_file()
    {
        let fs = MemFs::default().with_file("/r.c", b"old");
        fs.set_readonly("/r.c", true);
        let error = save(&fs, "/r.c", b"new", false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "file is read-only");
        assert_eq!(fs.file("/r.c").unwrap(), b"old");
    }

    #[test]
    fn test_save_failures_are_reported()
    {
        for op in [Op::Metadata, Op::Rename, Op::Write]
        {
            let fs = MemFs::default().with_file("/a.c", b"old");
            fs.fail(op, "/a.c");
            assert!(save(&fs, "/a.c", b"new", true).is_err(), "{op:?}");
        }
    }

    #[test]
    fn test_backup_path_replaces_extension()
    {
        assert_eq!(backup_path(Path::new("/x/main.c")), PathBuf::from("/x/main.bak"));
        assert_eq!(backup_path(Path::new("/x/Makefile")), PathBuf::from("/x/Makefile.bak"));
        assert_eq!(backup_path(Path::new("/x/a.tar.gz")), PathBuf::from("/x/a.tar.bak"));
    }
}
