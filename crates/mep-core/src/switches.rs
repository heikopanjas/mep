//! Editor switches (IMPLEMENTATION_PLAN.md §6.13; M guide §7.4 p. 57–63, Tables 7.2–7.4)
//!
//! Numeric switches take a decimal number, colors two hex digits, Boolean
//! switches are set with `name:` and cleared with `noname:`, text switches take
//! the rest of the line. An unknown switch is an error that does not abort.

use std::collections::BTreeMap;

use anyhow::anyhow;
use unicode_width::UnicodeWidthChar;

use crate::{
    Result,
    cells::{Entab, Retab, TAB_WIDTH},
    view::Scroll
};

/// What happens to the old version of a saved file (`backup`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backup
{
    /// `backup:none`: overwrite
    None,
    /// `backup:bak`: rename the old version to `.bak`
    Bak
}

/// All switch values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switches
{
    /// Prompt before quitting
    pub askexit:       bool,
    /// Prompt for Enter after a shell command (M4)
    pub askrtn:        bool,
    /// Save a modified file when switching away from it
    pub autosave:      bool,
    /// Old version handling on save
    pub backup:        Backup,
    /// Searches are case-sensitive
    pub case:          bool,
    /// Cursor shape in both modes: 0 underline, 1 block; not set: bar in insert mode, block in overtype (ORACLE CURSOR-1)
    pub cursorsize:    Option<usize>,
    /// Status line shows the cursor instead of the window position
    pub displaycursor: bool,
    /// Tab conversion of edited lines on save, 0–2
    pub entab:         usize,
    /// Start in insert mode
    pub enterinsmode:  bool,
    /// Error message color
    pub errcolor:      u8,
    /// Compile commands per file extension (M4)
    pub extmake:       BTreeMap<String, String>,
    /// Text color
    pub fgcolor:       u8,
    /// Window height from TOOLS.INI, if set
    pub height:        Option<usize>,
    /// Selection color
    pub hgcolor:       u8,
    /// Cursor line distance from the top after a far jump
    pub hike:          usize,
    /// Columns to jump when the cursor leaves the window sideways
    pub hscroll:       usize,
    /// Informative message color
    pub infcolor:      u8,
    /// Extensions to load (M4)
    pub load:          Vec<String>,
    /// File searched for undefined markers (M2)
    pub markfile:      String,
    /// Compile messages kept (M4)
    pub maxmsg:        usize,
    /// Lines counted at a time during long operations
    pub noise:         usize,
    /// Command run before overwriting a read-only file; empty asks for a new name
    pub readonly:      String,
    /// Right margin for word wrap
    pub rmargin:       usize,
    /// Auto-indent on new lines
    pub softcr:        bool,
    /// Status line color
    pub stacolor:      u8,
    /// Character code drawn for tab expansion
    pub tabdisp:       usize,
    /// `tab`/`backtab` insert/remove blanks at the cursor instead of moving (project addition)
    pub tabinsert:     bool,
    /// Distance of logical tab stops for `tab` and `backtab`
    pub tabstops:      usize,
    /// Files kept in the information file (M3)
    pub tmpsav:        usize,
    /// Character code drawn for trailing spaces; 0 is off
    pub traildisp:     usize,
    /// Keep trailing spaces of edited lines
    pub trailspace:    bool,
    /// Undo steps kept per file
    pub undocount:     usize,
    /// File tabs are `tabstops` wide and edited lines are saved with leading tabs (project addition)
    pub usetabs:       bool,
    /// Lines to jump when the cursor leaves the window vertically
    pub vscroll:       usize,
    /// Window width from TOOLS.INI, if set
    pub width:         Option<usize>,
    /// Break lines at `rmargin` while typing
    pub wordwrap:      bool
}

impl Default for Switches
{
    /// The defaults of plan §6.13
    fn default() -> Self
    {
        Self {
            askexit:       false,
            askrtn:        true,
            autosave:      true,
            backup:        Backup::None,
            case:          false,
            cursorsize:    None,
            displaycursor: false,
            entab:         1,
            enterinsmode:  false,
            errcolor:      0x04,
            extmake:       BTreeMap::new(),
            fgcolor:       0x07,
            height:        None,
            hgcolor:       0x70,
            hike:          4,
            hscroll:       10,
            infcolor:      0x06,
            load:          Vec::new(),
            markfile:      String::new(),
            maxmsg:        10,
            noise:         50,
            readonly:      String::new(),
            rmargin:       72,
            softcr:        true,
            stacolor:      0x03,
            tabdisp:       32,
            tabinsert:     false,
            tabstops:      4,
            tmpsav:        20,
            traildisp:     0,
            trailspace:    false,
            undocount:     10,
            usetabs:       false,
            vscroll:       7,
            width:         None,
            wordwrap:      false
        }
    }
}

/// How a switch parses its value
#[derive(Clone, Copy)]
enum Kind
{
    /// `name:` sets, `noname:` clears
    Flag(fn(&mut Switches) -> &mut bool),
    /// Decimal number in a range
    Number(fn(&mut Switches) -> &mut usize, usize, usize),
    /// Code of a printable one-cell character; with `true`, 0 is allowed and means off
    Char(fn(&mut Switches) -> &mut usize, bool),
    /// Optional decimal number in a range
    Optional(fn(&mut Switches) -> &mut Option<usize>, usize, usize),
    /// Two hex digits
    Color(fn(&mut Switches) -> &mut u8),
    /// Any text
    Text(fn(&mut Switches) -> &mut String),
    /// `backup`
    Backup,
    /// `extmake:ext command`
    Extmake,
    /// `load:file`
    Load,
    /// DOS-era Boolean switch, accepted without effect
    IgnoredFlag,
    /// DOS-era numeric switch, accepted without effect
    IgnoredNumber
}

/// Largest numeric switch value (docs/spec/switches.md)
const MAX: usize = 65535;

/// Every switch in alphabetical order
const TABLE: &[(&str, Kind)] = &[
    ("askexit", Kind::Flag(|s| &mut s.askexit)),
    ("askrtn", Kind::Flag(|s| &mut s.askrtn)),
    ("autosave", Kind::Flag(|s| &mut s.autosave)),
    ("backup", Kind::Backup),
    ("case", Kind::Flag(|s| &mut s.case)),
    ("cursorsize", Kind::Optional(|s| &mut s.cursorsize, 0, 1)),
    ("displaycursor", Kind::Flag(|s| &mut s.displaycursor)),
    ("entab", Kind::Number(|s| &mut s.entab, 0, 2)),
    ("enterinsmode", Kind::Flag(|s| &mut s.enterinsmode)),
    ("errcolor", Kind::Color(|s| &mut s.errcolor)),
    ("extmake", Kind::Extmake),
    ("fgcolor", Kind::Color(|s| &mut s.fgcolor)),
    ("height", Kind::Optional(|s| &mut s.height, 1, MAX)),
    ("hgcolor", Kind::Color(|s| &mut s.hgcolor)),
    ("hike", Kind::Number(|s| &mut s.hike, 0, MAX)),
    ("hscroll", Kind::Number(|s| &mut s.hscroll, 1, MAX)),
    ("infcolor", Kind::Color(|s| &mut s.infcolor)),
    ("load", Kind::Load),
    ("markfile", Kind::Text(|s| &mut s.markfile)),
    ("maxmsg", Kind::Number(|s| &mut s.maxmsg, 0, MAX)),
    ("noise", Kind::Number(|s| &mut s.noise, 0, MAX)),
    ("readonly", Kind::Text(|s| &mut s.readonly)),
    ("rmargin", Kind::Number(|s| &mut s.rmargin, 1, MAX)),
    ("savescreen", Kind::IgnoredFlag),
    ("shortnames", Kind::IgnoredFlag),
    ("softcr", Kind::Flag(|s| &mut s.softcr)),
    ("stacolor", Kind::Color(|s| &mut s.stacolor)),
    ("tabdisp", Kind::Char(|s| &mut s.tabdisp, false)),
    ("tabinsert", Kind::Flag(|s| &mut s.tabinsert)),
    ("tabstops", Kind::Number(|s| &mut s.tabstops, 1, 255)),
    ("tmpsav", Kind::Number(|s| &mut s.tmpsav, 0, MAX)),
    ("traildisp", Kind::Char(|s| &mut s.traildisp, true)),
    ("trailspace", Kind::Flag(|s| &mut s.trailspace)),
    ("undocount", Kind::Number(|s| &mut s.undocount, 0, MAX)),
    ("usetabs", Kind::Flag(|s| &mut s.usetabs)),
    ("vmbuf", Kind::IgnoredNumber),
    ("vscroll", Kind::Number(|s| &mut s.vscroll, 1, MAX)),
    ("width", Kind::Optional(|s| &mut s.width, 1, MAX)),
    ("wordwrap", Kind::Flag(|s| &mut s.wordwrap))
];

impl Switches
{
    /// Returns `true` if `name` is a switch, including `no` forms of Boolean switches
    pub fn is_switch(name: &str) -> bool
    {
        lookup(&name.to_lowercase()).is_some()
    }

    /// Sets switch `name` from the text after the colon
    ///
    /// # Errors
    ///
    /// Returns a message for an unknown switch or an invalid value; the switch keeps its value.
    pub fn set(&mut self, name: &str, value: &str) -> Result<()>
    {
        let fail = |message: String| anyhow!("{name}: {message}");
        let value = value.trim();
        let (kind, cleared) = lookup(&name.to_lowercase()).ok_or_else(|| fail("unknown switch".into()))?;
        match kind
        {
            | Kind::Flag(field) =>
            {
                require!(value.is_empty(), Err(fail("a Boolean switch takes no value".into())));
                *field(self) = cleared == false;
            }
            | Kind::IgnoredFlag => require!(value.is_empty(), Err(fail("a Boolean switch takes no value".into()))),
            | Kind::Number(field, min, max) => *field(self) = number(value, min, max).map_err(fail)?,
            | Kind::Char(field, zero) => *field(self) = character(value, zero).map_err(fail)?,
            | Kind::IgnoredNumber => drop(number(value, 0, MAX).map_err(fail)?),
            | Kind::Optional(field, min, max) => *field(self) = Some(number(value, min, max).map_err(fail)?),
            | Kind::Color(field) =>
            {
                let color = (1..=2).contains(&value.len()).then(|| u8::from_str_radix(value, 16).ok()).flatten();
                *field(self) = color.ok_or_else(|| fail("expected two hex digits".into()))?;
            }
            | Kind::Text(field) => *field(self) = value.to_string(),
            | Kind::Backup =>
            {
                self.backup = match value.to_lowercase().as_str()
                {
                    | "none" => Backup::None,
                    | "bak" => Backup::Bak,
                    | _ => return Err(fail("expected none or bak".into()))
                };
            }
            | Kind::Extmake =>
            {
                let (extension, command) = value
                    .split_once(char::is_whitespace)
                    .filter(|(_, command)| command.trim().is_empty() == false)
                    .ok_or_else(|| fail("expected an extension and a command".into()))?;
                self.extmake.insert(extension.to_lowercase(), command.trim().to_string());
            }
            | Kind::Load =>
            {
                require!(value.is_empty() == false, Err(fail("expected a file name".into())));
                self.load.push(value.to_string());
            }
        }
        Ok(())
    }

    /// Returns the width of a tab character in the file: `tabstops` with `usetabs`, else 8
    // ORACLE: TAB-1
    pub fn tab_width(&self) -> usize
    {
        match self.usetabs
        {
            | true => self.tabstops,
            | false => TAB_WIDTH
        }
    }

    /// Returns how edited lines are re-tabbed on save
    pub fn retab(&self) -> Retab
    {
        let mode = match self.entab
        {
            | 0 => Entab::Never,
            | 1 => Entab::OutsideQuotes,
            | _ => Entab::Everywhere
        };
        Retab { mode, tab: self.tab_width(), indent: self.usetabs }
    }

    /// Returns the scrolling switches
    pub fn scroll(&self) -> Scroll
    {
        Scroll { vscroll: self.vscroll, hscroll: self.hscroll, hike: self.hike }
    }
}

/// Finds a switch; the second value is `true` for the `no` form of a Boolean switch
fn lookup(name: &str) -> Option<(Kind, bool)>
{
    let find = |name: &str| TABLE.iter().find(|(entry, _)| *entry == name).map(|(_, kind)| *kind);
    find(name)
        .map(|kind| (kind, false))
        .or_else(|| name.strip_prefix("no").and_then(find).filter(|kind| matches!(kind, Kind::Flag(_) | Kind::IgnoredFlag)).map(|kind| (kind, true)))
}

/// Parses a decimal number within `min..=max`
fn number(value: &str, min: usize, max: usize) -> std::result::Result<usize, String>
{
    let number = value.parse::<usize>().map_err(|_| "expected a number".to_string())?;
    require!((min..=max).contains(&number), Err(format!("value must be between {min} and {max}")));
    Ok(number)
}

/// Parses the code of a printable one-cell character: 32–126 or 160 and above
fn character(value: &str, zero: bool) -> std::result::Result<usize, String>
{
    let code = number(value, 0, 0x10_ffff)?;
    let printable = char::from_u32(code as u32).is_some_and(|ch| (32..=126).contains(&code) || (code >= 160 && ch.width() == Some(1)));
    require!(printable || (zero && code == 0), Err("expected the code of a printable character".to_string()));
    Ok(code)
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    /// (switch, valid value, invalid value) for every switch
    const CASES: &[(&str, &str, &str)] = &[
        ("askexit", "", "yes"),
        ("askrtn", "", "1"),
        ("autosave", "", "x"),
        ("backup", "bak", "undel"),
        ("case", "", "on"),
        ("cursorsize", "0", "2"),
        ("displaycursor", "", "x"),
        ("entab", "2", "3"),
        ("enterinsmode", "", "x"),
        ("errcolor", "0E", "G0"),
        ("extmake", "c cl /c %s", "c"),
        ("fgcolor", "17", "123"),
        ("height", "41", "0"),
        ("hgcolor", "7", ""),
        ("hike", "0", "65536"),
        ("hscroll", "20", "0"),
        ("infcolor", "6", "x"),
        ("load", "filter.so", ""),
        ("maxmsg", "0", "x"),
        ("noise", "0", "many"),
        ("rmargin", "60", "0"),
        ("savescreen", "", "x"),
        ("shortnames", "", "x"),
        ("softcr", "", "x"),
        ("stacolor", "1F", "1FF"),
        ("tabdisp", "183", "127"),
        ("tabinsert", "", "x"),
        ("tabstops", "8", "256"),
        ("tmpsav", "0", "x"),
        ("traildisp", "0", "31"),
        ("trailspace", "", "x"),
        ("undocount", "0", "x"),
        ("usetabs", "", "x"),
        ("vmbuf", "128", "x"),
        ("vscroll", "1", "0"),
        ("width", "132", "x"),
        ("wordwrap", "", "x")
    ];

    #[test]
    fn test_defaults_match_plan()
    {
        let s = Switches::default();
        assert_eq!((s.undocount, s.displaycursor, s.tabstops, s.tabdisp, s.entab, s.traildisp), (10, false, 4, 32, 1, 0));
        assert_eq!((s.trailspace, s.case, s.enterinsmode, s.softcr, s.wordwrap, s.rmargin), (false, false, false, true, false, 72));
        assert!(s.tabinsert == false);
        assert_eq!((s.autosave, s.askexit, s.backup, s.readonly.as_str()), (true, false, Backup::None, ""));
        assert_eq!((s.vscroll, s.hscroll, s.hike, s.height, s.width), (7, 10, 4, None, None));
        assert_eq!((s.fgcolor, s.hgcolor, s.stacolor, s.infcolor, s.errcolor, s.noise), (0x07, 0x70, 0x03, 0x06, 0x04, 50));
        assert_eq!((s.tmpsav, s.maxmsg, s.askrtn, s.markfile.as_str(), s.load.len(), s.extmake.len()), (20, 10, true, "", 0, 0));
    }

    #[test]
    fn test_every_switch_accepts_valid_and_rejects_invalid_value()
    {
        // Free text switches (markfile, readonly) have no invalid value.
        assert_eq!(CASES.len() + 2, TABLE.len());
        for (name, valid, invalid) in CASES
        {
            let mut switches = Switches::default();
            assert!(switches.set(name, valid).is_ok(), "{name}:{valid}");
            let before = switches.clone();
            let error = switches.set(name, invalid).unwrap_err().to_string();
            assert!(error.starts_with(&format!("{name}:")), "{error}");
            assert_eq!(switches, before, "{name}");
        }
    }

    #[test]
    fn test_values_are_stored()
    {
        let mut s = Switches::default();
        s.set("case", "").unwrap();
        s.set("noautosave", "").unwrap();
        s.set("hscroll", "20").unwrap();
        s.set("errcolor", "0E").unwrap();
        s.set("hgcolor", "f").unwrap();
        s.set("backup", "BAK").unwrap();
        s.set("height", "41").unwrap();
        s.set("readonly", "attrib -r").unwrap();
        s.set("extmake", "C cl /c %s").unwrap();
        s.set("extmake", "asm masm -Mx %s;").unwrap();
        s.set("load", "filter.so").unwrap();
        s.set("vmbuf", "128").unwrap();
        assert!(s.case);
        assert!(s.autosave == false);
        assert_eq!((s.hscroll, s.errcolor, s.hgcolor, s.backup, s.height), (20, 0x0e, 0x0f, Backup::Bak, Some(41)));
        assert_eq!(s.readonly, "attrib -r");
        assert_eq!(s.extmake.get("c").unwrap(), "cl /c %s");
        assert_eq!(s.extmake.get("asm").unwrap(), "masm -Mx %s;");
        assert_eq!(s.load, vec!["filter.so"]);
        s.set("backup", "none").unwrap();
        assert_eq!(s.backup, Backup::None);
    }

    #[test]
    fn test_boolean_switches_take_no_value_in_no_form()
    {
        let mut s = Switches::default();
        assert_eq!(s.set("nocase", "x").unwrap_err().to_string(), "nocase: a Boolean switch takes no value");
    }

    #[test]
    fn test_unknown_switches_are_errors()
    {
        let mut s = Switches::default();
        assert_eq!(s.set("bogus", "1").unwrap_err().to_string(), "bogus: unknown switch");
        assert_eq!(s.set("nohike", "").unwrap_err().to_string(), "nohike: unknown switch");
        assert!(Switches::is_switch("Case"));
        assert!(Switches::is_switch("nocase"));
        assert!(Switches::is_switch("hike"));
        assert!(Switches::is_switch("nohike") == false);
        assert!(Switches::is_switch("exit") == false);
    }

    #[test]
    fn test_error_messages()
    {
        let mut s = Switches::default();
        assert_eq!(s.set("hike", "x").unwrap_err().to_string(), "hike: expected a number");
        assert_eq!(s.set("entab", "3").unwrap_err().to_string(), "entab: value must be between 0 and 2");
        assert_eq!(s.set("vscroll", "0").unwrap_err().to_string(), "vscroll: value must be between 1 and 65535");
        assert_eq!(s.set("fgcolor", "123").unwrap_err().to_string(), "fgcolor: expected two hex digits");
        assert_eq!(s.set("backup", "undel").unwrap_err().to_string(), "backup: expected none or bak");
        assert_eq!(s.set("extmake", "c").unwrap_err().to_string(), "extmake: expected an extension and a command");
        assert_eq!(s.set("load", "").unwrap_err().to_string(), "load: expected a file name");
        assert_eq!(s.set("height", "0").unwrap_err().to_string(), "height: value must be between 1 and 65535");
        assert_eq!(s.set("tabdisp", "0").unwrap_err().to_string(), "tabdisp: expected the code of a printable character");
        assert_eq!(s.set("tabdisp", "1114112").unwrap_err().to_string(), "tabdisp: value must be between 0 and 1114111");
        assert_eq!(s.set("tabdisp", "55296").unwrap_err().to_string(), "tabdisp: expected the code of a printable character");
        assert_eq!(s.set("tabdisp", "12354").unwrap_err().to_string(), "tabdisp: expected the code of a printable character");
    }

    #[test]
    fn test_text_switches_take_any_text_and_clear_when_empty()
    {
        let mut s = Switches::default();
        s.set("readonly", " attrib -r ").unwrap();
        s.set("markfile", "marks.txt").unwrap();
        assert_eq!((s.readonly.as_str(), s.markfile.as_str()), ("attrib -r", "marks.txt"));
        s.set("readonly", "").unwrap();
        s.set("markfile", "").unwrap();
        assert_eq!((s.readonly.as_str(), s.markfile.as_str()), ("", ""));
    }

    #[test]
    fn test_values_are_trimmed_and_characters_stored()
    {
        let mut s = Switches::default();
        s.set("hike", " 9 ").unwrap();
        s.set("case", "  ").unwrap();
        assert_eq!((s.hike, s.case), (9, true));
        s.set("tabdisp", "126").unwrap();
        s.set("traildisp", "8226").unwrap();
        assert_eq!((s.tabdisp, s.traildisp), (126, 8226));
    }

    #[test]
    fn test_retab_tab_width_and_scroll()
    {
        let mut s = Switches::default();
        assert_eq!(s.retab(), Retab { mode: Entab::OutsideQuotes, tab: 8, indent: false });
        s.set("entab", "0").unwrap();
        assert_eq!(s.retab().mode, Entab::Never);
        s.set("entab", "2").unwrap();
        assert_eq!(s.retab().mode, Entab::Everywhere);
        s.set("usetabs", "").unwrap();
        assert_eq!((s.tab_width(), s.retab().tab, s.retab().indent), (4, 4, true));
        assert_eq!(s.scroll(), Scroll { vscroll: 7, hscroll: 10, hike: 4 });
    }
}
