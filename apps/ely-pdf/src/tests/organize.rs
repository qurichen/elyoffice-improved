//! Organizing, undo, saving and output from the app (C11.2).

use ely_office::{Closing as _, Dialog, Saving as _, Tabs as _};
use ely_pdf_engine::Worker;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{file, frames, settle, shown, windowed};
use crate::view::PdfView;
use crate::viewer::Viewer;

/// Three Letter pages: one, two, three.
fn three<'a>(cx: &'a mut TestAppContext, dir: &std::path::Path) -> (Entity<Viewer>, &'a mut VisualTestContext, Entity<PdfView>) {
    let pages: &[(f32, f32, &[&str])] = &[(612.0, 792.0, &["one"]), (612.0, 792.0, &["two"]), (612.0, 792.0, &["three"])];
    let (viewer, cx) = windowed(cx, dir, &[file(dir, "three.pdf", pages)], true);
    let view = shown(cx, &viewer);
    (viewer, cx, view)
}

/// Each page's text, read by a worker of its own.
fn texts(path: &std::path::Path) -> Vec<String> {
    let mut worker = Worker::spawn().expect("a worker");
    let doc = worker.open(path).expect("opens").doc;
    let count = worker.pages(doc).expect("counts");
    (0..count).map(|page| worker.text(doc, page).expect("reads").iter().map(|c| c.c).collect()).collect()
}

fn dirty(cx: &mut VisualTestContext, viewer: &Entity<Viewer>) -> bool {
    cx.update(|_, app| viewer.read(app).active_tab().expect("a tab").dirty(app))
}

/// Runs `step` on the view, then waits for its edits.
fn edit(cx: &mut VisualTestContext, view: &Entity<PdfView>, step: impl FnOnce(&mut PdfView, &mut gpui::Context<PdfView>)) {
    cx.update(|_, app| view.update(app, step));
    settle(cx, |app| view.read(app).settled());
}

#[test]
fn a_delete_marks_the_tab_and_undo_clears_it() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    assert!(!dirty(cx, &viewer));
    edit(cx, &view, |view, cx| {
        view.picked = [1].into();
        view.edit("deleting", |worker, doc| worker.delete_pages(doc, &[1]), cx);
    });
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).sizes.len()), 2);
    assert!(dirty(cx, &viewer), "an edit is unsaved");
    edit(cx, &view, |view, cx| view.undo(false, cx));
    frames(cx);
    assert_eq!(cx.update(|_, app| view.read(app).sizes.len()), 3);
    assert!(!dirty(cx, &viewer), "undone to the file as loaded");
}

#[test]
fn a_save_writes_the_edits_and_undo_after_it_is_unsaved_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    edit(cx, &view, |view, cx| {
        view.picked = [2].into();
        let to = 0;
        view.move_to(to, cx);
    });
    assert_eq!(cx.update(|_, app| view.read(app).picked.iter().copied().collect::<Vec<_>>()), [0], "the moved page stays picked");
    let id = cx.update(|_, app| viewer.read(app).core.active.expect("a tab"));
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.save(id, None, window, cx)));
    settle(cx, |app| !viewer.read(app).active_tab().expect("a tab").dirty(app));
    assert_eq!(texts(&dir.path().join("three.pdf")), ["three", "one", "two"]);
    edit(cx, &view, |view, cx| view.undo(false, cx));
    frames(cx);
    assert!(dirty(cx, &viewer), "the file still holds the move");
    edit(cx, &view, |view, cx| view.undo(true, cx));
    frames(cx);
    assert!(!dirty(cx, &viewer), "redone to what was saved");
}

#[test]
fn an_edit_queued_behind_another_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = three(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| {
        view.edit("turning", |worker, doc| worker.rotate_pages(doc, &[0], 90), cx);
        view.edit("deleting", |worker, doc| worker.delete_pages(doc, &[0]), cx);
    }));
    // The worker sees one edit at a time.
    assert_eq!(cx.update(|_, app| (view.read(app).editing, view.read(app).queue.len())), (true, 1));
    settle(cx, |app| view.read(app).settled());
    // The delete aimed at the state before the turn.
    let sizes = cx.update(|_, app| view.read(app).sizes.clone());
    assert_eq!(sizes, [(792.0, 612.0), (612.0, 792.0), (612.0, 792.0)], "only the turn landed");
}

#[test]
fn a_read_from_before_an_edit_is_dropped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = three(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| {
        view.page_text(0, cx);
        // As an edit landing first does.
        view.version += 1;
    }));
    frames(cx);
    assert!(cx.update(|_, app| view.read(app).text.is_empty()), "page one's old text is not kept");
}

#[test]
fn closing_an_edited_tab_asks_first() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    edit(cx, &view, |view, cx| view.edit("turning", |worker, doc| worker.rotate_pages(doc, &[0], 90), cx));
    let id = cx.update(|_, app| viewer.read(app).core.active.expect("a tab"));
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.request_close(id, ely_office::After::Tab, window, cx)));
    settle(cx, |app| matches!(viewer.read(app).core.dialog, Some(Dialog::Unsaved { .. })));
}

#[test]
fn extract_and_split_write_new_files_and_refuse_to_overwrite() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    let id = cx.update(|_, app| viewer.read(app).core.active.expect("a tab"));
    let part = dir.path().join("part.pdf");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_pages(id, vec![0, 2], part.clone(), cx)));
    settle(cx, |_| part.exists());
    frames(cx);
    assert_eq!(texts(&part), ["one", "three"]);
    // The open file itself is refused.
    let open = dir.path().join("three.pdf");
    let before = std::fs::read(&open).expect("reads");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_pages(id, vec![0], open.clone(), cx)));
    frames(cx);
    assert_eq!(std::fs::read(&open).expect("reads"), before);
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("a folder");
    std::fs::write(out.join("three-2.pdf"), b"keep").expect("writes");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_parts(id, 1, out.clone(), cx)));
    frames(cx);
    let mut names: Vec<String> = std::fs::read_dir(&out).expect("lists").map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, ["three-2.pdf"], "a taken name writes nothing");
    std::fs::remove_file(out.join("three-2.pdf")).expect("removes");
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.write_parts(id, 2, out.clone(), cx)));
    settle(cx, |_| out.join("three-2.pdf").exists());
    frames(cx);
    assert_eq!((texts(&out.join("three-1.pdf")), texts(&out.join("three-2.pdf"))), (vec!["one".into(), "two".into()], vec!["three".into()]));
    assert_eq!(cx.update(|_, app| view.read(app).sizes.len()), 3, "the document stays");
}

#[test]
fn crop_margins_are_millimetres() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_crop(window, cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        if let Some(crate::ask::Ask::Crop { margins, .. }) = viewer.ask.as_mut() {
            *margins = [10.0, 0.0, 0.0, 25.4];
        }
        viewer.crop(cx);
    }));
    settle(cx, |app| view.read(app).edited == 1);
    let size = cx.update(|_, app| view.read(app).sizes[0]);
    let mm = 72.0 / 25.4;
    assert!((size.0 - (612.0 - 72.0)).abs() < 0.1 && (size.1 - (792.0 - 10.0 * mm)).abs() < 0.1, "{size:?}");
}

#[test]
fn picks_follow_plain_range_and_toggle_clicks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (_, cx, view) = three(&mut cx, dir.path());
    let picked = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).picked.iter().copied().collect::<Vec<_>>());
    cx.update(|_, app| view.update(app, |view, cx| view.pick(0, (false, false), cx)));
    cx.update(|_, app| view.update(app, |view, cx| view.pick(2, (true, false), cx)));
    assert_eq!(picked(cx), [0, 1, 2]);
    cx.update(|_, app| view.update(app, |view, cx| view.pick(1, (false, true), cx)));
    assert_eq!(picked(cx), [0, 2]);
    assert_eq!(cx.update(|_, app| view.read(app).chosen_pages()), [0, 2]);
}

#[test]
fn a_range_pick_starts_over_after_pages_move() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, cx| view.pick(2, (false, false), cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| viewer.delete(cx)));
    settle(cx, |app| view.read(app).sizes.len() == 2);
    cx.update(|_, app| view.update(app, |view, cx| view.pick(0, (true, false), cx)));
    assert_eq!(cx.update(|_, app| view.read(app).chosen_pages()), [0], "no range from a page that went");
    edit(cx, &view, |view, cx| view.edit("blank", |worker, doc| worker.insert_blank(doc, 1, (100.0, 100.0)), cx));
    assert_eq!(cx.update(|_, app| view.read(app).sizes.len()), 3);
}

#[test]
fn a_failed_edit_lets_a_waiting_save_go() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    let id = cx.update(|_, app| viewer.read(app).core.active.expect("a tab"));
    let ran = std::rc::Rc::new(std::cell::Cell::new(false));
    cx.update(|_, app| view.update(app, |view, cx| view.edit("cropping", |worker, doc| worker.crop_pages(doc, &[0], [900.0; 4]), cx)));
    cx.update(|_, app| viewer.update(app, |viewer, _| {
        let ran = ran.clone();
        viewer.when_settled(id, move |_, _, _| ran.set(true));
    }));
    settle(cx, |_| ran.get());
}

#[test]
fn a_turned_view_crops_what_is_seen() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    cx.update(|_, app| view.update(app, |view, _| view.rotate = 90));
    cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.ask_crop(window, cx)));
    cx.update(|_, app| viewer.update(app, |viewer, cx| {
        if let Some(crate::ask::Ask::Crop { margins, .. }) = viewer.ask.as_mut() {
            *margins = [25.4, 0.0, 0.0, 0.0];
        }
        viewer.crop(cx);
    }));
    settle(cx, |app| view.read(app).edited == 1);
    let (w, h) = cx.update(|_, app| view.read(app).sizes[0]);
    assert!((w - 540.0).abs() < 0.1 && (h - 792.0).abs() < 0.1, "the page's left went: {w} x {h}");
}

#[test]
fn the_window_closes_its_tabs_off_the_window() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let (viewer, cx, view) = three(&mut cx, dir.path());
    assert!(!cx.update(|window, app| viewer.update(app, |viewer, cx| viewer.should_close(window, cx))), "not at once");
    drop(view);
    // Once its tab closes, the window itself goes.
    for _ in 0..1000 {
        cx.executor().tick();
        if cx.cx.update(|app| app.windows().is_empty()) {
            return;
        }
    }
    panic!("the window stayed");
}
