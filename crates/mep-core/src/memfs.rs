//! In-memory file system for tests (IMPLEMENTATION_PLAN.md §4.2, §9.1)
//!
//! Every operation can be made to fail for a given path, so tests reach every
//! I/O error path of the editor.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError}
};

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

#[derive(Debug, Default)]
struct State
{
    files:    BTreeMap<PathBuf, Vec<u8>>,
    readonly: BTreeSet<PathBuf>,
    failures: BTreeSet<(Op, PathBuf)>
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
    pub fn with_file(self, path: impl Into<PathBuf>, bytes: &[u8]) -> Self
    {
        self.state().files.insert(path.into(), bytes.to_vec());
        self
    }

    /// Makes `op` fail with an I/O error for `path` from now on
    pub fn fail(&self, op: Op, path: impl Into<PathBuf>)
    {
        self.state().failures.insert((op, path.into()));
    }

    /// Returns the contents of a file
    pub fn file(&self, path: impl AsRef<Path>) -> Option<Vec<u8>>
    {
        self.state().files.get(path.as_ref()).cloned()
    }

    /// Marks a file read-only or writable
    pub fn set_readonly(&self, path: impl Into<PathBuf>, readonly: bool)
    {
        let path = path.into();
        let mut state = self.state();
        match readonly
        {
            | true => state.readonly.insert(path),
            | false => state.readonly.remove(&path)
        };
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State>
    {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Fails with an injected error if `op` was set to fail for `path`
    fn check(&self, op: Op, path: &Path) -> io::Result<()>
    {
        let failing = self.state().failures.contains(&(op, path.to_path_buf()));
        require!(failing == false, Err(io::Error::other(format!("injected {op:?} failure for {}", path.display()))));
        Ok(())
    }
}

impl Fs for MemFs
{
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>
    {
        self.check(Op::Read, path)?;
        self.file(path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()>
    {
        self.check(Op::Write, path)?;
        self.state().files.insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>
    {
        self.check(Op::Rename, from)?;
        self.check(Op::Rename, to)?;
        let mut state = self.state();
        let bytes = state.files.remove(from).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        state.files.insert(to.to_path_buf(), bytes);
        Ok(())
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMeta>
    {
        self.check(Op::Metadata, path)?;
        let state = self.state();
        let exists = state.files.contains_key(path);
        Ok(FileMeta { exists, readonly: exists && state.readonly.contains(path) })
    }

    fn read_dir_names(&self, dir: &Path) -> io::Result<Vec<OsString>>
    {
        self.check(Op::ReadDir, dir)?;
        let state = self.state();
        Ok(state.files.keys().filter(|path| path.parent() == Some(dir)).filter_map(|path| path.file_name()).map(OsString::from).collect())
    }

    fn create_dir_all(&self, dir: &Path) -> io::Result<()>
    {
        self.check(Op::CreateDir, dir)
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
        assert_eq!(fs.read(Path::new("/d/a")).unwrap(), b"1");
        fs.write_atomic(Path::new("/d/b"), b"2").unwrap();
        fs.rename(Path::new("/d/b"), Path::new("/d/a")).unwrap();
        assert_eq!(fs.file("/d/a").unwrap(), b"2");
        assert_eq!(fs.file("/d/b"), None);
    }

    #[test]
    fn test_missing_files_are_not_found()
    {
        let fs = MemFs::default();
        assert_eq!(fs.read(Path::new("/x")).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(fs.rename(Path::new("/x"), Path::new("/y")).unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_metadata_reports_existence_and_readonly()
    {
        let fs = MemFs::default().with_file("/a", b"");
        assert_eq!(fs.metadata(Path::new("/a")).unwrap(), FileMeta { exists: true, readonly: false });
        fs.set_readonly("/a", true);
        assert_eq!(fs.metadata(Path::new("/a")).unwrap(), FileMeta { exists: true, readonly: true });
        fs.set_readonly("/a", false);
        assert!(fs.metadata(Path::new("/a")).unwrap().readonly == false);
        assert_eq!(fs.metadata(Path::new("/b")).unwrap(), FileMeta { exists: false, readonly: false });
    }

    #[test]
    fn test_read_dir_lists_direct_children()
    {
        let fs = MemFs::default().with_file("/cfg/TOOLS.INI", b"").with_file("/cfg/sub/x", b"").with_file("/other", b"");
        assert_eq!(fs.read_dir_names(Path::new("/cfg")).unwrap(), vec![OsString::from("TOOLS.INI")]);
    }

    #[test]
    fn test_injected_failures()
    {
        let fs = MemFs::default().with_file("/a", b"x");
        for op in [Op::Read, Op::Write, Op::Rename, Op::Metadata, Op::ReadDir]
        {
            fs.fail(op, "/a");
        }
        assert!(fs.read(Path::new("/a")).is_err());
        assert!(fs.write_atomic(Path::new("/a"), b"y").is_err());
        assert!(fs.rename(Path::new("/a"), Path::new("/b")).is_err());
        assert!(fs.rename(Path::new("/c"), Path::new("/a")).is_err());
        assert!(fs.metadata(Path::new("/a")).is_err());
        assert_eq!(fs.read_dir_names(Path::new("/a")).unwrap_err().to_string(), "injected ReadDir failure for /a");
        assert_eq!(fs.file("/a").unwrap(), b"x");
    }
}
