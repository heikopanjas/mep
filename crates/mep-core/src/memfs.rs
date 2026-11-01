//! In-memory file system for tests (IMPLEMENTATION_PLAN.md §4.2, §9.1)
//!
//! Every operation can be made to fail for a given path, so tests reach every
//! I/O error path of the editor.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io,
    sync::{Mutex, PoisonError}
};

use typed_path::{Utf8TypedPathBuf, Utf8UnixPath};

use crate::io::{FileMeta, Fs};

/// A file system operation that can be made to fail
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op
{
    /// [`Fs::read`]
    Read,
    /// [`Fs::write_atomic`]
    Write,
    /// [`Fs::rename`] (fails for either path)
    Rename,
    /// [`Fs::metadata`]
    Metadata,
    /// [`Fs::read_dir_names`]
    ReadDir,
    /// [`Fs::create_dir_all`]
    CreateDir
}

/// Files are keyed by their path with `/` separators, so Unix and Windows paths of the core reach the same file
#[derive(Debug, Default)]
struct State
{
    files:    BTreeMap<String, Vec<u8>>,
    readonly: BTreeSet<String>,
    failures: BTreeSet<(Op, String)>
}

/// Returns the key of a path the core passes in
fn key(path: &Utf8TypedPathBuf) -> String
{
    path.with_unix_encoding().into_string()
}

/// File system held in memory; directories exist implicitly
#[derive(Debug, Default)]
pub struct MemFs
{
    state: Mutex<State>
}

impl MemFs
{
    /// Adds a file and returns the file system, for building test fixtures
    pub fn with_file(self, path: &str, bytes: &[u8]) -> Self
    {
        self.state().files.insert(path.to_string(), bytes.to_vec());
        self
    }

    /// Makes `op` fail with an I/O error for `path` from now on
    pub fn fail(&self, op: Op, path: &str)
    {
        self.state().failures.insert((op, path.to_string()));
    }

    /// Returns the contents of a file
    pub fn file(&self, path: &str) -> Option<Vec<u8>>
    {
        self.state().files.get(path).cloned()
    }

    /// Marks a file read-only or writable
    pub fn set_readonly(&self, path: &str, readonly: bool)
    {
        let mut state = self.state();
        match readonly
        {
            | true => state.readonly.insert(path.to_string()),
            | false => state.readonly.remove(path)
        };
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State>
    {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Fails with an injected error if `op` was set to fail for `path`
    fn check(&self, op: Op, path: &str) -> io::Result<()>
    {
        let failing = self.state().failures.contains(&(op, path.to_string()));
        require!(failing == false, Err(io::Error::other(format!("injected {op:?} failure for {path}"))));
        Ok(())
    }
}

impl Fs for MemFs
{
    fn read(&self, path: &Utf8TypedPathBuf) -> io::Result<Vec<u8>>
    {
        let path = key(path);
        self.check(Op::Read, &path)?;
        self.file(&path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn write_atomic(&self, path: &Utf8TypedPathBuf, data: &[u8]) -> io::Result<()>
    {
        let path = key(path);
        self.check(Op::Write, &path)?;
        self.state().files.insert(path, data.to_vec());
        Ok(())
    }

    fn rename(&self, from: &Utf8TypedPathBuf, to: &Utf8TypedPathBuf) -> io::Result<()>
    {
        let (from, to) = (key(from), key(to));
        self.check(Op::Rename, &from)?;
        self.check(Op::Rename, &to)?;
        let mut state = self.state();
        let bytes = state.files.remove(&from).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        state.files.insert(to, bytes);
        Ok(())
    }

    fn metadata(&self, path: &Utf8TypedPathBuf) -> io::Result<FileMeta>
    {
        let path = key(path);
        self.check(Op::Metadata, &path)?;
        let state = self.state();
        let exists = state.files.contains_key(&path);
        Ok(FileMeta { exists, readonly: exists && state.readonly.contains(&path) })
    }

    fn read_dir_names(&self, dir: &Utf8TypedPathBuf) -> io::Result<Vec<OsString>>
    {
        let dir = key(dir);
        self.check(Op::ReadDir, &dir)?;
        let state = self.state();
        let dir = Utf8UnixPath::new(&dir);
        Ok(state.files.keys().map(Utf8UnixPath::new).filter(|path| path.parent() == Some(dir)).filter_map(|path| path.file_name()).map(OsString::from).collect())
    }

    fn create_dir_all(&self, dir: &Utf8TypedPathBuf) -> io::Result<()>
    {
        self.check(Op::CreateDir, &key(dir))
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_files_can_be_read_written_and_renamed()
    {
        let fs = MemFs::default().with_file("/d/a", b"1");
        assert_eq!(fs.read(&Utf8TypedPathBuf::from("/d/a")).unwrap(), b"1");
        fs.write_atomic(&Utf8TypedPathBuf::from("/d/b"), b"2").unwrap();
        fs.rename(&Utf8TypedPathBuf::from("/d/b"), &Utf8TypedPathBuf::from("/d/a")).unwrap();
        assert_eq!(fs.file("/d/a").unwrap(), b"2");
        assert_eq!(fs.file("/d/b"), None);
    }

    #[test]
    fn test_missing_files_are_not_found()
    {
        let fs = MemFs::default();
        assert_eq!(fs.read(&Utf8TypedPathBuf::from("/x")).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(fs.rename(&Utf8TypedPathBuf::from("/x"), &Utf8TypedPathBuf::from("/y")).unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_metadata_reports_existence_and_readonly()
    {
        let fs = MemFs::default().with_file("/a", b"");
        assert_eq!(fs.metadata(&Utf8TypedPathBuf::from("/a")).unwrap(), FileMeta { exists: true, readonly: false });
        fs.set_readonly("/a", true);
        assert_eq!(fs.metadata(&Utf8TypedPathBuf::from("/a")).unwrap(), FileMeta { exists: true, readonly: true });
        fs.set_readonly("/a", false);
        assert!(fs.metadata(&Utf8TypedPathBuf::from("/a")).unwrap().readonly == false);
        assert_eq!(fs.metadata(&Utf8TypedPathBuf::from("/b")).unwrap(), FileMeta { exists: false, readonly: false });
    }

    #[test]
    fn test_read_dir_lists_direct_children()
    {
        let fs = MemFs::default().with_file("/cfg/TOOLS.INI", b"").with_file("/cfg/sub/x", b"").with_file("/other", b"");
        assert_eq!(fs.read_dir_names(&Utf8TypedPathBuf::from("/cfg")).unwrap(), vec![OsString::from("TOOLS.INI")]);
    }

    #[test]
    fn test_injected_failures()
    {
        let fs = MemFs::default().with_file("/a", b"x");
        for op in [Op::Read, Op::Write, Op::Rename, Op::Metadata, Op::ReadDir]
        {
            fs.fail(op, "/a");
        }
        assert!(fs.read(&Utf8TypedPathBuf::from("/a")).is_err());
        assert!(fs.write_atomic(&Utf8TypedPathBuf::from("/a"), b"y").is_err());
        assert!(fs.rename(&Utf8TypedPathBuf::from("/a"), &Utf8TypedPathBuf::from("/b")).is_err());
        assert!(fs.rename(&Utf8TypedPathBuf::from("/c"), &Utf8TypedPathBuf::from("/a")).is_err());
        assert!(fs.metadata(&Utf8TypedPathBuf::from("/a")).is_err());
        assert_eq!(fs.read_dir_names(&Utf8TypedPathBuf::from("/a")).unwrap_err().to_string(), "injected ReadDir failure for /a");
        assert_eq!(fs.file("/a").unwrap(), b"x");
    }
}
