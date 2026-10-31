//! The screen image the core renders (IMPLEMENTATION_PLAN.md §4.2, §6.2)
//!
//! The front end diffs consecutive frames and writes only the changes.

/// One screen cell
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell
{
    /// Text shown in the cell: one character plus any zero-width characters;
    /// empty for the second cell of a wide character
    pub text: String,
    /// DOS text attribute: background in the high nibble, foreground in the low nibble
    pub attr: u8
}

/// Shape of the terminal cursor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape
{
    /// Overtype mode
    Block,
    /// Insert mode
    Bar
}

/// A full screen image
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame
{
    cols:       u16,
    rows:       u16,
    cells:      Vec<Cell>,
    /// Cursor position as (column, row)
    pub cursor: (u16, u16),
    /// Cursor shape
    pub shape:  CursorShape
}

impl Frame
{
    /// Creates a frame filled with blanks in attribute `attr`
    pub fn new(cols: u16, rows: u16, attr: u8) -> Self
    {
        let cells = vec![Cell { text: " ".into(), attr }; usize::from(cols) * usize::from(rows)];
        Self { cols, rows, cells, cursor: (0, 0), shape: CursorShape::Block }
    }

    /// Returns the cell at (`col`, `row`), or `None` outside the frame
    pub fn cell(&self, col: u16, row: u16) -> Option<&Cell>
    {
        self.index(col, row).map(|index| &self.cells[index])
    }

    /// Returns the number of columns
    pub fn cols(&self) -> u16
    {
        self.cols
    }

    /// Writes `text` taking `width` cells at (`col`, `row`); returns the column after it
    ///
    /// A wide character fills its second cell with a continuation cell. Writing
    /// stops at the right edge; a wide character that does not fit is replaced by a blank.
    pub fn put(&mut self, col: u16, row: u16, text: &str, width: u16, attr: u8) -> u16
    {
        let Some(index) = self.index(col, row)
        else
        {
            return col;
        };
        if col + width > self.cols
        {
            self.cells[index] = Cell { text: " ".into(), attr };
            return self.cols;
        }
        self.cells[index] = Cell { text: text.into(), attr };
        for continuation in 1..usize::from(width)
        {
            self.cells[index + continuation] = Cell { text: String::new(), attr };
        }
        col + width
    }

    /// Writes `text` character by character from (`col`, `row`), one cell per character
    pub fn put_str(&mut self, col: u16, row: u16, text: &str, attr: u8) -> u16
    {
        text.chars().fold(col, |col, ch| self.put(col, row, ch.encode_utf8(&mut [0; 4]), 1, attr))
    }

    /// Returns the number of rows
    pub fn rows(&self) -> u16
    {
        self.rows
    }

    /// Returns the frame's text, one line per row, with trailing blanks removed
    pub fn to_text(&self) -> String
    {
        let lines: Vec<String> =
            self.cells.chunks(usize::from(self.cols).max(1)).map(|row| row.iter().map(|cell| cell.text.as_str()).collect::<String>().trim_end().to_string()).collect();
        lines.join("\n")
    }

    /// Returns the position of a cell in `cells`, or `None` outside the frame
    fn index(&self, col: u16, row: u16) -> Option<usize>
    {
        (col < self.cols && row < self.rows).then(|| usize::from(row) * usize::from(self.cols) + usize::from(col))
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    #[test]
    fn test_new_frame_is_blank()
    {
        let frame = Frame::new(3, 2, 0x07);
        assert_eq!((frame.cols(), frame.rows()), (3, 2));
        assert_eq!(frame.cell(2, 1), Some(&Cell { text: " ".into(), attr: 0x07 }));
        assert_eq!(frame.cell(3, 0), None);
        assert_eq!(frame.cell(0, 2), None);
        assert_eq!(frame.to_text(), "\n");
        assert_eq!(frame.cursor, (0, 0));
        assert_eq!(frame.shape, CursorShape::Block);
    }

    #[test]
    fn test_put_writes_cells_and_returns_next_column()
    {
        let mut frame = Frame::new(6, 1, 0x07);
        assert_eq!(frame.put_str(1, 0, "ab", 0x70), 3);
        assert_eq!(frame.cell(1, 0).unwrap(), &Cell { text: "a".into(), attr: 0x70 });
        assert_eq!(frame.to_text(), " ab");
    }

    #[test]
    fn test_wide_character_takes_continuation_cell()
    {
        let mut frame = Frame::new(4, 1, 0x07);
        assert_eq!(frame.put(0, 0, "日", 2, 0x07), 2);
        assert_eq!(frame.cell(1, 0).unwrap().text, "");
        assert_eq!(frame.to_text(), "日");
    }

    #[test]
    fn test_writing_stops_at_right_edge()
    {
        let mut frame = Frame::new(3, 1, 0x07);
        assert_eq!(frame.put_str(1, 0, "abcd", 0x07), 3);
        assert_eq!(frame.to_text(), " ab");
        assert_eq!(frame.put(2, 0, "日", 2, 0x04), 3);
        assert_eq!(frame.cell(2, 0).unwrap(), &Cell { text: " ".into(), attr: 0x04 });
        assert_eq!(frame.put_str(0, 5, "x", 0x07), 0);
    }

    #[test]
    fn test_combining_text_stays_in_one_cell()
    {
        let mut frame = Frame::new(3, 1, 0x07);
        frame.put(0, 0, "e\u{301}", 1, 0x07);
        assert_eq!(frame.cell(0, 0).unwrap().text, "e\u{301}");
        assert_eq!(frame.to_text(), "e\u{301}");
    }
}
