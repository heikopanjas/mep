//! Search and replace (docs/spec/commands: replace, qreplace; IMPLEMENTATION_PLAN.md §3.5, §10)
//!
//! Both prompt for the search and replacement strings, then work line by line
//! over the range in bounded slices. `qreplace` asks at every match.

use std::ops::Range;

use regex::bytes::Regex;

use super::search::compile;
use crate::{
    cells,
    editor::{Answer, Arg, Editor, Pending, Step, TextArg, Then},
    keys::{Key, KeyCode},
    view::Pos
};

/// Where replacements happen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Span
{
    /// From a position to the end of the file
    From(Pos),
    /// Whole lines `first..=last`
    Lines(usize, usize),
    /// From the first position up to the second
    Stream(Pos, Pos),
    /// Lines `top..=bottom`, cells `left..right`
    Box
    {
        top: usize, bottom: usize, left: usize, right: usize
    }
}

impl Span
{
    /// Returns the lines the span touches, limited to `count` lines
    fn lines(self, count: usize) -> Range<usize>
    {
        let (first, last) = match self
        {
            | Self::From(pos) => (pos.line, usize::MAX - 1),
            | Self::Lines(first, last) => (first, last),
            | Self::Stream(start, end) => (start.line, end.line),
            | Self::Box { top, bottom, .. } => (top, bottom)
        };
        first..(last + 1).min(count)
    }

    /// Returns the bytes of a displayed line a match may cover
    fn window(self, line: usize, shown: &[u8]) -> Range<usize>
    {
        let at = |col: usize| cells::byte_at(shown, col);
        match self
        {
            | Self::From(pos) if line == pos.line => at(pos.col)..shown.len(),
            | Self::Stream(start, end) =>
            {
                let from = if line == start.line
                {
                    at(start.col)
                }
                else
                {
                    0
                };
                let to = if line == end.line
                {
                    at(end.col)
                }
                else
                {
                    shown.len()
                };
                from..to.max(from)
            }
            | Self::Box { left, right, .. } => cells::box_range(shown, left, right),
            | _ => 0..shown.len()
        }
    }
}

/// A replace in progress
#[derive(Debug, Clone)]
pub(crate) struct Replace
{
    regex:       Regex,
    replacement: Vec<u8>,
    expand:      bool,
    span:        Span,
    query:       bool,
    all:         bool,
    line:        usize,
    at:          usize,
    seen:        usize,
    count:       usize,
    text:        String,
    current:     (Range<usize>, Vec<u8>)
}

/// `replace` (replace.md)
pub(crate) fn replace(ed: &mut Editor, arg: Arg) -> Step
{
    start(ed, arg, false)
}

/// `qreplace` (qreplace.md)
pub(crate) fn qreplace(ed: &mut Editor, arg: Arg) -> Step
{
    start(ed, arg, true)
}

/// Returns a prompt prefilled with `text`, the cursor at its start
fn prompt(prompt: &str, text: &str, then: Then) -> Step
{
    Step::Wait(Pending::Prompt { prompt: prompt.to_string(), edit: TextArg { chars: text.chars().collect(), cursor: 0 }, then })
}

/// Works out the range and asks for the search string
// ORACLE: S6, RPL-1
fn start(ed: &mut Editor, arg: Arg, query: bool) -> Step
{
    let regex = arg.count() >= 2;
    let span = match arg
    {
        | Arg::Line { first, last, .. } => Span::Lines(first, last),
        | Arg::Stream { start, end, .. } => Span::Stream(start, end),
        | Arg::Box { top, bottom, left, right, .. } => Span::Box { top, bottom, left, right },
        | _ => Span::From(ed.cursor())
    };
    prompt("Search: ", &ed.last_find.clone(), Then::ReplaceFind { span, regex, query })
}

/// Finds the next non-empty match at or after `from` and returns its range and replacement
fn next_match(regex: &Regex, replacement: &[u8], expand: bool, haystack: &[u8], from: usize) -> Option<(Range<usize>, Vec<u8>)>
{
    let mut pos = from;
    while pos <= haystack.len()
    {
        let captures = regex.captures_at(haystack, pos)?;
        let whole = captures.get(0).expect("group 0 is the whole match");
        if whole.is_empty() == false
        {
            let mut out = Vec::new();
            match expand
            {
                | true => captures.expand(replacement, &mut out),
                | false => out.extend_from_slice(replacement)
            }
            return Some((whole.range(), out));
        }
        pos = whole.end() + 1;
    }
    None
}

impl Editor
{
    /// Continues `replace` after a prompt
    pub(crate) fn replace_continue(&mut self, then: Then, answer: Answer) -> Step
    {
        match (then, answer)
        {
            | (Then::ReplaceFind { .. }, Answer::Name(find)) if find.is_empty() => self.fail("Empty search string"),
            | (Then::ReplaceFind { span, regex, query }, Answer::Name(find)) =>
            {
                self.last_find = find.clone();
                let replacement = self.last_replace.clone();
                prompt("Replace with: ", &replacement, Then::ReplaceWith { span, regex, query, find })
            }
            | (Then::ReplaceWith { span, regex, query, find }, Answer::Name(with)) =>
            {
                self.last_replace = with.clone();
                match compile(&find, regex, self.switches.case)
                {
                    | Ok(compiled) =>
                    {
                        let line = span.lines(usize::MAX).start;
                        let replace = Replace {
                            regex: compiled,
                            replacement: with.into_bytes(),
                            expand: regex,
                            span,
                            query,
                            all: false,
                            line,
                            at: 0,
                            seen: 0,
                            count: 0,
                            text: find,
                            current: (0..0, Vec::new())
                        };
                        self.replace_step(replace)
                    }
                    | Err(error) => self.fail(&error)
                }
            }
            | _ => Step::Done(false)
        }
    }

    /// Replaces matches for one slice of lines; asks, finishes or suspends for the next slice
    pub(crate) fn replace_step(&mut self, mut replace: Replace) -> Step
    {
        let lines = replace.span.lines(self.line_count());
        let mut budget = self.slice_lines;
        while replace.line < lines.end && budget > 0
        {
            let shown = self.shown_line(replace.line);
            let window = replace.span.window(replace.line, &shown);
            let from = replace.at.max(window.start);
            match next_match(&replace.regex, &replace.replacement, replace.expand, &shown[..window.end], from)
            {
                | None =>
                {
                    replace.line += 1;
                    replace.at = 0;
                    budget -= 1;
                }
                | Some((range, out)) =>
                {
                    replace.seen += 1;
                    if replace.query == true && replace.all == false
                    {
                        let col = cells::col_at(&shown, range.start);
                        self.set_cursor(Pos::new(replace.line, col));
                        self.highlight = Some((Pos::new(replace.line, col), cells::width(&shown[range.clone()])));
                        replace.current = (range, out);
                        return Step::Wait(Pending::Query(Box::new(replace)));
                    }
                    self.apply_match(&mut replace, range, out);
                }
            }
        }
        match replace.line < lines.end
        {
            | true => Step::Wait(Pending::Replace(Box::new(replace))),
            | false => self.replace_done(&replace)
        }
    }

    /// Replaces one match in the current line
    fn apply_match(&mut self, replace: &mut Replace, range: Range<usize>, out: Vec<u8>)
    {
        self.detab_line(replace.line);
        let start = self.line_start(replace.line);
        self.replace(start + range.start..start + range.end, &out);
        replace.at = range.start + out.len();
        replace.count += 1;
    }

    /// Reports the result of a replace
    fn replace_done(&mut self, replace: &Replace) -> Step
    {
        match (replace.count, replace.seen)
        {
            | (0, 0) => self.fail(&format!("{} not found", replace.text)),
            | (count, _) =>
            {
                self.info(&format!("{count} replaced"));
                Step::Done(count > 0)
            }
        }
    }

    /// Answers the `qreplace` question: y, n, p or Esc; other keys keep asking
    pub(crate) fn query_key(&mut self, mut replace: Replace, key: Key)
    {
        let (range, out) = replace.current.clone();
        let step = match key.code
        {
            | KeyCode::Char('y' | 'Y') =>
            {
                self.apply_match(&mut replace, range, out);
                self.replace_step(replace)
            }
            | KeyCode::Char('n' | 'N') =>
            {
                replace.at = range.end;
                self.replace_step(replace)
            }
            | KeyCode::Char('p' | 'P') =>
            {
                replace.all = true;
                self.apply_match(&mut replace, range, out);
                self.replace_step(replace)
            }
            | KeyCode::Esc => self.replace_done(&replace),
            | _ => Step::Wait(Pending::Query(Box::new(replace)))
        };
        self.run_step(step);
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_next_match_skips_empty_matches_and_expands()
    {
        let regex = compile("(b*)c", true, true).unwrap();
        assert_eq!(next_match(&regex, b"[$1]", true, b"abbc", 0), Some((1..4, b"[bb]".to_vec())));
        let empty = compile("x*", true, true).unwrap();
        assert_eq!(next_match(&empty, b"y", false, b"ab", 0), None);
        assert_eq!(next_match(&compile("b", false, true).unwrap(), b"$1", false, b"ab", 0), Some((1..2, b"$1".to_vec())));
    }

    #[test]
    fn test_span_lines_and_windows()
    {
        assert_eq!(Span::From(Pos::new(2, 1)).lines(5), 2..5);
        assert_eq!(Span::Lines(1, 9).lines(5), 1..5);
        assert_eq!(Span::Stream(Pos::new(0, 2), Pos::new(1, 1)).window(0, b"abcd"), 2..4);
        assert_eq!(Span::Stream(Pos::new(0, 2), Pos::new(1, 1)).window(1, b"abcd"), 0..1);
        assert_eq!(Span::Stream(Pos::new(0, 3), Pos::new(0, 1)).window(0, b"abcd"), 3..3);
        assert_eq!(Span::Box { top: 0, bottom: 1, left: 1, right: 3 }.window(0, b"abcd"), 1..3);
        assert_eq!(Span::From(Pos::new(0, 2)).window(0, b"abcd"), 2..4);
        assert_eq!(Span::From(Pos::new(0, 2)).window(1, b"abcd"), 0..4);
        assert_eq!(Span::Lines(0, 0).window(0, b"ab"), 0..2);
    }
}
