//! Stashes: the list, and what one holds (TREE-10).

use std::{io, path::Path};

use crate::{
    detail::{self, Detail, Touched},
    git::Git,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stash {
    pub oid: String,
    /// What the user wrote, or Git's `WIP` line.
    pub message: String,
    pub branch: String,
    pub at: i64,
}

fn bad(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread stash {what:?}"),
    )
}

/// `On main: words` or `WIP on main: abc subject`.
fn subject(said: &str) -> (String, String) {
    let rest = said
        .strip_prefix("WIP on ")
        .or_else(|| said.strip_prefix("On "));
    match rest.and_then(|rest| rest.split_once(": ")) {
        Some((branch, message)) => (branch.to_string(), message.to_string()),
        None => (String::new(), said.to_string()),
    }
}

/// Newest first, as `stash@{n}` counts.
pub fn list(git: &Git, root: &Path) -> io::Result<Vec<Stash>> {
    let out = git.run(root, &["stash", "list", "-z", "--format=%H%x00%ct%x00%gs"])?;
    crate::recover::records(&out, 3)?
        .into_iter()
        .map(|fields| {
            let [oid, at, said] = fields[..] else {
                return Err(bad(&fields.join(" ")));
            };
            let (branch, message) = subject(said);
            Ok(Stash {
                oid: oid.to_string(),
                message,
                branch,
                at: at.parse().map_err(|_| bad(at))?,
            })
        })
        .collect()
}

/// Its changes, then the untracked files it took.
pub fn detail(git: &Git, root: &Path, oid: &str) -> io::Result<Detail> {
    let mut detail = detail::read(git, root, oid)?;
    if let Some(untracked) = detail.parents.get(2).cloned() {
        let held = detail::read(git, root, &untracked)?;
        detail
            .files
            .extend(held.files.into_iter().map(|file| Touched {
                commit: Some(untracked.clone()),
                ..file
            }));
    }
    Ok(detail)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_subject_gives_branch_and_message() {
        let read = |said| super::subject(said);
        assert_eq!(read("On main: a: b"), ("main".into(), "a: b".into()));
        assert_eq!(
            read("WIP on (no branch): 1a2b c"),
            ("(no branch)".into(), "1a2b c".into())
        );
        assert_eq!(read("odd"), (String::new(), "odd".into()));
    }
}
