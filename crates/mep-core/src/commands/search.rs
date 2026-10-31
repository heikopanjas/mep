//! Plain-string search (docs/spec/commands: psearch, msearch; IMPLEMENTATION_PLAN.md §3.5, §6.11)
//!
//! A search scans a bounded number of lines per step, so input stays responsive
//! and Esc cancels a long search.

use crate::{
    cells,
    editor::{Arg, Editor, Pending, Step},
    view::Pos
};

/// A search in progress
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Search
{
    pub(crate) forward: bool,
    pub(crate) pattern: Vec<char>,
    pub(crate) origin:  Pos,
    pub(crate) next:    usize,
    pub(crate) done:    bool
}

/// `psearch` (psearch.md)
pub(crate) fn psearch(ed: &mut Editor, arg: Arg) -> Step
{
    start(ed, arg, true)
}

/// `msearch` (msearch.md)
pub(crate) fn msearch(ed: &mut Editor, arg: Arg) -> Step
{
    start(ed, arg, false)
}

/// Sets the search string from the argument and runs the first slice
fn start(ed: &mut Editor, arg: Arg, forward: bool) -> Step
{
    if arg.count() >= 2
    {
        return ed.fail("Regular expressions are not yet supported");
    }
    if let Arg::Text { text, .. } = arg
    {
        if text.is_empty() == true
        {
            return ed.fail("Invalid argument");
        }
        ed.search = Some(text);
    }
    let Some(pattern) = ed.search.clone()
    else
    {
        return ed.fail("No search string");
    };
    let origin = ed.cursor();
    ed.search_step(Search { forward, pattern: pattern.chars().collect(), origin, next: origin.line, done: false })
}

/// Returns the characters of a line with their byte offsets; invalid bytes become U+FFFD
fn chars_of(line: &[u8]) -> Vec<(usize, char)>
{
    let mut chars = Vec::new();
    let mut offset = 0;
    for chunk in line.utf8_chunks()
    {
        for (at, ch) in chunk.valid().char_indices()
        {
            chars.push((offset + at, ch));
        }
        offset += chunk.valid().len();
        for _ in chunk.invalid()
        {
            chars.push((offset, char::REPLACEMENT_CHARACTER));
            offset += 1;
        }
    }
    chars
}

/// Folds a character for comparison when `case` is off
fn fold(ch: char, case: bool) -> char
{
    match case
    {
        | true => ch,
        | false => ch.to_lowercase().next().unwrap_or(ch)
    }
}

/// Returns the byte ranges of all matches of `pattern` in `line`
fn matches(line: &[u8], pattern: &[char], case: bool) -> Vec<std::ops::Range<usize>>
{
    let chars = chars_of(line);
    (0..chars.len())
        .filter(|start| pattern.len() <= chars.len() - start && pattern.iter().zip(&chars[*start..]).all(|(want, (_, have))| fold(*want, case) == fold(*have, case)))
        .map(|start| {
            let end = chars.get(start + pattern.len()).map_or(line.len(), |(at, _)| *at);
            chars[start].0..end
        })
        .collect()
}

impl Editor
{
    /// Scans one slice of lines; finds, fails or suspends for the next slice
    pub(crate) fn search_step(&mut self, mut search: Search) -> Step
    {
        let case = self.switches.case;
        let count = self.line_count();
        for _ in 0..self.slice_lines
        {
            let line = search.next;
            let shown = self.shown_line(line);
            let found = matches(&shown, &search.pattern, case).into_iter().map(|range| (cells::col_at(&shown, range.start), range)).filter(|(col, _)| {
                (line != search.origin.line) || (search.forward && *col > search.origin.col) || (search.forward == false && *col < search.origin.col)
            });
            let hit = match search.forward
            {
                | true => found.into_iter().next(),
                | false => found.into_iter().next_back()
            };
            if let Some((col, range)) = hit
            {
                let width = cells::width(&shown[range]);
                self.set_cursor(Pos::new(line, col));
                self.highlight = Some((Pos::new(line, col), width));
                return Step::Done(true);
            }
            match (search.forward, line)
            {
                | (true, line) if line + 1 < count => search.next = line + 1,
                | (false, line) if line > 0 => search.next = line - 1,
                | _ =>
                {
                    search.done = true;
                    break;
                }
            }
        }
        match search.done
        {
            | true =>
            {
                let text: String = search.pattern.iter().collect();
                self.fail(&format!("{text} not found"))
            }
            | false => Step::Wait(Pending::Search(search))
        }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_matches_fold_case_and_handle_invalid_bytes()
    {
        let pattern: Vec<char> = "ab".chars().collect();
        assert_eq!(matches(b"xAbab", &pattern, false), vec![1..3, 3..5]);
        assert_eq!(matches(b"xAbab", &pattern, true), vec![3..5]);
        assert_eq!(matches(b"\xffab", &pattern, false), vec![1..3]);
        assert_eq!(matches(b"a", &pattern, false), Vec::<std::ops::Range<usize>>::new());
        assert_eq!(chars_of("ä\u{fffd}".as_bytes()).len(), 2);
    }
}
