//! Committing: message, amend, identity, signing as set (TREE-05).

use std::{io, path::Path};

use crate::git::Git;

/// Who commits, and how Git signs, as Git's settings say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Me {
    pub name: Option<String>,
    pub email: Option<String>,
    /// `openpgp`, `ssh` or `x509` when commits are signed.
    pub signs: Option<String>,
}

/// A setting as Git resolves it; None when unset.
fn setting(git: &Git, root: &Path, key: &str) -> io::Result<Option<String>> {
    let said = git.run_or_one(root, &["config", "--get", key])?;
    let said = String::from_utf8_lossy(&said).trim().to_string();
    Ok((!said.is_empty()).then_some(said))
}

pub fn me(git: &Git, root: &Path) -> io::Result<Me> {
    let signs = git.run_or_one(root, &["config", "--type=bool", "--get", "commit.gpgsign"])?;
    let signs = match String::from_utf8_lossy(&signs).trim() {
        "true" => Some(setting(git, root, "gpg.format")?.unwrap_or_else(|| "openpgp".into())),
        _ => None,
    };
    Ok(Me {
        name: setting(git, root, "user.name")?,
        email: setting(git, root, "user.email")?,
        signs,
    })
}

/// The last commit's whole message, to amend.
pub fn last_message(git: &Git, root: &Path) -> io::Result<String> {
    let said = git.run(root, &["log", "-1", "--format=%B"])?;
    Ok(String::from_utf8_lossy(&said).trim_end().to_string())
}

/// Subject and body as one message: a blank line between.
pub fn message(subject: &str, body: &str) -> String {
    match body.trim() {
        "" => subject.trim().to_string(),
        body => format!("{}\n\n{body}", subject.trim()),
    }
}

/// A message back to subject and body.
pub fn split(message: &str) -> (String, String) {
    let (subject, body) = message.split_once('\n').unwrap_or((message, ""));
    (subject.trim().to_string(), body.trim().to_string())
}
