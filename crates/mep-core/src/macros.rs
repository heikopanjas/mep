//! Macro definitions (IMPLEMENTATION_PLAN.md §6.14; M guide §6.3 p. 48–53, Table 6.2 p. 52)

use std::collections::HashMap;

use anyhow::anyhow;

use crate::Result;

/// Most macros defined at one time (M guide p. 49)
pub const MAX_MACROS: usize = 1024;

/// Condition of a macro jump
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jump
{
    /// `=>`: always
    Always,
    /// `+>`: if the previous function returned true
    IfTrue,
    /// `->`: if the previous function returned false
    IfFalse
}

/// One element of a macro definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item
{
    /// A function or macro name, lowercase
    Call(String),
    /// A quoted string, entered as if typed
    Text(String),
    /// `:>label`
    Label(String),
    /// A jump to a label, or out of the macro without one
    Jump(Jump, Option<String>)
}

/// Parses a macro definition body such as `Meta Up Arg "11" Plines Begline`
///
/// Names and labels are case-insensitive and stored lowercase.
///
/// # Errors
///
/// Returns an error for an unterminated string.
pub fn parse(body: &str) -> Result<Vec<Item>>
{
    let mut items = Vec::new();
    let mut rest = body.trim_start();
    while rest.is_empty() == false
    {
        if let Some(quoted) = rest.strip_prefix('"')
        {
            let end = quoted.find('"').ok_or_else(|| anyhow!("unterminated string in macro"))?;
            items.push(Item::Text(quoted[..end].to_string()));
            rest = &quoted[end + 1..];
        }
        else
        {
            let end = rest.find(|ch: char| ch.is_whitespace() || ch == '"').unwrap_or(rest.len());
            items.push(item(&rest[..end].to_lowercase()));
            rest = &rest[end..];
        }
        rest = rest.trim_start();
    }
    Ok(items)
}

/// Classifies one unquoted token
fn item(token: &str) -> Item
{
    let target = |label: &str| (label.is_empty() == false).then(|| label.to_string());
    if let Some(label) = token.strip_prefix(":>")
    {
        Item::Label(label.to_string())
    }
    else if let Some(label) = token.strip_prefix("=>")
    {
        Item::Jump(Jump::Always, target(label))
    }
    else if let Some(label) = token.strip_prefix("+>")
    {
        Item::Jump(Jump::IfTrue, target(label))
    }
    else if let Some(label) = token.strip_prefix("->")
    {
        Item::Jump(Jump::IfFalse, target(label))
    }
    else
    {
        Item::Call(token.to_string())
    }
}

/// All defined macros
#[derive(Debug, Clone, Default)]
pub struct Macros
{
    defined: HashMap<String, Vec<Item>>
}

impl Macros
{
    /// Defines or replaces macro `name`
    ///
    /// # Errors
    ///
    /// Returns an error if the body does not parse or [`MAX_MACROS`] macros already exist.
    pub fn define(&mut self, name: &str, body: &str) -> Result<()>
    {
        let items = parse(body)?;
        let name = name.to_lowercase();
        require!(self.defined.len() < MAX_MACROS || self.defined.contains_key(&name), Err(anyhow!("too many macros (limit {MAX_MACROS})")));
        self.defined.insert(name, items);
        Ok(())
    }

    /// Returns the items of macro `name`
    pub fn get(&self, name: &str) -> Option<&[Item]>
    {
        self.defined.get(&name.to_lowercase()).map(Vec::as_slice)
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    fn call(name: &str) -> Item
    {
        Item::Call(name.into())
    }

    #[test]
    fn test_parse_functions_and_text()
    {
        assert_eq!(parse("Meta Up Arg \"11\" Plines Begline").unwrap(), vec![
            call("meta"),
            call("up"),
            call("arg"),
            Item::Text("11".into()),
            call("plines"),
            call("begline")
        ]);
    }

    #[test]
    fn test_parse_text_with_spaces_and_text_only_macro()
    {
        assert_eq!(parse("\"procedure();\"").unwrap(), vec![Item::Text("procedure();".into())]);
        assert_eq!(parse("arg  \"PRINT TEMP.DAT\"shell").unwrap(), vec![call("arg"), Item::Text("PRINT TEMP.DAT".into()), call("shell")]);
        assert_eq!(parse("\"\"").unwrap(), vec![Item::Text(String::new())]);
    }

    #[test]
    fn test_parse_labels_and_jumps()
    {
        assert_eq!(parse("Endline :>back Sdelete Left +>Back").unwrap(), vec![
            call("endline"),
            Item::Label("back".into()),
            call("sdelete"),
            call("left"),
            Item::Jump(Jump::IfTrue, Some("back".into()))
        ]);
        assert_eq!(parse("=> -> +> =>x ->y").unwrap(), vec![
            Item::Jump(Jump::Always, None),
            Item::Jump(Jump::IfFalse, None),
            Item::Jump(Jump::IfTrue, None),
            Item::Jump(Jump::Always, Some("x".into())),
            Item::Jump(Jump::IfFalse, Some("y".into()))
        ]);
    }

    #[test]
    fn test_parse_empty_body()
    {
        assert_eq!(parse("   ").unwrap(), Vec::new());
    }

    #[test]
    fn test_parse_rejects_unterminated_string()
    {
        assert_eq!(parse("arg \"oops").unwrap_err().to_string(), "unterminated string in macro");
    }

    #[test]
    fn test_define_and_get()
    {
        let mut macros = Macros::default();
        macros.define("Save", "Arg Arg Setfile").unwrap();
        assert_eq!(macros.get("save").unwrap(), [call("arg"), call("arg"), call("setfile")]);
        macros.define("save", "exit").unwrap();
        assert_eq!(macros.get("SAVE").unwrap(), [call("exit")]);
        assert!(macros.get("other").is_none());
        assert!(macros.define("bad", "\"x").is_err());
    }

    #[test]
    fn test_define_limit()
    {
        let mut macros = Macros::default();
        for index in 0..MAX_MACROS
        {
            macros.define(&format!("m{index}"), "up").unwrap();
        }
        macros.define("m0", "down").unwrap();
        assert_eq!(macros.define("one-too-many", "up").unwrap_err().to_string(), "too many macros (limit 1024)");
    }
}
