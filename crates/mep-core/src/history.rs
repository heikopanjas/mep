//! The information file: files remembered between sessions (docs/spec/commands/information.md; M guide p. 38, p. 60)
//!
//! One file per line as `path<TAB>line<TAB>column<TAB>top<TAB>left`, most
//! recent first, positions 0-based (ORACLE INFO-1).

use std::path::PathBuf;

use crate::view::{Pos, View};

/// A remembered file with its cursor and window position
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry
{
    /// The file
    pub path: PathBuf,
    /// Its cursor and window position
    pub view: View
}

/// Reads the entries of an information file; malformed lines are skipped
// ORACLE: INFO-1
pub fn parse(text: &str) -> Vec<Entry>
{
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let path = fields.next().filter(|path| path.is_empty() == false)?;
            let numbers: Vec<usize> = fields.map(|field| field.parse().ok()).collect::<Option<_>>()?;
            match numbers.as_slice()
            {
                | [line, col, top, left] => Some(Entry { path: PathBuf::from(path), view: View { top: *top, left: *left, cursor: Pos::new(*line, *col) } }),
                | _ => None
            }
        })
        .collect()
}

/// Writes entries in the information file format
pub fn format(entries: &[Entry]) -> String
{
    entries
        .iter()
        .map(|entry| {
            let view = entry.view;
            format!("{}\t{}\t{}\t{}\t{}\n", entry.path.display(), view.cursor.line, view.cursor.col, view.top, view.left)
        })
        .collect()
}

/// Puts `recent` first, then the `earlier` entries for other files, cut to `limit`
pub fn merge(recent: Vec<Entry>, earlier: &[Entry], limit: usize) -> Vec<Entry>
{
    let mut entries = recent;
    for entry in earlier
    {
        if entries.iter().any(|known| known.path == entry.path) == false
        {
            entries.push(entry.clone());
        }
    }
    entries.truncate(limit);
    entries
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    fn entry(path: &str, line: usize) -> Entry
    {
        Entry { path: PathBuf::from(path), view: View { top: line, left: 0, cursor: Pos::new(line, 1) } }
    }

    #[test]
    fn test_format_and_parse_round_trip()
    {
        let entries = vec![entry("/a b.txt", 3), entry("/c", 0)];
        let text = format(&entries);
        assert_eq!(text, "/a b.txt\t3\t1\t3\t0\n/c\t0\t1\t0\t0\n");
        assert_eq!(parse(&text), entries);
    }

    #[test]
    fn test_malformed_lines_are_skipped()
    {
        assert!(parse("\n/a\t1\t2\t3\n/a\t1\t2\t3\t4\t5\n/a\tx\t2\t3\t4\n\t1\t2\t3\t4\n").is_empty());
    }

    #[test]
    fn test_merge_keeps_each_file_once_and_cuts()
    {
        let merged = merge(vec![entry("/b", 5)], &[entry("/a", 1), entry("/b", 2), entry("/c", 3)], 2);
        assert_eq!(merged, vec![entry("/b", 5), entry("/a", 1)]);
    }
}
