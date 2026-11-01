//! Search (docs/spec/commands: psearch, msearch; IMPLEMENTATION_PLAN.md §3.5, §6.11, §10)
//!
//! `arg` searches for literal text, `arg arg` for a regular expression in the
//! syntax of the `regex` crate. A search scans a bounded number of lines per
//! step, so input stays responsive and Esc cancels a long search.

use regex::bytes::{Regex, RegexBuilder};

use crate::{
    cells,
    editor::{Arg, Editor, Pending, Step},
    view::Pos
};

/// A search in progress
#[derive(Debug, Clone)]
pub(crate) struct Search
{
    pub(crate) forward: bool,
    pub(crate) regex:   Regex,
    pub(crate) text:    String,
    pub(crate) origin:  Pos,
    pub(crate) next:    usize,
    pub(crate) done:    bool
}

/// Compiles a search string: literal text unless `regex`; case-insensitive unless `case` (plan §10)
pub(crate) fn compile(text: &str, regex: bool, case: bool) -> Result<Regex, String>
{
    let pattern = match regex
    {
        | true => text.to_string(),
        | false => regex::escape(text)
    };
    // The crate's message spans several lines with a caret display; the last line says what is wrong.
    RegexBuilder::new(&pattern).case_insensitive(case == false).build().map_err(|error| error.to_string().lines().last().unwrap_or_default().trim().to_string())
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

/// Takes the search string from the argument or the previous search and runs the first slice
fn start(ed: &mut Editor, arg: Arg, forward: bool) -> Step
{
    let regex = arg.count() >= 2;
    let wanted = match arg
    {
        | Arg::Text { text, .. } if text.is_empty() => return ed.fail("Invalid argument"),
        | Arg::Text { text, .. } => Some((text, regex)),
        | _ => ed.search.clone()
    };
    let Some((text, regex)) = wanted
    else
    {
        return ed.fail("No search string");
    };
    match compile(&text, regex, ed.switches.case)
    {
        | Ok(compiled) =>
        {
            ed.search = Some((text.clone(), regex));
            let origin = ed.cursor();
            ed.search_step(Search { forward, regex: compiled, text, origin, next: origin.line, done: false })
        }
        | Err(error) => ed.fail(&error)
    }
}

impl Editor
{
    /// Scans one slice of lines; finds, fails or suspends for the next slice
    pub(crate) fn search_step(&mut self, mut search: Search) -> Step
    {
        let count = self.line_count();
        for _ in 0..self.slice_lines
        {
            let line = search.next;
            let shown = self.shown_line(line);
            let on_origin = line == search.origin.line;
            let hit = match search.forward
            {
                | true =>
                {
                    // Start one character after the cursor so a repeated search finds the next match.
                    let after = cells::glyphs(&shown).into_iter().find(|glyph| glyph.col == search.origin.col).map_or(shown.len() + 1, |glyph| glyph.bytes.end);
                    let from = if on_origin
                    {
                        after
                    }
                    else
                    {
                        0
                    };
                    (from <= shown.len()).then(|| search.regex.find_at(&shown, from)).flatten()
                }
                | false =>
                {
                    let limit = if on_origin
                    {
                        cells::byte_at(&shown, search.origin.col)
                    }
                    else
                    {
                        usize::MAX
                    };
                    search.regex.find_iter(&shown).take_while(|found| found.start() < limit).last()
                }
            };
            if let Some(found) = hit
            {
                let col = cells::col_at(&shown, found.start());
                self.set_cursor(Pos::new(line, col));
                self.highlight = Some((Pos::new(line, col), cells::width(found.as_bytes())));
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
            | true => self.fail(&format!("{} not found", search.text)),
            | false => Step::Wait(Pending::Search(Box::new(search)))
        }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_compile_literal_regex_and_case()
    {
        assert!(compile("a.c", false, true).unwrap().is_match(b"xa.cx"));
        assert!(compile("a.c", false, true).unwrap().is_match(b"abc") == false);
        assert!(compile("a.c", true, true).unwrap().is_match(b"abc"));
        assert!(compile("ABC", false, false).unwrap().is_match(b"abc"));
        assert!(compile("ABC", false, true).unwrap().is_match(b"abc") == false);
        assert_eq!(compile("(a", true, true).unwrap_err(), "error: unclosed group");
    }
}
