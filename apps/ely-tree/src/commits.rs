//! History as Git prints it: graph, file commits, search (TREE-06).

use std::{io, path::Path};

use crate::git::Git;

/// Commits read per page.
pub const PAGE: usize = 2000;

/// One commit as a log lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Logged {
    pub id: String,
    pub parents: Vec<String>,
    pub author: String,
    /// Author time, seconds since the epoch.
    pub at: i64,
    pub subject: String,
    pub refs: Vec<String>,
}

/// What a search looks in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Find {
    #[default]
    Message,
    Author,
    Hash,
    /// Commits adding or removing the text (`-S`).
    Content,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub find: Find,
    pub text: String,
}

const FORMAT: &str = "--format=%x1e%H%x00%P%x00%an%x00%at%x00%s%x00%D";

fn bad(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread log record {what:?}"),
    )
}

fn text(bytes: &[u8]) -> io::Result<&str> {
    std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a log that is not UTF-8"))
}

/// `HEAD -> main, origin/main, tag: v1` as `main`, `origin/main`, `v1`.
fn refs(decorations: &str) -> Vec<String> {
    decorations
        .split(", ")
        .filter(|name| !name.is_empty() && *name != "HEAD")
        .map(|name| {
            name.trim_start_matches("HEAD -> ")
                .trim_start_matches("tag: ")
                .to_string()
        })
        .collect()
}

fn record(fields: &[&str], whole: &str) -> io::Result<Logged> {
    let [id, parents, author, at, subject, ..] = fields else {
        return Err(bad(whole));
    };
    Ok(Logged {
        id: id.to_string(),
        parents: parents
            .split(' ')
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect(),
        author: author.to_string(),
        at: at.parse().map_err(|_| bad(whole))?,
        subject: subject.to_string(),
        refs: fields
            .get(5)
            .map(|d| refs(d.trim_end()))
            .unwrap_or_default(),
    })
}

/// Reads `log` output in `FORMAT`.
pub fn parse(bytes: &[u8]) -> io::Result<Vec<Logged>> {
    text(bytes)?
        .split('\u{1e}')
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| {
            record(
                &chunk.trim_end_matches('\n').split('\0').collect::<Vec<_>>(),
                chunk,
            )
        })
        .collect()
}

/// A page of history: every ref, or a search.
pub fn read(git: &Git, root: &Path, skip: usize, query: Option<&Query>) -> io::Result<Vec<Logged>> {
    let (skip, page) = (format!("--skip={skip}"), format!("--max-count={PAGE}"));
    let mut args: Vec<String> = ["log", "--topo-order", FORMAT, &page, &skip]
        .map(String::from)
        .to_vec();
    match query {
        Some(Query {
            find: Find::Hash,
            text,
        }) => {
            let found = git.run_or_one(
                root,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("{text}^{{commit}}"),
                ],
            )?;
            let id = String::from_utf8_lossy(&found).trim().to_string();
            if id.is_empty() || skip != "--skip=0" {
                return Ok(Vec::new());
            }
            args = ["log", FORMAT, "--max-count=1", &id]
                .map(String::from)
                .to_vec();
        }
        Some(Query { find, text }) => {
            args.extend(
                [
                    "--exclude=refs/stash",
                    "--all",
                    "--regexp-ignore-case",
                    "--fixed-strings",
                ]
                .map(String::from),
            );
            args.push(match find {
                Find::Author => format!("--author={text}"),
                Find::Content => format!("-S{text}"),
                _ => format!("--grep={text}"),
            });
        }
        None => args.extend(["--exclude=refs/stash", "--all"].map(String::from)),
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    parse(&git.run(root, &args)?)
}

/// The refs as they stand, to see whether history moved.
pub fn refs_now(git: &Git, root: &Path) -> io::Result<Vec<u8>> {
    git.run_or_one(root, &["show-ref", "--head"])
}

/// A file's commit, the path it had, and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Touch {
    pub commit: Logged,
    pub path: String,
    pub from: Option<String>,
    /// None for binary files or commits without a diff.
    pub lines: Option<(usize, usize)>,
}

/// The file's commits newest first, through renames.
pub fn file(git: &Git, root: &Path, path: &str) -> io::Result<Vec<Touch>> {
    let format = "--format=%x1e%H%x00%P%x00%an%x00%at%x00%s";
    let out = git.run(
        root,
        &["log", "--follow", format, "--numstat", "-z", "--", path],
    )?;
    let mut now = path.to_string();
    let mut touches = Vec::new();
    for chunk in text(&out)?
        .split('\u{1e}')
        .filter(|chunk| !chunk.is_empty())
    {
        let fields: Vec<&str> = chunk.splitn(6, '\0').collect();
        let commit = record(&fields[..fields.len().min(5)], chunk)?;
        let stat = fields
            .get(5)
            .copied()
            .unwrap_or("")
            .trim_start_matches('\n');
        let mut parts = stat.split('\0').filter(|part| !part.is_empty());
        let (lines, path, from) = match parts.next() {
            None => (None, now.clone(), None),
            Some(head) => {
                let mut cells = head.splitn(3, '\t');
                let (added, removed, named) =
                    (cells.next(), cells.next(), cells.next().unwrap_or(""));
                let lines = added
                    .zip(removed)
                    .and_then(|(a, r)| Some((a.parse().ok()?, r.parse().ok()?)));
                match named {
                    "" => {
                        let from = parts.next().ok_or_else(|| bad(chunk))?.to_string();
                        (
                            lines,
                            parts.next().ok_or_else(|| bad(chunk))?.to_string(),
                            Some(from),
                        )
                    }
                    named => (lines, named.to_string(), None),
                }
            }
        };
        now = from.clone().unwrap_or_else(|| path.clone());
        touches.push(Touch {
            commit,
            path,
            from,
            lines,
        });
    }
    Ok(touches)
}
