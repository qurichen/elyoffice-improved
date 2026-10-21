//! Undo and redo lists as the kernel reports them.

use std::io;

/// One entry: the kernel's comment and time.
pub type Step = (String, String);

/// Entries of an `.uno:Undo` or `.uno:Redo` reply, newest first.
pub fn steps(json: &str) -> io::Result<Vec<Step>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| io::Error::other(format!("a bad undo list: {error}")))?;
    // An empty history is `"actions": ""`.
    let Some(actions) = value["actions"].as_array() else {
        return Ok(Vec::new());
    };
    Ok(actions
        .iter()
        .map(|action| {
            let field = |name: &str| action[name].as_str().unwrap_or_default().to_string();
            (field("comment"), field("dateTime"))
        })
        .collect())
}
