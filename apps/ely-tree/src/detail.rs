//! One commit: who, when, its message, signature and files (TREE-06).

use std::{io, path::Path};

use crate::{
    diff::Space,
    git::Git,
    shown::{self, Found},
    status,
};

/// A name, an address and a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub name: String,
    pub email: String,
    pub at: i64,
}

/// A file the commit changed, against its first parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Touched {
    pub path: String,
    pub from: Option<String>,
    /// Git's letter: `A`, `M`, `D`, `R`, `C`, `T`.
    pub status: char,
    pub lines: Option<(usize, usize)>,
    /// The commit holding it, when not the detail's own.
    pub commit: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detail {
    pub id: String,
    pub parents: Vec<String>,
    pub author: Person,
    pub committer: Person,
    pub refs: String,
    /// `%G?`: `G` good, `N` none, and Git's other letters.
    pub signed: char,
    pub signer: String,
    pub message: String,
    pub files: Vec<Touched>,
}

fn bad(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("an unread commit {what:?}"),
    )
}

const FORMAT: &str =
    "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%cn%x00%ce%x00%ct%x00%D%x00%G?%x00%GS%x00%B";

/// What a commit is compared with: first parent, or nothing.
pub fn base(git: &Git, root: &Path, parents: &[String]) -> io::Result<String> {
    match parents.first() {
        Some(parent) => Ok(parent.clone()),
        None => {
            let empty = git.run_input(root, &["hash-object", "-t", "tree", "--stdin"], b"")?;
            Ok(String::from_utf8_lossy(&empty).trim().to_string())
        }
    }
}

/// `--name-status -z`: letters, then one path or two.
fn statuses(bytes: &[u8]) -> io::Result<Vec<(char, Option<String>, String)>> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("names not UTF-8"))?;
    let mut parts = text.split('\0').filter(|part| !part.is_empty());
    let mut out = Vec::new();
    while let Some(letters) = parts.next() {
        let letter = letters.chars().next().ok_or_else(|| bad(letters))?;
        let first = parts.next().ok_or_else(|| bad(letters))?.to_string();
        match letter {
            'R' | 'C' => out.push((
                letter,
                Some(first),
                parts.next().ok_or_else(|| bad(letters))?.to_string(),
            )),
            _ => out.push((letter, None, first)),
        }
    }
    Ok(out)
}

pub fn read(git: &Git, root: &Path, id: &str) -> io::Result<Detail> {
    let out = git.run(root, &["show", "-s", FORMAT, id, "--"])?;
    let text = String::from_utf8(out).map_err(|_| bad("not UTF-8"))?;
    let fields: Vec<&str> = text.splitn(12, '\0').collect();
    let [
        id,
        parents,
        an,
        ae,
        at,
        cn,
        ce,
        ct,
        refs,
        signed,
        signer,
        message,
    ] = fields[..]
    else {
        return Err(bad(&text));
    };
    let time = |at: &str| at.parse::<i64>().map_err(|_| bad(at));
    let mut detail = Detail {
        id: id.to_string(),
        parents: parents
            .split(' ')
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect(),
        author: Person {
            name: an.into(),
            email: ae.into(),
            at: time(at)?,
        },
        committer: Person {
            name: cn.into(),
            email: ce.into(),
            at: time(ct)?,
        },
        refs: refs.to_string(),
        signed: signed.chars().next().unwrap_or('N'),
        signer: signer.to_string(),
        message: message.trim_end().to_string(),
        files: Vec::new(),
    };
    let base = base(git, root, &detail.parents)?;
    let names = git.run(
        root,
        &["diff", "--name-status", "-z", "-M", &base, id, "--"],
    )?;
    let lines =
        status::lines(&git.run(root, &["diff", "--numstat", "-z", "-M", &base, id, "--"])?)?;
    detail.files = statuses(&names)?
        .into_iter()
        .map(|(status, from, path)| Touched {
            lines: lines.get(&path).copied().flatten(),
            commit: None,
            path,
            from,
            status,
        })
        .collect();
    Ok(detail)
}

/// Whether `name`, as `rev:path`, names something.
pub fn exists(git: &Git, root: &Path, name: &str) -> io::Result<bool> {
    Ok(!git
        .run_or_one(root, &["rev-parse", "--verify", "--quiet", name])?
        .is_empty())
}

/// A file's change in `id`; status from which sides exist.
pub fn change(
    git: &Git,
    root: &Path,
    id: &str,
    parents: &[String],
    path: &str,
    from: Option<String>,
) -> io::Result<Found> {
    let base = base(git, root, parents)?;
    let was = from.as_deref().unwrap_or(path);
    let status = match (
        exists(git, root, &format!("{base}:{was}"))?,
        exists(git, root, &format!("{id}:{path}"))?,
    ) {
        (false, _) => 'A',
        (_, false) => 'D',
        _ if from.is_some() => 'R',
        _ => 'M',
    };
    file(
        git,
        root,
        id,
        parents,
        &Touched {
            path: path.to_string(),
            from,
            status,
            lines: None,
            commit: None,
        },
    )
}

/// One file's change in a commit, for the middle.
pub fn file(
    git: &Git,
    root: &Path,
    id: &str,
    parents: &[String],
    touched: &Touched,
) -> io::Result<Found> {
    let base = base(git, root, parents)?;
    let mut args = shown::base(Space::Exact);
    args.extend([
        "-M".into(),
        base.clone(),
        id.to_string(),
        "--".into(),
        touched.path.clone(),
    ]);
    args.extend(touched.from.clone());
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let seen = git.run(root, &args)?;
    let was = touched.from.as_deref().unwrap_or(&touched.path);
    let old = (touched.status != 'A').then(|| format!("{base}:{was}"));
    let new = (touched.status != 'D').then(|| Some(format!("{id}:{}", touched.path)));
    shown::compare(git, root, seen, &touched.path, old, new)
}
