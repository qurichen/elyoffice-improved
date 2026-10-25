//! Dragging past the right edge scrolls and grows (#15).

use std::time::Duration;

use super::*;

/// A sheet with a value far to the right.
const WIDE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S">
 <table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell table:number-columns-repeated="60"/><table:table-cell><text:p>far</text:p></table:table-cell></table:table-row>
 </table:table></office:spreadsheet></office:body>
</office:document>"#;

#[test]
fn a_drag_held_past_the_edge_scrolls_and_grows_the_range() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "wide.fods", WIDE));
    let json = cx.update(|app| {
        let read = view.read(app);
        let mut worker = worker.lock().expect("the worker lock");
        worker.values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.set_geometry(geometry.clone(), cx);
            view.prepare(bounds);
        })
    });
    for _ in 0..100 {
        cx.executor().tick();
    }
    let (edge, modifiers) = (point(px(799.0), px(10.0)), gpui::Modifiers::default());
    let last = cx.update(|app| geometry.columns.at(view.read(app).doc_point(edge).0));
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.mouse(0, point(px(20.0), px(10.0)), 1, 1, &modifiers);
            view.dragging = true;
            view.drag_to(point(px(900.0), px(10.0)), modifiers, cx);
        })
    });
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(40));
        cx.executor().tick();
        cx.update(|app| view.update(app, |view, _| _ = view.prepare(bounds)));
    }
    let scrolled = cx.update(|app| view.read(app).scroll().0);
    assert!(scrolled > 0, "the view scrolled right");
    cx.update(|app| {
        view.update(app, |view, _| {
            view.mouse(1, edge, 1, 1, &modifiers);
            view.end_drag();
        })
    });
    let mut address = String::new();
    for _ in 0..200 {
        cx.executor().tick();
        address = cx.update(|app| view.read(app).cell().address.clone());
        if address.contains(':') {
            break;
        }
    }
    let end = address.split(':').nth(1).expect("a range").trim_end_matches(char::is_numeric);
    let end = (end.bytes().fold(0, |at, b| at * 26 + u32::from(b - b'A' + 1))) - 1;
    assert!(end > last, "{address} ends past column {last}");
}

#[test]
fn the_wheel_scrolls_past_the_used_columns() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = gpui::TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let (view, worker) = cx.update(|app| live_view_of(app, &install, "wide.fods", WIDE));
    let json = cx.update(|app| {
        let read = view.read(app);
        let mut worker = worker.lock().expect("the worker lock");
        worker.values(&read.doc, crate::sheet::GEOMETRY).expect("geometry")
    });
    let geometry = crate::sheet::geometry(&json).expect("parsed");
    let bounds = Bounds { origin: point(px(0.0), px(0.0)), size: size(px(800.0), px(600.0)) };
    cx.update(|app| {
        view.update(app, |view, cx| {
            view.set_geometry(geometry.clone(), cx);
            view.prepare(bounds);
        })
    });
    for _ in 0..100 {
        cx.executor().tick();
    }
    for _ in 0..200 {
        cx.update(|app| {
            view.update(app, |view, cx| {
                view.scroll_by(-400.0, 0.0, cx);
                view.prepare(bounds);
            })
        });
    }
    let column = cx.update(|app| geometry.columns.at(view.read(app).doc_point(point(px(0.0), px(10.0))).0));
    assert!(column > 200, "the left edge reached column {column}");
}
