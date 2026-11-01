//! Windows (docs/spec/commands: window; M guide §4.6 p. 37, p. 109)

use crate::editor::{Arg, Editor, Step, Window};

/// Fewest lines a window keeps (p. 37)
const MIN_HEIGHT: usize = 5;

/// Fewest columns a window keeps (p. 37)
const MIN_WIDTH: usize = 10;

/// Most windows on the screen (p. 37)
const MAX_WINDOWS: usize = 8;

/// `window`: next window; `arg window`: split above and below; `arg arg window`: split side by side;
/// `meta window`: close (window.md)
// ORACLE: S10, WIN-1
pub(crate) fn window(ed: &mut Editor, arg: Arg) -> Step
{
    match (arg, ed.meta)
    {
        | (Arg::Null { count, .. }, _) => split(ed, count < 2),
        | (_, true) => close(ed),
        | (_, false) => next(ed)
    }
}

/// Moves to the next window in screen order
fn next(ed: &mut Editor) -> Step
{
    let order = ed.layout.order();
    let at = order.iter().position(|index| *index == ed.active).unwrap_or_default();
    let index = order[(at + 1) % order.len()];
    ed.activate(index);
    Step::Done(order.len() > 1)
}

/// Splits the current window at the cursor row (`stacked`) or column; the new window becomes current
fn split(ed: &mut Editor, stacked: bool) -> Step
{
    require!(ed.windows.len() < MAX_WINDOWS, ed.fail("Too many windows"));
    let (view, rect) = (ed.view(), ed.window_rects()[ed.active]);
    let (at, total, least) = match stacked
    {
        | true => (view.cursor.line - view.top, rect.height, MIN_HEIGHT),
        | false => (view.cursor.col - view.left, rect.width, MIN_WIDTH)
    };
    require!(fits(at, total, least), ed.fail("Window too small"));
    // The new window starts at the cursor; the old one keeps its text and its cursor moves next to the separator.
    let (mut old, mut lower) = (view, view);
    match stacked
    {
        | true =>
        {
            lower.top = view.cursor.line;
            old.cursor.line -= 1;
        }
        | false =>
        {
            lower.left = view.cursor.col;
            old.cursor.col -= 1;
        }
    }
    *ed.view_mut() = old;
    let new = ed.windows.len();
    ed.windows.push(Window { file: ed.current, view: lower });
    ed.layout.split(ed.active, new, stacked, at);
    ed.activate(new);
    Step::Done(true)
}

/// Returns `true` if both parts of a split at `at` keep `least` rows or columns
fn fits(at: usize, total: usize, least: usize) -> bool
{
    at >= least && total.saturating_sub(at + 1) >= least
}

/// Closes the current window; its neighbor takes its space and becomes current
fn close(ed: &mut Editor) -> Step
{
    let Some(neighbor) = ed.layout.neighbor(ed.active)
    else
    {
        return Step::Done(false);
    };
    let closed = ed.active;
    if ed.shown(ed.current) == 1
    {
        ed.state().view = ed.view();
    }
    ed.windows.remove(closed);
    ed.layout.remove(closed);
    ed.activate(neighbor - usize::from(neighbor > closed));
    Step::Done(true)
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_fits_keeps_both_parts_large_enough()
    {
        assert!(fits(5, 11, 5));
        assert!(fits(5, 10, 5) == false);
        assert!(fits(4, 20, 5) == false);
    }
}
