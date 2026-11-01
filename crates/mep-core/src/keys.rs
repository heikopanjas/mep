//! Keys, key names and the key map (IMPLEMENTATION_PLAN.md §6.8; M guide p. 46–47, Table A.2 p. 90–92)

use std::{collections::HashMap, fmt};

use anyhow::anyhow;

use crate::Result;

/// A key without modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyCode
{
    /// A character key; letters are lowercase when combined with Ctrl or Alt
    Char(char),
    /// Function key F1–F12
    F(u8),
    /// Home
    Home,
    /// End
    End,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Page up
    PgUp,
    /// Page down
    PgDn,
    /// Insert
    Ins,
    /// Delete
    Del,
    /// Backspace
    Bksp,
    /// Tab
    Tab,
    /// Escape
    Esc,
    /// Enter
    Enter,
    /// Keypad digit 0–9
    NumDigit(u8),
    /// Keypad +
    NumPlus,
    /// Keypad -
    NumMinus
}

/// A key with its modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key
{
    /// The key itself
    pub code:  KeyCode,
    /// Ctrl held
    pub ctrl:  bool,
    /// Alt held
    pub alt:   bool,
    /// Shift held
    pub shift: bool,
    /// Command (macOS) or Super held; terminals report it only with the kitty keyboard protocol
    pub cmd:   bool
}

/// Names of the keys without modifiers, besides characters and F-keys
const NAMED: &[(&str, KeyCode)] = &[
    ("home", KeyCode::Home),
    ("end", KeyCode::End),
    ("left", KeyCode::Left),
    ("right", KeyCode::Right),
    ("up", KeyCode::Up),
    ("down", KeyCode::Down),
    ("pgup", KeyCode::PgUp),
    ("pgdn", KeyCode::PgDn),
    ("ins", KeyCode::Ins),
    ("del", KeyCode::Del),
    ("bksp", KeyCode::Bksp),
    ("tab", KeyCode::Tab),
    ("esc", KeyCode::Esc),
    ("enter", KeyCode::Enter),
    ("space", KeyCode::Char(' ')),
    ("num+", KeyCode::NumPlus),
    ("num-", KeyCode::NumMinus)
];

/// Modifier prefixes in the order the printer writes them; `cmd+` is a project addition (ORACLE KEY-1)
const MODIFIERS: [&str; 4] = ["ctrl+", "alt+", "shift+", "cmd+"];

impl Key
{
    /// Creates a key without modifiers
    pub fn plain(code: KeyCode) -> Self
    {
        Self { code, ctrl: false, alt: false, shift: false, cmd: false }
    }

    /// Parses a key name such as `alt+a`, `ctrl+pgdn`, `shift+f2`, `cmd+c` or `num+`; case does not matter
    ///
    /// # Errors
    ///
    /// Returns an error naming the text if it is not a key name.
    // ORACLE: KEY-1
    pub fn parse(name: &str) -> Result<Self>
    {
        let lower = name.to_lowercase();
        let mut rest = lower.as_str();
        let mut held = [false; 4];
        // A modifier needs a key after it, so "alt++" is Alt with the + key.
        while let Some(index) = MODIFIERS.iter().position(|prefix| rest.len() > prefix.len() && rest.starts_with(prefix))
        {
            held[index] = true;
            rest = &rest[MODIFIERS[index].len()..];
        }
        let code = code_of(rest).ok_or_else(|| anyhow!("unknown key name '{name}'"))?;
        let [ctrl, alt, shift, cmd] = held;
        Ok(Self { code, ctrl, alt, shift, cmd })
    }
}

impl fmt::Display for Key
{
    /// Writes the canonical key name, e.g. `ctrl+alt+x`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        let prefixes: String =
            [self.ctrl, self.alt, self.shift, self.cmd].into_iter().zip(MODIFIERS).filter(|(held, _)| *held == true).map(|(_, prefix)| prefix).collect();
        let name = match self.code
        {
            | KeyCode::F(number) => format!("f{number}"),
            | KeyCode::NumDigit(digit) => format!("num{digit}"),
            | KeyCode::Char(ch) if ch != ' ' => ch.to_string(),
            | code => NAMED.iter().find(|(_, named)| *named == code).map_or("", |(name, _)| name).to_string()
        };
        write!(f, "{prefixes}{name}")
    }
}

/// Returns the key code for a name without modifiers
fn code_of(name: &str) -> Option<KeyCode>
{
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next())
    {
        return Some(KeyCode::Char(ch));
    }
    let digit = |text: &str, range: std::ops::RangeInclusive<u8>| text.parse::<u8>().ok().filter(|value| range.contains(value));
    NAMED
        .iter()
        .find(|(named, _)| *named == name)
        .map(|(_, code)| *code)
        .or_else(|| name.strip_prefix("num").filter(|rest| rest.len() == 1).and_then(|rest| digit(rest, 0..=9)).map(KeyCode::NumDigit))
        .or_else(|| name.strip_prefix('f').filter(|rest| rest.starts_with('+') == false).and_then(|rest| digit(rest, 1..=12)).map(KeyCode::F))
}

/// The default MEP key assignments: Default column of Table A.2 (M guide p. 90–92)
pub const DEFAULT_KEYS: &[(&str, &str)] = &[
    ("arg", "alt+a"),
    ("argcompile", "f5"),
    ("assign", "alt+="),
    ("backtab", "shift+tab"),
    ("begline", "home"),
    ("cancel", "esc"),
    ("cdelete", "ctrl+g"),
    ("compile", "shift+f3"),
    ("copy", "ctrl+ins"),
    ("copy", "num+"),
    ("down", "down"),
    ("down", "ctrl+x"),
    ("emacscdel", "bksp"),
    ("emacsnewl", "enter"),
    ("endline", "end"),
    ("execute", "f7"),
    ("exit", "f8"),
    ("help", "f1"),
    ("home", "ctrl+home"),
    ("information", "shift+f1"),
    ("initialize", "shift+f8"),
    ("insertmode", "ins"),
    ("insertmode", "ctrl+v"),
    ("lasttext", "ctrl+o"),
    ("ldelete", "ctrl+y"),
    ("left", "left"),
    ("left", "ctrl+s"),
    ("linsert", "ctrl+n"),
    ("mark", "ctrl+m"),
    ("meta", "f9"),
    ("mlines", "ctrl+w"),
    ("mpage", "pgup"),
    ("mpage", "ctrl+r"),
    ("mpara", "ctrl+pgup"),
    ("msearch", "f4"),
    ("mword", "ctrl+left"),
    ("mword", "ctrl+a"),
    ("paste", "shift+ins"),
    ("pbal", "ctrl+["),
    ("plines", "ctrl+z"),
    ("ppage", "pgdn"),
    ("ppage", "ctrl+c"),
    ("ppara", "ctrl+pgdn"),
    ("psearch", "f3"),
    ("pword", "ctrl+right"),
    ("pword", "ctrl+f"),
    ("qreplace", "ctrl+\\"),
    ("quote", "ctrl+p"),
    ("refresh", "shift+f7"),
    ("replace", "ctrl+l"),
    ("right", "right"),
    ("right", "ctrl+d"),
    ("sdelete", "del"),
    ("setfile", "f2"),
    ("setwindow", "ctrl+]"),
    ("shell", "shift+f9"),
    ("sinsert", "ctrl+j"),
    ("tab", "tab"),
    ("undo", "alt+bksp"),
    ("up", "up"),
    ("up", "ctrl+e"),
    ("window", "f6")
];

/// Assignment of keys to function and macro names
#[derive(Debug, Clone)]
pub struct Keymap
{
    map: HashMap<Key, String>
}

impl Keymap
{
    /// Assigns `key` to the function or macro `name`, replacing its previous assignment
    pub fn assign(&mut self, key: Key, name: &str)
    {
        self.map.insert(key, name.to_string());
    }

    /// Returns the function or macro name for `key`
    ///
    /// Keys without an assignment run `graphic` if they type a printable
    /// character and `unassigned` otherwise.
    pub fn lookup(&self, key: Key) -> &str
    {
        self.map.get(&key).map_or_else(
            || match key.code
            {
                | KeyCode::Char(ch) if key.ctrl == false && key.alt == false && key.cmd == false && ch.is_control() == false => "graphic",
                | _ => "unassigned"
            },
            String::as_str
        )
    }
}

impl Default for Keymap
{
    /// The default MEP key map
    fn default() -> Self
    {
        let map = DEFAULT_KEYS.iter().filter_map(|(function, name)| Key::parse(name).ok().map(|key| (key, function.to_string()))).collect();
        Self { map }
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use proptest::prelude::*;

    use super::*;

    fn key(code: KeyCode, ctrl: bool, alt: bool, shift: bool) -> Key
    {
        Key { code, ctrl, alt, shift, cmd: false }
    }

    #[test]
    fn test_every_default_key_name_round_trips()
    {
        for (_, name) in DEFAULT_KEYS
        {
            let key = Key::parse(name).unwrap();
            assert_eq!(key.to_string(), *name);
            assert_eq!(Key::parse(&key.to_string()).unwrap(), key);
        }
    }

    #[test]
    fn test_parse_examples()
    {
        assert_eq!(Key::parse("ALT+A").unwrap(), key(KeyCode::Char('a'), false, true, false));
        assert_eq!(Key::parse("Ctrl+PgDn").unwrap(), key(KeyCode::PgDn, true, false, false));
        assert_eq!(Key::parse("shift+f12").unwrap(), key(KeyCode::F(12), false, false, true));
        assert_eq!(Key::parse("alt++").unwrap(), key(KeyCode::Char('+'), false, true, false));
        assert_eq!(Key::parse("ctrl+-").unwrap(), key(KeyCode::Char('-'), true, false, false));
        assert_eq!(Key::parse("NUM4").unwrap(), Key::plain(KeyCode::NumDigit(4)));
        assert_eq!(Key::parse("num-").unwrap(), Key::plain(KeyCode::NumMinus));
        assert_eq!(Key::parse("alt+ctrl+x").unwrap(), key(KeyCode::Char('x'), true, true, false));
        assert_eq!(Key::parse("space").unwrap(), Key::plain(KeyCode::Char(' ')));
        assert_eq!(Key::parse("7").unwrap(), Key::plain(KeyCode::Char('7')));
        let cmd = Key::parse("Cmd+Shift+C").unwrap();
        assert_eq!(cmd, Key { cmd: true, ..key(KeyCode::Char('c'), false, false, true) });
        assert_eq!(cmd.to_string(), "shift+cmd+c");
        assert!(Key::parse("cmd+").is_err());
    }

    #[test]
    fn test_print_examples()
    {
        assert_eq!(key(KeyCode::Char('x'), true, true, true).to_string(), "ctrl+alt+shift+x");
        assert_eq!(Key::plain(KeyCode::NumDigit(0)).to_string(), "num0");
        assert_eq!(Key::plain(KeyCode::Char(' ')).to_string(), "space");
        assert_eq!(Key::plain(KeyCode::F(1)).to_string(), "f1");
    }

    #[test]
    fn test_parse_rejects_garbage()
    {
        for name in ["", "alt+", "ctrl+foo", "f0", "f13", "fx", "num*", "num10", "ab", "shift+shift", "ctrl+ab"]
        {
            let error = Key::parse(name).unwrap_err();
            assert_eq!(error.to_string(), format!("unknown key name '{name}'"));
        }
    }

    #[test]
    fn test_default_keymap_lookups()
    {
        let keymap = Keymap::default();
        assert_eq!(keymap.lookup(Key::parse("alt+a").unwrap()), "arg");
        assert_eq!(keymap.lookup(Key::parse("num+").unwrap()), "copy");
        assert_eq!(keymap.lookup(Key::parse("ctrl+ins").unwrap()), "copy");
        assert_eq!(keymap.lookup(Key::parse("ctrl+\\").unwrap()), "qreplace");
        assert_eq!(keymap.lookup(Key::plain(KeyCode::Char('x'))), "graphic");
        assert_eq!(keymap.lookup(Key::plain(KeyCode::Char('Ä'))), "graphic");
        assert_eq!(keymap.lookup(Key::parse("alt+p").unwrap()), "unassigned");
        assert_eq!(keymap.lookup(Key::parse("ctrl+q").unwrap()), "unassigned");
        assert_eq!(keymap.lookup(Key::plain(KeyCode::Char('\u{7}'))), "unassigned");
        assert_eq!(keymap.lookup(Key::parse("f12").unwrap()), "unassigned");
        assert_eq!(keymap.lookup(Key::parse("cmd+x").unwrap()), "unassigned");
    }

    #[test]
    fn test_assign_replaces_and_unassigns()
    {
        let mut keymap = Keymap::default();
        keymap.assign(Key::parse("ctrl+b").unwrap(), "savecur");
        assert_eq!(keymap.lookup(Key::parse("ctrl+b").unwrap()), "savecur");
        keymap.assign(Key::parse("x").unwrap(), "unassigned");
        assert_eq!(keymap.lookup(Key::parse("x").unwrap()), "unassigned");
        keymap.assign(Key::parse("ctrl+l").unwrap(), "graphic");
        assert_eq!(keymap.lookup(Key::parse("ctrl+l").unwrap()), "graphic");
    }

    fn code() -> impl Strategy<Value = KeyCode>
    {
        prop_oneof![
            prop::sample::select("abcdefghijklmnopqrstuvwxyz0123456789`~,.<>/?;'\"[]{}\\|-=_+ ".chars().collect::<Vec<_>>()).prop_map(KeyCode::Char),
            (1u8..=12).prop_map(KeyCode::F),
            (0u8..=9).prop_map(KeyCode::NumDigit),
            prop::sample::select(NAMED.iter().map(|(_, code)| *code).collect::<Vec<_>>())
        ]
    }

    proptest! {
        #[test]
        fn prop_key_names_round_trip(code in code(), ctrl in any::<bool>(), alt in any::<bool>(), shift in any::<bool>())
        {
            let key = key(code, ctrl, alt, shift);
            prop_assert_eq!(Key::parse(&key.to_string()).unwrap(), key);
        }
    }
}
