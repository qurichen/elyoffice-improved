use ely_lok::sheet::{self, Layout};

use super::data::{cells, open, quiet, run};
use super::*;
use crate::analysis::Asked;

/// Reads through the active tab's worker.
fn read<T>(cx: &mut TestAppContext, book: &Entity<Book>, what: &str, parse: fn(&str) -> T) -> T {
    quiet(cx, book);
    cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        parse(&worker.values(&tab.doc, what).expect("the read"))
    })
}

fn ask(cx: &mut TestAppContext, book: &Entity<Book>, asked: Asked, is: fn(&Dialog) -> bool) {
    update(cx, book, |book, cx| {
        book.dialog = None;
        book.ask(asked, cx)
    });
    settle(cx, |app| book.read(app).dialog.as_ref().is_some_and(is));
}

#[test]
fn a_pivot_table_is_made_then_changed_in_place() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["B3"]);
    ask(&mut cx, &book, Asked::Pivot, |dialog| {
        matches!(dialog, Dialog::Pivot { editing: false, .. })
    });
    update(&mut cx, &book, |book, cx| {
        let Some(Dialog::Pivot {
            source,
            fields,
            layout,
            ..
        }) = &mut book.dialog
        else {
            unreachable!("a pivot dialog")
        };
        assert!(source.ends_with("$A$1:$C$5"), "the data area: {source}");
        assert_eq!(fields, &["Name", "Qty", "City"], "the header row");
        layout.rows.push("City".into());
        layout.data.push(("Qty".into(), "sum".into()));
        book.pivot_asked(cx)
    });
    settle(&cx, |app| sheet_names(app, &book).len() == 2);
    let made = read(&mut cx, &book, sheet::PIVOT, |json| {
        sheet::pivot(json).expect("parses")
    })
    .expect("the cursor is on the new table");
    assert_eq!(made.layout.rows, ["City"]);
    let shown = cells(&mut cx, &book, "A1:B4");
    assert!(
        shown.contains("x\t6") && shown.contains("y\t3"),
        "{shown:?}"
    );

    run(&mut cx, &book, "cell.goto", &["A2"]);
    ask(&mut cx, &book, Asked::Pivot, |dialog| {
        matches!(dialog, Dialog::Pivot { editing: true, .. })
    });
    let changed = Layout {
        rows: vec!["Name".into()],
        data: vec![("Qty".into(), "max".into())],
        ..Layout::default()
    };
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Pivot { layout, .. }) = &mut book.dialog {
            *layout = changed.clone();
        }
        book.pivot_asked(cx)
    });
    run(&mut cx, &book, "cell.goto", &["A2"]);
    let now = read(&mut cx, &book, sheet::PIVOT, |json| {
        sheet::pivot(json).expect("parses")
    });
    let placed = Layout {
        columns: vec![String::new()],
        ..changed
    };
    assert_eq!(now.expect("still there").layout, placed);
    let sheets = cx.update(|app| sheet_names(app, &book));
    assert_eq!(sheets.len(), 2, "the change made no sheet");
}

#[test]
fn a_chart_is_inserted_then_edited() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["A1:B5"]);
    ask(&mut cx, &book, Asked::Chart, |dialog| {
        matches!(dialog, Dialog::Chart { chart: None, .. })
    });
    let range = cx.update(|app| {
        let Some(Dialog::Chart { fields, .. }) = &book.read(app).dialog else {
            unreachable!("a chart dialog")
        };
        fields.range.read(app).text().to_string()
    });
    assert!(range.ends_with("$A$1:$B$5"), "the picked range: {range}");
    update(&mut cx, &book, |book, cx| {
        let Some(Dialog::Chart { look, fields, .. }) = &mut book.dialog else {
            unreachable!("a chart dialog")
        };
        look.kind = "line".into();
        fields
            .look
            .title
            .update(cx, |input, cx| input.set_text("Qty by name", cx));
        book.chart_asked(cx)
    });
    let chart = |cx: &mut TestAppContext| {
        read(cx, &book, sheet::CHART, |json| {
            sheet::chart(json).expect("parses")
        })
    };
    let made = chart(&mut cx).expect("the new chart is selected");
    assert_eq!(
        (made.look.kind.as_str(), made.look.title.as_str()),
        ("line", "Qty by name")
    );

    ask(&mut cx, &book, Asked::Chart, |dialog| {
        matches!(dialog, Dialog::Chart { chart: Some(_), .. })
    });
    update(&mut cx, &book, |book, cx| {
        if let Some(Dialog::Chart { look, .. }) = &mut book.dialog {
            look.kind = "pie".into();
            look.legend = false;
        }
        book.chart_asked(cx)
    });
    let edited = chart(&mut cx).expect("still selected");
    assert_eq!(
        (edited.look.kind.as_str(), edited.look.legend),
        ("pie", false)
    );
    assert_eq!(edited.look.title, "Qty by name", "the title stays");
}

#[test]
fn asking_on_an_empty_cell_leaves_enter_moving_down() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    blank(&mut cx, &book);
    update(&mut cx, &book, |book, cx| book.ask(Asked::Chart, cx));
    for _ in 0..200 {
        cx.executor().tick();
    }
    quiet(&cx, &book);
    let address = cx.update(|app| {
        let book = book.read(app);
        let tab = book.active_tab().expect("a tab");
        let mut worker = book.core.worker.lock().expect("the worker lock");
        worker.post_key(&tab.doc, '7' as u32, 0).expect("a key");
        worker.post_key(&tab.doc, 0, 1280).expect("Enter");
        let json = worker
            .values(&tab.doc, sheet::SELECTED)
            .expect("the selection");
        sheet::selected_range(&json)
            .expect("parses")
            .expect("one")
            .range
    });
    assert!(address.ends_with("$A$2"), "Enter moved down: {address}");
}
