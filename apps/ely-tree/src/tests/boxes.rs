//! Commit box, history refresh and settings, windowed (C13.3).

use gpui::{App, Entity, TestAppContext, VisualTestContext};

use super::{
    committed, sh,
    window::{branch, settle, window},
};
use crate::tree::Tree;

pub(crate) fn subject(tree: &Entity<Tree>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, app| tree.read(app).subject.read(app).text().to_string())
}

pub(crate) fn type_subject(tree: &Entity<Tree>, cx: &mut VisualTestContext, words: &str) {
    let words = words.to_string();
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.subject
                .update(cx, |field, cx| field.set_text(words, cx))
        })
    });
}

/// Two repositories, each with its own identity set.
pub(crate) fn two(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let (a, b) = (committed(&dir.join("a")), committed(&dir.join("b")));
    for root in [&a, &b] {
        for (key, value) in [
            ("user.name", "Ely"),
            ("user.email", "ely@example.com"),
            ("commit.gpgsign", "false"),
        ] {
            sh(root, &["config", key, value]);
        }
    }
    (a, b)
}

#[test]
fn each_repository_keeps_its_own_commit_box() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, b) = two(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    for (path, id) in [(a.clone(), 1), (b.clone(), 2)] {
        cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(path, cx)));
        settle(cx, |app| tree.read(app).active == Some(id));
    }
    cx.update(|_, app| tree.update(app, |tree, cx| tree.activate(Some(1), cx)));
    cx.update(|_, app| tree.update(app, |tree, cx| tree.set_amend(true, cx)));
    settle(cx, |app| {
        tree.read(app).amend && !tree.read(app).amend_loading
    });
    assert_eq!(subject(&tree, cx), "first", "a's last message");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.activate(Some(2), cx)));
    assert!(
        !cx.update(|_, app| tree.read(app).amend),
        "b does not amend"
    );
    assert_eq!(subject(&tree, cx), "");
    type_subject(&tree, cx, "b's draft");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.activate(Some(1), cx)));
    assert!(cx.update(|_, app| tree.read(app).amend));
    assert_eq!(subject(&tree, cx), "first");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.activate(Some(2), cx)));
    assert_eq!(subject(&tree, cx), "b's draft");
}

#[test]
fn unticking_amend_voids_a_read_on_its_way() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, _) = two(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(a.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    type_subject(&tree, cx, "mine");
    // Two clicks on the box, as the control sends them.
    for _ in 0..2 {
        cx.update(|_, app| {
            tree.update(app, |tree, cx| {
                let on = !tree.amend;
                tree.set_amend(on, cx)
            })
        });
    }
    for _ in 0..20 {
        settle(cx, |_| true);
    }
    assert!(!cx.update(|_, app| tree.read(app).amend));
    assert_eq!(subject(&tree, cx), "mine");
}

#[test]
fn typing_during_a_commit_is_kept() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, _) = two(dir.path());
    std::fs::write(a.join("a.txt"), "changed\n").expect("writes");
    sh(&a, &["add", "a.txt"]);
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(a.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    type_subject(&tree, cx, "submitted");
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.commit(cx);
            tree.subject
                .update(cx, |field, cx| field.set_text("next draft".to_string(), cx));
        })
    });
    settle(cx, |_| {
        sh(&a, &["log", "-1", "--format=%s"]).trim() == "submitted"
    });
    for _ in 0..5 {
        settle(cx, |_| true);
    }
    assert_eq!(subject(&tree, cx), "next draft");
}

#[test]
fn only_the_latest_blame_takes_its_read() {
    use crate::look::Look;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, b) = two(dir.path());
    std::fs::write(b.join("a.txt"), "other\n").expect("writes");
    sh(&b, &["commit", "-qam", "b's own"]);
    let b_head = sh(&b, &["rev-parse", "HEAD"]).trim().to_string();
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    for (path, id) in [(a.clone(), 1), (b.clone(), 2)] {
        cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(path, cx)));
        settle(cx, |app| tree.read(app).active == Some(id));
    }
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.activate(Some(1), cx);
            tree.blame("a.txt", cx);
            tree.activate(Some(2), cx);
            tree.blame("a.txt", cx);
        })
    });
    let owner = |app: &App| match &tree.read(app).look {
        Some(Look::Blame {
            read: Some(Ok(read)),
            ..
        }) => Some(read.owners[read.lines[0]].commit.clone()),
        _ => None,
    };
    settle(cx, |app| owner(app).is_some());
    for _ in 0..10 {
        settle(cx, |_| true);
    }
    assert_eq!(cx.update(|_, app| owner(app)), Some(b_head));
}

#[test]
fn history_catches_up_and_shows_ref_errors() {
    use crate::tree::Side;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, _) = two(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(a.clone(), cx)));
    settle(cx, |app| branch(tree.read(app)).is_some());
    let rows = |app: &App| {
        tree.read(app)
            .active()
            .map_or(0, |opened| opened.hist.rows.len())
    };
    // A refresh asked mid-read is held, then run.
    cx.update(|_, app| {
        tree.update(app, |tree, cx| {
            tree.set_side(Side::History, cx);
            tree.refresh_log(1, cx);
        })
    });
    std::fs::write(a.join("a.txt"), "outside\n").expect("writes");
    sh(&a, &["commit", "-qam", "outside"]);
    settle(cx, |app| rows(app) == 2);
    let error = |app: &App| {
        tree.read(app)
            .active()
            .and_then(|opened| opened.hist.error.clone())
    };
    std::fs::write(a.join(".git/refs/heads/broken"), "broken-ref\n").expect("writes");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.refresh_log(1, cx)));
    settle(cx, |app| {
        error(app).is_some_and(|error| error.contains("broken"))
    });
    std::fs::remove_file(a.join(".git/refs/heads/broken")).expect("removes");
    cx.update(|_, app| tree.update(app, |tree, cx| tree.refresh_log(1, cx)));
    settle(cx, |app| error(app).is_none() && rows(app) == 2);
}

#[test]
fn identity_follows_outside_settings_and_names_its_fields() {
    use crate::asks::Ask;
    let dir = tempfile::tempdir().expect("a temp dir");
    let (a, _) = two(dir.path());
    let mut cx = TestAppContext::single();
    let (tree, cx) = window(&mut cx, dir.path());
    cx.update(|_, app| tree.update(app, |tree, cx| tree.open_path(a.clone(), cx)));
    let name = |app: &App| match tree.read(app).active().and_then(|opened| opened.me.clone()) {
        Some(Ok(me)) => me.name,
        _ => None,
    };
    settle(cx, |app| name(app).as_deref() == Some("Ely"));
    sh(&a, &["config", "user.name", "Changed Identity"]);
    // Any outside change rereads the settings too.
    std::fs::write(a.join("poke.txt"), "x\n").expect("writes");
    settle(cx, |app| name(app).as_deref() == Some("Changed Identity"));
    cx.update(|window, app| tree.update(app, |tree, cx| tree.ask_identity(window, cx)));
    let labels = cx.update(|_, app| match &tree.read(app).ask {
        Some(Ask::Identity { name, email, .. }) => (
            name.read(app).label_text().to_string(),
            email.read(app).label_text().to_string(),
        ),
        _ => (String::new(), String::new()),
    });
    let want = cx.update(|_, app| {
        let text = |key| crate::tree::text(app, key, &[]).to_string();
        (text("tree.identity.name"), text("tree.identity.email"))
    });
    assert_eq!(labels, want);
}
