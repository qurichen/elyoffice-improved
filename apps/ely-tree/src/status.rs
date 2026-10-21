//! The work tree from porcelain v2 status (TREE-02).

use std::{collections::HashMap, io, path::Path};

use crate::git::Git;

/// The arguments; untracked and ignored folders stay one entry.
pub const ARGS: [&str; 6] = [
    "status",
    "--porcelain=v2",
    "-z",
    "--branch",
    "--untracked-files=normal",
    "--ignored",
];

/// Where HEAD stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Head {
    /// None before the first commit.
    pub oid: Option<String>,
    /// None when detached.
    pub branch: Option<String>,
    pub upstream: Option<String>,
    /// Ahead and behind; None when the upstream is gone.
    pub counts: Option<(u32, u32)>,
}

/// What a path is to Git.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Index and work tree letters, `.` for none.
    Tracked {
        staged: char,
        unstaged: char,
    },
    /// Both sides' letters, as `UU`, `AA`, `DU`.
    Conflicted(String),
    Untracked,
    Ignored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    /// The old path of a rename or copy.
    pub from: Option<String>,
    pub state: State,
}

/// Added and removed lines by path; None if binary.
pub type Lines = HashMap<String, Option<(usize, usize)>>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub head: Head,
    pub entries: Vec<Entry>,
    /// Index against HEAD.
    pub staged: Lines,
    /// Work tree against the index.
    pub unstaged: Lines,
}

fn bad(record: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread status record {record:?}"),
    )
}

/// The path after `skip` space-separated fields.
fn after(record: &str, skip: usize) -> io::Result<String> {
    record
        .splitn(skip + 1, ' ')
        .nth(skip)
        .map(str::to_string)
        .ok_or_else(|| bad(record))
}

fn letters(record: &str) -> io::Result<(char, char)> {
    let mut xy = record.get(2..4).ok_or_else(|| bad(record))?.chars();
    Ok((
        xy.next().ok_or_else(|| bad(record))?,
        xy.next().ok_or_else(|| bad(record))?,
    ))
}

/// A changed file's letters, each one Git documents.
fn tracked(record: &str) -> io::Result<State> {
    let (staged, unstaged) = letters(record)?;
    match [staged, unstaged]
        .iter()
        .all(|letter| ".MTADRC".contains(*letter))
    {
        true => Ok(State::Tracked { staged, unstaged }),
        false => Err(bad(record)),
    }
}

fn header(head: &mut Head, line: &str) -> io::Result<()> {
    let (key, value) = line.split_once(' ').ok_or_else(|| bad(line))?;
    match key {
        "branch.oid" => head.oid = (value != "(initial)").then(|| value.to_string()),
        "branch.head" => head.branch = (value != "(detached)").then(|| value.to_string()),
        "branch.upstream" => head.upstream = Some(value.to_string()),
        "branch.ab" => {
            let (ahead, behind) = value.split_once(' ').ok_or_else(|| bad(line))?;
            let count = |n: &str, sign| {
                n.strip_prefix(sign)
                    .and_then(|n| n.parse().ok())
                    .ok_or_else(|| bad(line))
            };
            head.counts = Some((count(ahead, '+')?, count(behind, '-')?));
        }
        // Stash counts and later headers are not read here.
        _ => {}
    }
    Ok(())
}

/// Reads porcelain v2 with `-z`: records end in NUL.
pub fn parse(bytes: &[u8]) -> io::Result<Status> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "status that is not UTF-8"))?;
    let mut records = text.split('\0').filter(|record| !record.is_empty());
    let mut status = Status::default();
    while let Some(record) = records.next() {
        let entry = match record.as_bytes()[0] {
            b'#' => {
                header(&mut status.head, &after(record, 1)?)?;
                continue;
            }
            b'1' => Entry {
                path: after(record, 8)?,
                from: None,
                state: tracked(record)?,
            },
            b'2' => {
                let state = tracked(record)?;
                let from = records.next().ok_or_else(|| bad(record))?.to_string();
                Entry {
                    path: after(record, 9)?,
                    from: Some(from),
                    state,
                }
            }
            b'u' => {
                let (ours, theirs) = letters(record)?;
                Entry {
                    path: after(record, 10)?,
                    from: None,
                    state: State::Conflicted(format!("{ours}{theirs}")),
                }
            }
            b'?' => Entry {
                path: after(record, 1)?,
                from: None,
                state: State::Untracked,
            },
            b'!' => Entry {
                path: after(record, 1)?,
                from: None,
                state: State::Ignored,
            },
            _ => return Err(bad(record)),
        };
        status.entries.push(entry);
    }
    Ok(status)
}

/// Reads `diff --numstat -z`; renames keyed by new path.
pub fn lines(bytes: &[u8]) -> io::Result<Lines> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "numstat that is not UTF-8"))?;
    let mut records = text.split('\0');
    let mut lines = Lines::new();
    while let Some(record) = records.next().filter(|record| !record.is_empty()) {
        let mut fields = record.splitn(3, '\t');
        let (added, removed, path) = (
            fields.next(),
            fields.next(),
            fields.next().ok_or_else(|| bad(record))?,
        );
        let path = match path {
            "" => records.nth(1).ok_or_else(|| bad(record))?,
            path => path,
        };
        let count = match (added, removed) {
            (Some("-"), Some("-")) => None,
            (Some(added), Some(removed)) => Some((
                added.parse().map_err(|_| bad(record))?,
                removed.parse().map_err(|_| bad(record))?,
            )),
            _ => return Err(bad(record)),
        };
        lines.insert(path.to_string(), count);
    }
    Ok(lines)
}

/// The work tree at `dir`, read without taking Git's locks.
pub fn read(git: &Git, dir: &Path) -> io::Result<Status> {
    let mut status = parse(&git.run(dir, &ARGS)?)?;
    let tracked = status
        .entries
        .iter()
        .any(|entry| matches!(entry.state, State::Tracked { .. }));
    if tracked {
        status.staged = lines(&git.run(dir, &["diff", "--cached", "--numstat", "-z"])?)?;
        status.unstaged = lines(&git.run(dir, &["diff", "--numstat", "-z"])?)?;
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_record_kind_reads() {
        let bytes = b"# branch.oid (initial)\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaa bbb a b.txt\0\
2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\0old name.txt\0\
u UU N... 100644 100644 100644 100644 a b c clash.txt\0? new.txt\0! target/\0";
        let status = parse(bytes).expect("reads");
        assert_eq!(
            status.head,
            Head {
                oid: None,
                branch: Some("main".into()),
                upstream: Some("origin/main".into()),
                counts: Some((2, 1)),
            }
        );
        let states: Vec<(&str, &State)> = status
            .entries
            .iter()
            .map(|e| (e.path.as_str(), &e.state))
            .collect();
        assert_eq!(
            states,
            [
                (
                    "a b.txt",
                    &State::Tracked {
                        staged: '.',
                        unstaged: 'M'
                    }
                ),
                (
                    "new name.txt",
                    &State::Tracked {
                        staged: 'R',
                        unstaged: '.'
                    }
                ),
                ("clash.txt", &State::Conflicted("UU".into())),
                ("new.txt", &State::Untracked),
                ("target/", &State::Ignored),
            ]
        );
        assert_eq!(status.entries[1].from.as_deref(), Some("old name.txt"));
    }

    #[test]
    fn numstat_reads_counts_binaries_and_renames() {
        let read = lines(b"3\t1\ta b.txt\0-\t-\tlogo.png\0" as &[u8]).expect("reads");
        assert_eq!(read["a b.txt"], Some((3, 1)));
        assert_eq!(read["logo.png"], None, "binary");
        let read = lines(b"0\t0\t\0old.txt\0new.txt\0").expect("reads");
        assert_eq!(read.keys().collect::<Vec<_>>(), ["new.txt"]);
        assert!(lines(b"x\t1\tf\0").is_err() && lines(b"1\0").is_err());
    }

    #[test]
    fn a_strange_record_is_an_error() {
        let gone = parse(b"# branch.head main\0# branch.upstream origin/main\0").expect("reads");
        assert_eq!(gone.head.counts, None, "no counts is not zero");
        assert!(parse(b"7 what\0").is_err());
        assert!(
            parse(b"1 .X N... 100644 100644 100644 aaa bbb f\0").is_err(),
            "an unknown letter"
        );
        assert!(parse(b"1 .M\0").is_err(), "cut short");
        assert!(
            parse(b"u\0").is_err() && parse(b"?\0").is_err(),
            "no panic on a stub"
        );
    }
}
