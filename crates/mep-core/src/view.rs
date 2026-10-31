//! Window state and jump scrolling (IMPLEMENTATION_PLAN.md §6.3; M guide p. 26, Table 7.2 p. 59–60)

/// A position in a file: 0-based line and display cell
///
/// The column may lie beyond the end of the line and the line beyond the end
/// of the file (virtual space, plan §5.4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos
{
    /// 0-based line
    pub line: usize,
    /// 0-based display cell
    pub col:  usize
}

impl Pos
{
    /// Creates a position
    pub fn new(line: usize, col: usize) -> Self
    {
        Self { line, col }
    }
}

/// Scrolling switches: `vscroll`, `hscroll`, `hike`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scroll
{
    /// Lines the window jumps when the cursor leaves it vertically
    pub vscroll: usize,
    /// Columns the window jumps when the cursor leaves it horizontally
    pub hscroll: usize,
    /// Cursor line distance from the top after a jump of more than `vscroll` lines
    pub hike:    usize
}

/// The part of a file shown in the window, and the cursor
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct View
{
    /// First file line shown
    pub top:    usize,
    /// First display cell shown
    pub left:   usize,
    /// Cursor position
    pub cursor: Pos
}

impl View
{
    /// Moves the window so the cursor is visible in a window of `height` lines and `width` cells
    pub fn follow(&mut self, height: usize, width: usize, scroll: Scroll)
    {
        if height == 0 || width == 0
        {
            return;
        }
        let hike = scroll.hike.min(height - 1);
        self.top = jump(self.top, self.cursor.line, height, scroll.vscroll, (hike, height - 1 - hike));
        let hscroll = clamp_step(scroll.hscroll, width);
        self.left = jump(self.left, self.cursor.col, width, hscroll, (hscroll, hscroll));
    }
}

/// Limits a jump to less than the window size, but at least one ("or less if in a small window")
// ORACLE: CMD-SCROLL-2
fn clamp_step(step: usize, size: usize) -> usize
{
    step.min(size - 1).max(1)
}

/// Returns the new first visible index so that `pos` lies in `start..start + size`
///
/// A position just outside moves the window by `step`; a farther one places
/// it `far.0` from the start (moving back) or `far.1` from the end (moving forward).
fn jump(start: usize, pos: usize, size: usize, step: usize, far: (usize, usize)) -> usize
{
    let step = clamp_step(step, size);
    if pos < start
    {
        match start - pos <= step
        {
            | true => start.saturating_sub(step),
            | false => pos.saturating_sub(far.0)
        }
    }
    else if pos >= start + size
    {
        match pos - (start + size - 1) <= step
        {
            | true => start + step,
            | false => (pos + 1 + far.1).saturating_sub(size).min(pos)
        }
    }
    else
    {
        start
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    const SCROLL: Scroll = Scroll { vscroll: 7, hscroll: 10, hike: 4 };

    fn follow(top: usize, left: usize, line: usize, col: usize) -> (usize, usize)
    {
        let mut view = View { top, left, cursor: Pos::new(line, col) };
        view.follow(20, 80, SCROLL);
        (view.top, view.left)
    }

    #[test]
    fn test_visible_cursor_keeps_window()
    {
        assert_eq!(follow(10, 5, 10, 5), (10, 5));
        assert_eq!(follow(10, 5, 29, 84), (10, 5));
    }

    #[test]
    fn test_leaving_bottom_jumps_by_vscroll()
    {
        assert_eq!(follow(0, 0, 20, 0), (7, 0));
        assert_eq!(follow(0, 0, 26, 0), (7, 0));
    }

    #[test]
    fn test_leaving_top_jumps_by_vscroll()
    {
        assert_eq!(follow(10, 0, 9, 0), (3, 0));
        assert_eq!(follow(3, 0, 2, 0), (0, 0));
    }

    #[test]
    fn test_far_jump_places_cursor_hike_lines_from_top()
    {
        assert_eq!(follow(0, 0, 100, 0), (96, 0));
        assert_eq!(follow(100, 0, 50, 0), (46, 0));
        assert_eq!(follow(100, 0, 2, 0), (0, 0));
    }

    #[test]
    fn test_leaving_sides_jumps_by_hscroll()
    {
        assert_eq!(follow(0, 0, 0, 80), (0, 10));
        assert_eq!(follow(0, 20, 0, 19), (0, 10));
        assert_eq!(follow(0, 5, 0, 0), (0, 0));
    }

    #[test]
    fn test_far_horizontal_jumps()
    {
        assert_eq!(follow(0, 0, 0, 200), (0, 131));
        assert_eq!(follow(0, 150, 0, 3), (0, 0));
        assert_eq!(follow(0, 150, 0, 40), (0, 30));
    }

    #[test]
    fn test_small_window_limits_scrolling()
    {
        let mut view = View { top: 0, left: 0, cursor: Pos::new(3, 0) };
        view.follow(3, 4, Scroll { vscroll: 7, hscroll: 10, hike: 4 });
        assert_eq!(view.top, 2);
        view.cursor = Pos::new(9, 4);
        view.follow(3, 4, Scroll { vscroll: 7, hscroll: 10, hike: 4 });
        assert_eq!((view.top, view.left), (7, 3));
    }

    #[test]
    fn test_empty_window_does_not_panic()
    {
        let mut view = View { top: 0, left: 0, cursor: Pos::new(5, 5) };
        view.follow(0, 0, SCROLL);
        assert_eq!((view.top, view.left), (0, 0));
        view.follow(3, 0, SCROLL);
        assert_eq!((view.top, view.left), (0, 0));
    }
}
