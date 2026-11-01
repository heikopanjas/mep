//! Open files and pseudo-files (IMPLEMENTATION_PLAN.md §5.1, §5.6)

use typed_path::Utf8TypedPathBuf;

use crate::{Text, io::Loaded};

/// Name of an open file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileName
{
    /// A file on disk, as an absolute path
    Disk(Utf8TypedPathBuf),
    /// A pseudo-file such as `<clipboard>`, stored without the angle brackets
    Pseudo(String)
}

impl FileName
{
    /// Parses a name typed by the user: `<name>` is a pseudo-file, anything else a path relative to `cwd`
    pub fn parse(name: &str, cwd: &Utf8TypedPathBuf) -> Self
    {
        match name.strip_prefix('<').and_then(|rest| rest.strip_suffix('>'))
        {
            | Some(pseudo) => Self::Pseudo(pseudo.to_string()),
            | None => Self::Disk(resolve(cwd, name))
        }
    }

    /// Returns the name as shown on the status line
    pub fn display(&self) -> String
    {
        match self
        {
            | Self::Disk(path) => path.to_string(),
            | Self::Pseudo(name) => format!("<{name}>")
        }
    }

    /// Returns the path of a disk file
    pub fn path(&self) -> Option<&Utf8TypedPathBuf>
    {
        match self
        {
            | Self::Disk(path) => Some(path),
            | Self::Pseudo(_) => None
        }
    }
}

/// Makes `name` absolute against `cwd` and removes `.` and `..` components without touching the disk
pub fn resolve(cwd: &Utf8TypedPathBuf, name: &str) -> Utf8TypedPathBuf
{
    cwd.join(name).normalize()
}

/// One open file or pseudo-file
#[derive(Debug)]
pub struct File
{
    /// Where the file lives
    pub name:     FileName,
    /// The file's bytes and lines
    pub text:     Text,
    /// Changed since loading or saving
    pub modified: bool,
    /// The file on disk cannot be written
    pub readonly: bool,
    /// New lines end with CRLF; otherwise LF and the status line shows `NL`
    pub crlf:     bool
}

impl File
{
    /// Creates an empty file whose new lines end with CRLF if `crlf` is set
    pub fn new(name: FileName, crlf: bool) -> Self
    {
        Self { name, text: Text::from(Vec::new()), modified: false, readonly: false, crlf }
    }

    /// Creates a file from loaded bytes; CRLF is the default terminator if any line ends in CRLF
    pub fn loaded(name: FileName, loaded: Loaded) -> Self
    {
        // Every line feed ends a line, so any CR LF pair is a CRLF terminator.
        let crlf = loaded.bytes.windows(2).any(|pair| pair == b"\r\n");
        Self { name, text: Text::from(loaded.bytes), modified: false, readonly: loaded.readonly, crlf }
    }
}

/// Handle of an open file, stable for the file's lifetime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(usize);

impl FileId
{
    /// Returns the extension handle (`PFILE`) of the file: never 0
    pub fn handle(self) -> usize
    {
        self.0 + 1
    }

    /// Returns the file of an extension handle; `None` for 0
    pub fn from_handle(handle: usize) -> Option<Self>
    {
        handle.checked_sub(1).map(Self)
    }
}

/// All open files
#[derive(Debug, Default)]
pub struct FileTable
{
    files: Vec<Option<File>>
}

impl FileTable
{
    /// Adds a file and returns its handle
    pub fn add(&mut self, file: File) -> FileId
    {
        self.files.push(Some(file));
        FileId(self.files.len() - 1)
    }

    /// Returns the handle of the open file named `name`
    pub fn find(&self, name: &FileName) -> Option<FileId>
    {
        self.files.iter().position(|file| file.as_ref().is_some_and(|file| file.name == *name)).map(FileId)
    }

    /// Returns an open file
    pub fn get(&self, id: FileId) -> Option<&File>
    {
        self.files.get(id.0).and_then(Option::as_ref)
    }

    /// Returns an open file for changing
    pub fn get_mut(&mut self, id: FileId) -> Option<&mut File>
    {
        self.files.get_mut(id.0).and_then(Option::as_mut)
    }

    /// Returns all open files with their handles
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &File)>
    {
        self.files.iter().enumerate().filter_map(|(index, file)| file.as_ref().map(|file| (FileId(index), file)))
    }

    /// Closes a file and returns it
    pub fn remove(&mut self, id: FileId) -> Option<File>
    {
        self.files.get_mut(id.0).and_then(Option::take)
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    fn loaded(bytes: &[u8]) -> Loaded
    {
        Loaded { bytes: bytes.to_vec(), readonly: false }
    }

    #[test]
    fn test_resolve_relative_and_absolute_names()
    {
        let cwd = &Utf8TypedPathBuf::from("/home/u/src");
        assert_eq!(resolve(cwd, "a.c"), Utf8TypedPathBuf::from("/home/u/src/a.c"));
        assert_eq!(resolve(cwd, "./x/../b.c"), Utf8TypedPathBuf::from("/home/u/src/b.c"));
        assert_eq!(resolve(cwd, "../../../../etc/p"), Utf8TypedPathBuf::from("/etc/p"));
        assert_eq!(resolve(cwd, "/tmp/c.c"), Utf8TypedPathBuf::from("/tmp/c.c"));
        let cwd = &Utf8TypedPathBuf::from_windows(r"C:\src\mep");
        assert_eq!(resolve(cwd, r"..\a.c").as_str(), r"C:\src\a.c");
        assert_eq!(resolve(cwd, "x/b.c").as_str(), r"C:\src\mep\x\b.c");
        assert_eq!(resolve(cwd, r"D:\c.c").as_str(), r"D:\c.c");
    }

    #[test]
    fn test_parse_pseudo_and_disk_names()
    {
        let cwd = &Utf8TypedPathBuf::from("/w");
        assert_eq!(FileName::parse("<clipboard>", cwd), FileName::Pseudo("clipboard".into()));
        assert_eq!(FileName::parse("a.txt", cwd), FileName::Disk(Utf8TypedPathBuf::from("/w/a.txt")));
        assert_eq!(FileName::parse("<odd", cwd), FileName::Disk(Utf8TypedPathBuf::from("/w/<odd")));
    }

    #[test]
    fn test_display_and_path()
    {
        let disk = FileName::Disk(Utf8TypedPathBuf::from("/w/a.txt"));
        assert_eq!(disk.display(), "/w/a.txt");
        assert_eq!(disk.path(), Some(&Utf8TypedPathBuf::from("/w/a.txt")));
        let pseudo = FileName::Pseudo("clipboard".into());
        assert_eq!(pseudo.display(), "<clipboard>");
        assert_eq!(pseudo.path(), None);
    }

    #[test]
    fn test_loaded_file_detects_default_terminator()
    {
        let name = FileName::Pseudo("t".into());
        assert!(File::loaded(name.clone(), loaded(b"a\nb\r\nc")).crlf);
        assert!(File::loaded(name.clone(), loaded(b"a\nb\n")).crlf == false);
        assert!(File::loaded(name.clone(), loaded(b"a\rb")).crlf == false);
        let file = File::loaded(name, Loaded { bytes: b"x".to_vec(), readonly: true });
        assert!(file.readonly);
        assert!(file.modified == false);
        assert_eq!(file.text.to_bytes(), b"x");
    }

    #[test]
    fn test_new_file_is_empty()
    {
        let file = File::new(FileName::Pseudo("untitled".into()), true);
        assert!(file.text.is_empty());
        assert!(file.crlf);
        assert!(file.modified == false);
        assert!(file.readonly == false);
    }

    #[test]
    fn test_file_table_handles()
    {
        let mut table = FileTable::default();
        let a = table.add(File::new(FileName::Pseudo("a".into()), false));
        let b = table.add(File::new(FileName::Pseudo("b".into()), false));
        assert_ne!(a, b);
        assert_eq!(table.find(&FileName::Pseudo("b".into())), Some(b));
        table.get_mut(a).unwrap().modified = true;
        assert!(table.get(a).unwrap().modified);
        assert!(table.remove(a).is_some());
        assert!(table.get(a).is_none());
        assert!(table.get_mut(a).is_none());
        assert!(table.remove(a).is_none());
        assert_eq!(table.find(&FileName::Pseudo("a".into())), None);
        assert_eq!(table.get(b).unwrap().name, FileName::Pseudo("b".into()));
        assert_eq!(table.iter().map(|(id, _)| id).collect::<Vec<_>>(), vec![b]);
    }
}
