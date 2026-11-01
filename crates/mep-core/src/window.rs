//! Window layout (docs/spec/commands/window.md; M guide §4.6 p. 37)
//!
//! The text area is a tree of splits. Each leaf is a window; closing a window
//! gives its space to its sibling.

/// A rectangle of screen cells
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect
{
    /// First row
    pub top:    usize,
    /// First column
    pub left:   usize,
    /// Number of rows
    pub height: usize,
    /// Number of columns
    pub width:  usize
}

/// Split tree of the windows; leaves hold indices into the editor's window list
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout
{
    /// One window
    Leaf(usize),
    /// Two parts with a one-cell separator; `stacked` puts them above each other
    Split
    {
        /// Above each other (separator row) instead of side by side (separator column)
        stacked: bool,
        /// Rows or columns of the first part
        size:    usize,
        /// Upper or left part
        first:   Box<Layout>,
        /// Lower or right part
        second:  Box<Layout>
    }
}

/// A window's place on the screen, or a separator line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile
{
    /// Window with its index
    Window(usize, Rect),
    /// Separator; `true` for a row between stacked windows
    Separator(Rect, bool)
}

impl Layout
{
    /// Places the windows and separators in `area`
    pub fn tiles(&self, area: Rect) -> Vec<Tile>
    {
        let mut tiles = Vec::new();
        self.place(area, &mut tiles);
        tiles
    }

    fn place(&self, area: Rect, tiles: &mut Vec<Tile>)
    {
        match self
        {
            | Self::Leaf(index) => tiles.push(Tile::Window(*index, area)),
            | Self::Split { stacked, size, first, second } =>
            {
                let total = if *stacked
                {
                    area.height
                }
                else
                {
                    area.width
                };
                // A shrunken terminal shrinks the first part, keeping room for the separator.
                let first_size = (*size).min(total.saturating_sub(2));
                let second_size = total.saturating_sub(first_size + 1);
                let (one, separator, two) = match stacked
                {
                    | true => (Rect { height: first_size, ..area }, Rect { top: area.top + first_size, height: 1, ..area }, Rect {
                        top: area.top + first_size + 1,
                        height: second_size,
                        ..area
                    }),
                    | false => (Rect { width: first_size, ..area }, Rect { left: area.left + first_size, width: 1, ..area }, Rect {
                        left: area.left + first_size + 1,
                        width: second_size,
                        ..area
                    })
                };
                first.place(one, tiles);
                tiles.push(Tile::Separator(separator, *stacked));
                second.place(two, tiles);
            }
        }
    }

    /// Returns the window indices in screen order (left to right, top to bottom)
    pub fn order(&self) -> Vec<usize>
    {
        match self
        {
            | Self::Leaf(index) => vec![*index],
            | Self::Split { first, second, .. } => [first.order(), second.order()].concat()
        }
    }

    /// Returns the first window of the part that shares a split with window `index`
    pub fn neighbor(&self, index: usize) -> Option<usize>
    {
        match self
        {
            | Self::Leaf(_) => None,
            | Self::Split { first, second, .. } if **first == Self::Leaf(index) => second.order().first().copied(),
            | Self::Split { first, second, .. } if **second == Self::Leaf(index) => first.order().first().copied(),
            | Self::Split { first, second, .. } => first.neighbor(index).or_else(|| second.neighbor(index))
        }
    }

    /// Replaces window `index` with a split of it and window `new`
    pub fn split(&mut self, index: usize, new: usize, stacked: bool, size: usize)
    {
        match self
        {
            | Self::Leaf(leaf) if *leaf == index =>
            {
                *self = Self::Split { stacked, size, first: Box::new(Self::Leaf(index)), second: Box::new(Self::Leaf(new)) };
            }
            | Self::Leaf(_) =>
            {}
            | Self::Split { first, second, .. } =>
            {
                first.split(index, new, stacked, size);
                second.split(index, new, stacked, size);
            }
        }
    }

    /// Removes window `index`; its sibling takes its place and higher indices move down by one
    pub fn remove(&mut self, index: usize)
    {
        if let Self::Split { first, second, .. } = self
        {
            if **first == Self::Leaf(index)
            {
                *self = (**second).clone();
            }
            else if **second == Self::Leaf(index)
            {
                *self = (**first).clone();
            }
            else
            {
                first.remove(index);
                second.remove(index);
            }
        }
        self.renumber(index);
    }

    /// Moves window indices above `removed` down by one
    fn renumber(&mut self, removed: usize)
    {
        match self
        {
            | Self::Leaf(leaf) if *leaf > removed => *leaf -= 1,
            | Self::Leaf(_) =>
            {}
            | Self::Split { first, second, .. } =>
            {
                first.renumber(removed);
                second.renumber(removed);
            }
        }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    const AREA: Rect = Rect { top: 0, left: 0, height: 20, width: 80 };

    #[test]
    fn test_single_window_fills_the_area()
    {
        assert_eq!(Layout::Leaf(0).tiles(AREA), vec![Tile::Window(0, AREA)]);
        assert_eq!(Layout::Leaf(0).order(), vec![0]);
    }

    #[test]
    fn test_stacked_and_side_by_side_splits()
    {
        let mut layout = Layout::Leaf(0);
        layout.split(0, 1, true, 8);
        assert_eq!(layout.tiles(AREA), vec![
            Tile::Window(0, Rect { top: 0, left: 0, height: 8, width: 80 }),
            Tile::Separator(Rect { top: 8, left: 0, height: 1, width: 80 }, true),
            Tile::Window(1, Rect { top: 9, left: 0, height: 11, width: 80 })
        ]);
        layout.split(1, 2, false, 30);
        assert_eq!(layout.order(), vec![0, 1, 2]);
        assert_eq!(layout.tiles(AREA)[4], Tile::Window(2, Rect { top: 9, left: 31, height: 11, width: 49 }));
        layout.split(7, 3, false, 5);
        assert_eq!(layout.order(), vec![0, 1, 2]);
    }

    #[test]
    fn test_small_area_shrinks_the_first_part()
    {
        let mut layout = Layout::Leaf(0);
        layout.split(0, 1, true, 15);
        let tiles = layout.tiles(Rect { height: 6, ..AREA });
        assert_eq!(tiles[0], Tile::Window(0, Rect { top: 0, left: 0, height: 4, width: 80 }));
        assert_eq!(tiles[2], Tile::Window(1, Rect { top: 5, left: 0, height: 1, width: 80 }));
    }

    #[test]
    fn test_remove_gives_space_to_the_sibling_and_renumbers()
    {
        let mut layout = Layout::Leaf(0);
        layout.split(0, 1, true, 8);
        layout.split(1, 2, false, 30);
        layout.remove(1);
        assert_eq!(layout.order(), vec![0, 1]);
        layout.remove(0);
        assert_eq!(layout, Layout::Leaf(0));
        let mut layout = Layout::Leaf(0);
        layout.split(0, 1, true, 8);
        layout.remove(0);
        assert_eq!(layout, Layout::Leaf(0));
        layout.remove(0);
        assert_eq!(layout, Layout::Leaf(0));
    }

    #[test]
    fn test_neighbor_is_the_first_window_of_the_sibling_part()
    {
        let mut layout = Layout::Leaf(0);
        assert_eq!(layout.neighbor(0), None);
        layout.split(0, 1, true, 8);
        layout.split(1, 2, false, 30);
        assert_eq!(layout.neighbor(0), Some(1));
        assert_eq!(layout.neighbor(1), Some(2));
        assert_eq!(layout.neighbor(2), Some(1));
        assert_eq!(layout.neighbor(5), None);
    }
}
