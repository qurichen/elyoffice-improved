use super::analysis::steps;
use super::show::open_deck;
use super::*;
use crate::show::{self, ChartData};

fn chart(worker: &mut Worker, doc: &Doc) -> Option<show::SlideChart> {
    show::slide_chart(&worker.values(doc, crate::sheet::CHART).expect("reads")).expect("parses")
}

fn data(series: &[(&str, [f64; 2])]) -> ChartData {
    ChartData {
        categories: vec!["Q1".into(), "Q2".into()],
        series: series
            .iter()
            .map(|(name, values)| (name.to_string(), values.iter().map(|v| Some(*v)).collect()))
            .collect(),
    }
}

#[test]
fn charts_insert_change_and_undo_with_their_data() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let look = crate::sheet::Look {
        kind: "column".into(),
        title: "Sales".into(),
        legend: true,
        ..Default::default()
    };
    let first = data(&[("North", [1.0, 2.0]), ("South", [3.0, 4.0])]);
    worker
        .run(
            &doc,
            "show.chart",
            &[&show::chart_json(&look, Some(&first))],
        )
        .expect("inserts");
    let read = chart(&mut worker, &doc).expect("the new chart is selected");
    assert_eq!(
        (read.look.kind.as_str(), read.look.title.as_str()),
        ("column", "Sales")
    );
    assert_eq!(read.data.as_ref(), Some(&first));
    let pie = crate::sheet::Look {
        kind: "pie".into(),
        ..look
    };
    let second = data(&[("West", [5.0, 6.0])]);
    worker
        .run(
            &doc,
            "show.chart",
            &[&show::chart_json(&pie, Some(&second))],
        )
        .expect("changes");
    let read = chart(&mut worker, &doc).expect("still selected");
    assert_eq!(
        (read.look.kind.as_str(), read.data.as_ref()),
        ("pie", Some(&second))
    );
    let objects =
        show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    assert_eq!(objects.objects.len(), 2, "changed in place, not added");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &["1", "false"])
        .expect("selects");
    assert_eq!(
        chart(&mut worker, &doc).expect("the chart").data,
        Some(first)
    );
    // A ragged table is refused and nothing changes.
    let bad = r#"{"type":"column","title":"","legend":true,"xtitle":"","ytitle":"","data":{"categories":["A"],"series":[{"name":"S","values":[1,2]}]}}"#;
    let before = steps(&mut worker, &doc);
    assert!(worker.run(&doc, "show.chart", &[bad]).is_err());
    assert_eq!(steps(&mut worker, &doc), before);
}

#[test]
fn notes_set_read_and_undo_as_one_step() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let notes = |worker: &mut Worker| {
        show::notes(&worker.values(&doc, show::NOTES).expect("reads")).expect("parses")
    };
    let blank = notes(&mut worker);
    assert!(
        blank.editable && blank.text.is_empty() && !blank.rich,
        "{blank:?}"
    );
    let before = steps(&mut worker, &doc);
    notes_set(&mut worker, &doc, &["Say hello\nThen the plan", "0"])
        .expect("sets");
    let set = notes(&mut worker);
    assert_eq!(
        (set.text.as_str(), set.rich),
        ("Say hello\nThen the plan", false)
    );
    assert_eq!(steps(&mut worker, &doc), before + 1);
    notes_set(&mut worker, &doc, &["", "0"]).expect("clears");
    assert_eq!(notes(&mut worker).text, "");
    worker.run(&doc, "edit.undo", &[]).expect("undoes");
    assert_eq!(notes(&mut worker).text, "Say hello\nThen the plan");
    // Notes go to the slide named, not the one shown.
    notes_set(&mut worker, &doc, &["For two", "1"])
        .expect("sets");
    assert_eq!(notes(&mut worker).part, Some(0));
    assert_eq!(notes(&mut worker).text, "Say hello\nThen the plan");
    doc.show_sheet(&mut worker, 1).expect("the second slide");
    let second = notes(&mut worker);
    assert_eq!((second.part, second.text.as_str()), (Some(1), "For two"));
}

/// Sets notes as an app does, at a fresh read.
fn notes_set(worker: &mut Worker, doc: &Doc, values: &[&str]) -> io::Result<()> {
    let at = worker.now(doc)?;
    worker.run(&at, "notes.set", values)
}

#[test]
fn notes_for_a_slide_number_need_the_state_it_was_read_in() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    doc.show_sheet(&mut worker, 0).expect("the first slide");
    let read = worker.now(&doc).expect("the state");
    worker.run(&doc, "slide.delete", &[]).expect("deletes the first");
    // Slide 0 was the deleted one when read.
    let stale = worker.run(&read, "notes.set", &["Late", "0"]).expect_err("an older state");
    assert!(crate::is_stale(&stale), "{stale}");
    let kept = show::notes(&worker.values(&doc, show::NOTES).expect("reads")).expect("parses");
    assert_eq!(kept.text, "", "the next slide keeps its notes");
}

#[test]
fn comments_carry_their_slide_author_edits_and_deletes() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let remarks = |worker: &mut Worker| {
        show::remarks(&worker.values(&doc, ".uno:ViewAnnotations").expect("reads")).expect("parses")
    };
    let count = |worker: &mut Worker| {
        let read = worker.values(&doc, show::OBJECTS).expect("reads");
        show::objects(&read).expect("parses").objects.len()
    };
    worker.set_author("Ada Lovelace").expect("the author");
    doc.show_sheet(&mut worker, 1).expect("the second slide");
    let shapes = count(&mut worker);
    worker
        .run(&doc, "comment.insert", &["Check the numbers"])
        .expect("inserts");
    let list = remarks(&mut worker);
    assert_eq!(list.len(), 1);
    assert_eq!(count(&mut worker), shapes, "comments are not objects");
    assert_eq!(
        (list[0].part, list[0].text.as_str(), list[0].author.as_str()),
        (1, "Check the numbers", "Ada Lovelace")
    );
    let id = list[0].id.clone();
    worker
        .run(&doc, "remark.edit", &[&id, "Check the totals"])
        .expect("edits");
    assert_eq!(remarks(&mut worker)[0].text, "Check the totals");
    let ids: Vec<String> = remarks(&mut worker).into_iter().map(|r| r.id).collect();
    for id in ids {
        worker.run(&doc, "remark.delete", &[&id]).expect("deletes");
    }
    assert!(remarks(&mut worker).is_empty());
}

#[test]
fn tables_insert_and_grow_on_a_slide() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    worker
        .run(&doc, "table.insert", &["3", "2"])
        .expect("inserts");
    let read = show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    let table = read.selection.expect("the new table is selected");
    assert_eq!(table.kind, "table");
    let (w, h) = doc.size(&mut worker).expect("the slide");
    let (sw, sh) = (w * 127 / 72, h * 127 / 72);
    let (x, y, tw, th) = table.rect;
    // Centred across; rows grow down from the middle.
    assert!((x + tw / 2 - sw / 2).abs() < 50, "{table:?}");
    assert!(y > 0 && y + th < sh, "on the slide: {table:?}");
    let tall = table.rect.3;
    for id in ["table.row_below", "table.column_right"] {
        let before = steps(&mut worker, &doc);
        worker.run(&doc, id, &[]).expect(id);
        assert_eq!(steps(&mut worker, &doc), before + 1, "{id} is one step");
    }
    let now = show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    let table = now
        .objects
        .iter()
        .find(|o| o.kind == "table")
        .expect("the table");
    assert!(table.rect.3 > tall, "a row more: {table:?}");
}

#[test]
fn notes_keep_one_look_and_refuse_varied_ones() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    notes_set(&mut worker, &doc, &["Say this first", "0"])
        .expect("sets");
    let out = dir.path().join("plain.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp")
        .expect("saves");
    for kind in ["odp", "pptx"] {
        let copy = dir.path().join(format!("plain.{kind}"));
        doc.save_as(&mut worker, &crate::file_url(&copy), kind)
            .expect("saves");
    }
    for name in ["plain.fodp", "plain.odp", "plain.pptx"] {
        let doc = worker
            .load(&crate::file_url(&dir.path().join(name)))
            .expect("loads");
        let read = show::notes(&worker.values(&doc, show::NOTES).expect("reads")).expect("parses");
        assert_eq!(
            (read.text.as_str(), read.rich),
            ("Say this first", false),
            "{name}"
        );
    }
    // PowerPoint styles every run; new text keeps the look.
    let deck = worker
        .load(&crate::file_url(&dir.path().join("plain.pptx")))
        .expect("loads");
    worker
        .run(&deck, "notes.set", &["Then the plan", "0"])
        .expect("sets");
    let again = dir.path().join("again.fodp");
    deck.save_as(&mut worker, &crate::file_url(&again), "fodp")
        .expect("saves");
    let xml = std::fs::read_to_string(&again).expect("reads");
    let span = xml
        .split("Then the plan")
        .next()
        .expect("text")
        .rsplit("<text:span text:style-name=\"")
        .next();
    let style = span
        .and_then(|s| s.split('"').next())
        .expect("a styled span");
    let def = xml
        .split(&format!("<style:style style:name=\"{style}\""))
        .nth(1)
        .expect("its style");
    let def = def.split("</style:style>").next().expect("ends");
    assert!(
        def.contains(r#"fo:font-size="20pt""#) && def.contains("Arial"),
        "{def}"
    );
    // Make one word bold, as another editor would.
    let bold = r#"<style:style style:name="Bold" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>"#;
    let xml = std::fs::read_to_string(&out)
        .expect("reads")
        .replacen(
            "<office:automatic-styles>",
            &format!("<office:automatic-styles>{bold}"),
            1,
        )
        .replacen(
            "<text:p>Say this first</text:p>",
            r#"<text:p>Say <text:span text:style-name="Bold">this</text:span> first</text:p>"#,
            1,
        );
    let rich = dir.path().join("rich.fodp");
    std::fs::write(&rich, xml).expect("writes");
    let doc = worker.load(&crate::file_url(&rich)).expect("loads");
    let read = show::notes(&worker.values(&doc, show::NOTES).expect("reads")).expect("parses");
    assert_eq!((read.text.as_str(), read.rich), ("Say this first", true));
    notes_set(&mut worker, &doc, &["Plain", "0"])
        .expect_err("refuses");
    let kept = show::notes(&worker.values(&doc, show::NOTES).expect("reads")).expect("parses");
    assert_eq!(
        kept.text, "Say this first",
        "varied formatting is not overwritten"
    );
}

#[test]
fn notes_keep_one_paragraph_look_and_refuse_two() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    notes_set(&mut worker, &doc, &["One\nTwo", "0"])
        .expect("sets");
    let out = dir.path().join("two.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp")
        .expect("saves");
    let end = r#"<style:style style:name="End" style:family="paragraph"><style:paragraph-properties fo:text-align="end"/></style:style>"#;
    let styled = |one: &str, two: &str| {
        std::fs::read_to_string(&out)
            .expect("reads")
            .replacen(
                "<office:automatic-styles>",
                &format!("<office:automatic-styles>{end}"),
                1,
            )
            .replacen("<text:p>One</text:p>", one, 1)
            .replacen("<text:p>Two</text:p>", two, 1)
    };
    let both = r#"<text:p text:style-name="End">One</text:p>"#;
    let both = styled(both, &both.replace("One", "Two"));
    let path = dir.path().join("both.fodp");
    std::fs::write(&path, both).expect("writes");
    let deck = worker.load(&crate::file_url(&path)).expect("loads");
    let read = show::notes(&worker.values(&deck, show::NOTES).expect("reads")).expect("parses");
    assert!(!read.rich, "one look throughout");
    worker
        .run(&deck, "notes.set", &["Three", "0"])
        .expect("sets");
    let again = dir.path().join("again.fodp");
    deck.save_as(&mut worker, &crate::file_url(&again), "fodp")
        .expect("saves");
    let xml = std::fs::read_to_string(&again).expect("reads");
    let tag = xml.split(">Three<").next().expect("text");
    let style = tag.rsplit("text:style-name=\"").next().expect("a style");
    let style = style.split('"').next().expect("its name");
    let def = xml
        .split(&format!("<style:style style:name=\"{style}\""))
        .nth(1)
        .expect("its definition");
    let def = def.split("</style:style>").next().expect("ends");
    assert!(def.contains(r#"fo:text-align="end""#), "{def}");
    // One paragraph aligned, one not: plain text would lose it.
    let one = styled(
        r#"<text:p text:style-name="End">One</text:p>"#,
        "<text:p>Two</text:p>",
    );
    let path = dir.path().join("one.fodp");
    std::fs::write(&path, one).expect("writes");
    let deck = worker.load(&crate::file_url(&path)).expect("loads");
    let read = show::notes(&worker.values(&deck, show::NOTES).expect("reads")).expect("parses");
    assert!(read.rich, "the look varies");
    worker
        .run(&deck, "notes.set", &["Three", "0"])
        .expect_err("refuses");
}
