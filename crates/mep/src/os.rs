//! Real file system for the editor (IMPLEMENTATION_PLAN.md §7)
//!
//! Thin wrappers around `std::fs`; all decisions live in `mep-core`.

use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH}
};

use mep_core::{
    env::{Env, LocalDateTime, Os},
    io::{FileMeta, Fs}
};

use crate::tz;

/// The operating system's file system
#[derive(Debug, Default, Clone, Copy)]
pub struct OsFs;

impl Fs for OsFs
{
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>
    {
        fs::read(path)
    }

    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()>
    {
        let temp = temp_path(path)?;
        // Keep the old file's permissions, e.g. the executable bit of a script.
        let permissions = fs::metadata(path).map(|meta| meta.permissions()).ok();
        fs::write(&temp, data)
            .and_then(|()| permissions.map_or(Ok(()), |permissions| fs::set_permissions(&temp, permissions)))
            .and_then(|()| fs::rename(&temp, path))
            .inspect_err(|_| {
                let _ = fs::remove_file(&temp);
            })
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>
    {
        fs::rename(from, to)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMeta>
    {
        match fs::metadata(path)
        {
            | Ok(meta) => Ok(FileMeta { exists: true, readonly: meta.permissions().readonly() }),
            | Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(FileMeta { exists: false, readonly: false }),
            | Err(error) => Err(error)
        }
    }

    fn read_dir_names(&self, dir: &Path) -> io::Result<Vec<OsString>>
    {
        fs::read_dir(dir)?.map(|entry| entry.map(|entry| entry.file_name())).collect()
    }

    fn create_dir_all(&self, dir: &Path) -> io::Result<()>
    {
        fs::create_dir_all(dir)
    }
}

/// The operating system this binary was built for
#[cfg(windows)]
const OS: Os = Os::Windows;
/// The operating system this binary was built for
#[cfg(target_os = "macos")]
const OS: Os = Os::MacOs;
/// The operating system this binary was built for
#[cfg(not(any(windows, target_os = "macos")))]
const OS: Os = Os::Linux;

/// The process environment and the system clock
#[derive(Debug, Default, Clone, Copy)]
pub struct OsEnv;

impl Env for OsEnv
{
    fn var(&self, name: &str) -> Option<OsString>
    {
        std::env::var_os(name)
    }

    fn now(&self) -> LocalDateTime
    {
        let utc = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
        let tz = std::env::var("TZ").ok();
        tz::civil(utc + local_offset(tz.as_deref(), &|path| fs::read(path).ok(), utc))
    }

    fn os(&self) -> Os
    {
        OS
    }

    fn cwd(&self) -> PathBuf
    {
        std::env::current_dir().unwrap_or_default()
    }
}

/// Returns the local UTC offset in seconds: from `TZ` (a POSIX rule or a zone file name) or `/etc/localtime`; 0 if unknown
pub fn local_offset(tz: Option<&str>, read: &dyn Fn(&Path) -> Option<Vec<u8>>, utc: i64) -> i64
{
    let zone_file = |name: &str| read(Path::new(&format!("/usr/share/zoneinfo/{name}"))).or_else(|| read(Path::new(name)));
    let offset = match tz.map(|tz| tz.strip_prefix(':').unwrap_or(tz))
    {
        | Some(tz) if tz.contains('/') => zone_file(tz).and_then(|bytes| tz::offset_at(&bytes, utc)),
        | Some(tz) if tz.is_empty() == false => tz::posix_offset(tz, utc).or_else(|| zone_file(tz).and_then(|bytes| tz::offset_at(&bytes, utc))),
        | _ => read(Path::new("/etc/localtime")).and_then(|bytes| tz::offset_at(&bytes, utc))
    };
    offset.unwrap_or(0)
}

/// Returns the temporary file next to `path` used for an atomic write
fn temp_path(path: &Path) -> io::Result<PathBuf>
{
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut temp = OsString::from(".");
    temp.push(name);
    temp.push(".mep-save");
    Ok(path.with_file_name(temp))
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use tempfile::TempDir;

    use super::*;

    fn set_readonly(path: &Path, readonly: bool)
    {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(readonly);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn test_read_and_write_atomic()
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a.txt");
        OsFs.write_atomic(&path, b"one\r\n").unwrap();
        assert_eq!(OsFs.read(&path).unwrap(), b"one\r\n");
        OsFs.write_atomic(&path, b"two\n").unwrap();
        assert_eq!(OsFs.read(&path).unwrap(), b"two\n");
        assert_eq!(OsFs.read_dir_names(dir.path()).unwrap(), vec![OsString::from("a.txt")]);
    }

    #[test]
    fn test_read_missing_file_is_not_found()
    {
        let dir = TempDir::new().unwrap();
        assert_eq!(OsFs.read(&dir.path().join("none")).unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn test_write_atomic_keeps_permissions()
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        OsFs.write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn test_write_atomic_failures_leave_no_temporary_file()
    {
        let dir = TempDir::new().unwrap();
        assert!(OsFs.write_atomic(&dir.path().join("missing").join("a"), b"x").is_err());
        let target = dir.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"").unwrap();
        assert!(OsFs.write_atomic(&target, b"x").is_err());
        assert_eq!(OsFs.read_dir_names(dir.path()).unwrap(), vec![OsString::from("target")]);
    }

    #[test]
    fn test_write_atomic_rejects_path_without_file_name()
    {
        assert_eq!(OsFs.write_atomic(Path::new("/"), b"x").unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_rename_replaces_target()
    {
        let dir = TempDir::new().unwrap();
        let (from, to) = (dir.path().join("a"), dir.path().join("b"));
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();
        OsFs.rename(&from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        assert!(OsFs.rename(&from, &to).is_err());
    }

    #[test]
    fn test_metadata_reports_existence_and_readonly()
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("r.txt");
        assert_eq!(OsFs.metadata(&path).unwrap(), FileMeta { exists: false, readonly: false });
        fs::write(&path, b"").unwrap();
        assert_eq!(OsFs.metadata(&path).unwrap(), FileMeta { exists: true, readonly: false });
        set_readonly(&path, true);
        assert!(OsFs.metadata(&path).unwrap().readonly);
        set_readonly(&path, false);
    }

    #[test]
    fn test_metadata_reports_other_errors()
    {
        assert_eq!(OsFs.metadata(Path::new("bad\0name")).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_os_env()
    {
        let env = OsEnv;
        assert!(env.now().year >= 2026);
        assert!(env.cwd().is_absolute());
        assert_eq!(env.os(), OS);
        assert!(env.var("PATH").is_some());
    }

    #[test]
    fn test_local_offset_sources()
    {
        // A version 1 zone file with no transitions and one zone type at UTC+1.
        let mut berlin = b"TZif".to_vec();
        berlin.extend([0; 16]);
        for count in [0u32, 0, 0, 0, 1, 0]
        {
            berlin.extend(count.to_be_bytes());
        }
        berlin.extend(3600i32.to_be_bytes());
        berlin.extend([0, 0]);
        let read = |path: &Path| match path.to_str()
        {
            | Some("/usr/share/zoneinfo/Europe/Berlin") | Some("/etc/localtime") | Some("Local") => Some(berlin.clone()),
            | _ => None
        };
        assert_eq!(local_offset(Some(":Europe/Berlin"), &read, 0), 3600);
        assert_eq!(local_offset(Some("EST5"), &read, 0), -5 * 3600);
        assert_eq!(local_offset(Some("Local"), &read, 0), 3600);
        assert_eq!(local_offset(Some("Mars/Base"), &read, 0), 0);
        assert_eq!(local_offset(Some(""), &read, 0), 3600);
        assert_eq!(local_offset(None, &read, 0), 3600);
        assert_eq!(local_offset(None, &|_| None, 0), 0);
    }

    #[test]
    fn test_read_dir_missing_directory_fails()
    {
        let dir = TempDir::new().unwrap();
        assert!(OsFs.read_dir_names(&dir.path().join("none")).is_err());
    }

    #[test]
    fn test_create_dir_all_creates_parents()
    {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b");
        OsFs.create_dir_all(&nested).unwrap();
        assert!(nested.is_dir());
    }
}
