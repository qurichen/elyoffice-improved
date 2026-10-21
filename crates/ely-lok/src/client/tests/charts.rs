use super::analysis::steps;
use super::show::open_deck;
use super::*;
use crate::show::{self, ChartData};

fn chart(worker: &mut Worker, doc: &Doc) -> show::SlideChart {
    let read = worker.values(doc, crate::sheet::CHART).expect("reads");
    show::slide_chart(&read).expect("parses").expect("a chart")
}

fn select_chart(worker: &mut Worker, doc: &Doc) {
    let objects =
        show::objects(&worker.values(doc, show::OBJECTS).expect("reads")).expect("parses");
    let at = (objects.objects.iter())
        .position(|object| object.kind == "chart")
        .expect("a chart");
    let state = worker.now(doc).expect("the state");
    worker
        .run(&state, "object.select", &[&at.to_string(), "false"])
        .expect("selects");
}

fn look(kind: &str) -> crate::sheet::Look {
    crate::sheet::Look {
        kind: kind.into(),
        legend: true,
        ..Default::default()
    }
}

/// Saves `doc` as PPTX, reopens it and selects its chart.
fn round_trip(worker: &mut Worker, doc: &Doc, path: &std::path::Path) -> Doc {
    doc.save_as(worker, &crate::file_url(path), "pptx")
        .expect("saves");
    let deck = worker.load(&crate::file_url(path)).expect("loads");
    select_chart(worker, &deck);
    deck
}

#[test]
fn a_scatter_keeps_editable_data_through_pptx() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let table = |values: [f64; 2]| ChartData {
        categories: vec!["10".into(), "20".into()],
        series: vec![("North".into(), values.iter().map(|v| Some(*v)).collect())],
    };
    let json = show::chart_json(&look("scatter"), Some(&table([1.0, 2.0])));
    worker.run(&doc, "show.chart", &[&json]).expect("inserts");
    let deck = round_trip(&mut worker, &doc, &dir.path().join("one.pptx"));
    let read = chart(&mut worker, &deck);
    assert_eq!(read.data, Some(table([1.0, 2.0])), "X and Y come back");
    let titled = crate::sheet::Look {
        title: "Spread".into(),
        ..read.look
    };
    let before = steps(&mut worker, &deck);
    let json = show::chart_json(&titled, Some(&table([1.0, 5.0])));
    worker.run(&deck, "show.chart", &[&json]).expect("changes");
    assert_eq!(steps(&mut worker, &deck), before + 1);
    let again = round_trip(&mut worker, &deck, &dir.path().join("two.pptx"));
    let read = chart(&mut worker, &again);
    assert_eq!(
        (read.look.title.as_str(), read.data),
        ("Spread", Some(table([1.0, 5.0])))
    );
}

#[test]
fn a_missing_point_reads_as_none_and_goes_back() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let data = ChartData {
        categories: vec!["Q1".into(), "Q2".into()],
        series: vec![("North".into(), vec![Some(1.0), None])],
    };
    let json = show::chart_json(&look("column"), Some(&data));
    worker.run(&doc, "show.chart", &[&json]).expect("inserts");
    let read = chart(&mut worker, &doc);
    assert_eq!(read.data.as_ref(), Some(&data));
    let titled = crate::sheet::Look {
        title: "Gaps".into(),
        ..read.look
    };
    let json = show::chart_json(&titled, read.data.as_ref());
    worker.run(&doc, "show.chart", &[&json]).expect("retitles");
    assert_eq!(chart(&mut worker, &doc).data, Some(data));
}

#[test]
fn a_chart_field_of_the_wrong_type_is_refused() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let data = r#""data":{"categories":["A"],"series":[{"name":"S","values":[1]}]}"#;
    let shapes = |worker: &mut Worker| {
        let read = worker.values(&doc, show::OBJECTS).expect("reads");
        show::objects(&read).expect("parses").objects.len()
    };
    let (before, objects) = (steps(&mut worker, &doc), shapes(&mut worker));
    for bad in [
        format!(r#"{{"type":"column","legend":"banana",{data}}}"#),
        format!(r#"{{"type":"column","legend":"true",{data}}}"#),
        format!(r#"{{"type":"column","title":123,{data}}}"#),
        format!(r#"{{"type":"column","title":{{"bad":"object"}},{data}}}"#),
        r#"{"type":"column","data":{"categories":["A"],"series":[{"name":"S","values":["1"]}]}}"#
            .into(),
        r#"{"type":"scatter","data":{"categories":["A"],"series":[{"name":"S","values":[1]}]}}"#
            .into(),
    ] {
        assert!(worker.run(&doc, "show.chart", &[&bad]).is_err(), "{bad}");
    }
    assert_eq!(steps(&mut worker, &doc), before, "no undo step");
    assert_eq!(shapes(&mut worker), objects, "no chart");
}

#[test]
fn comments_do_not_shift_object_indices() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_deck();
    let shape = |worker: &mut Worker, kind: &str| {
        worker
            .run(&doc, "draw.shape", &[kind, "true"])
            .expect("draws");
    };
    shape(&mut worker, "rectangle");
    worker
        .run(&doc, "comment.insert", &["Between"])
        .expect("comments");
    shape(&mut worker, "ellipse");
    let read = |worker: &mut Worker| {
        show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses")
    };
    let last = read(&mut worker).objects.len() - 1;
    let state = worker.now(&doc).expect("the state");
    worker
        .run(&state, "object.select", &[&last.to_string(), "false"])
        .expect("selects");
    let objects = read(&mut worker);
    let marked: Vec<bool> = objects.objects.iter().map(|o| o.selected).collect();
    assert_eq!(marked.iter().filter(|on| **on).count(), 1, "{marked:?}");
    assert!(marked[last], "the ellipse, not the comment");
}

/// Where the first chart series' style sits.
fn series_style(xml: &str) -> (usize, usize) {
    let at = xml.find("<chart:series").expect("a series");
    let name = xml[at..]
        .split("chart:style-name=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a series style");
    let start = xml
        .find(&format!("<style:style style:name=\"{name}\""))
        .expect("its style");
    let end = start + xml[start..].find("</style:style>").expect("ends");
    (start, end)
}

#[test]
fn a_retitled_scatter_keeps_its_lines() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let data = ChartData {
        categories: vec!["10".into(), "20".into()],
        series: vec![("North".into(), vec![Some(1.0), Some(2.0)])],
    };
    let json = show::chart_json(&look("scatter"), Some(&data));
    worker.run(&doc, "show.chart", &[&json]).expect("inserts");
    let out = dir.path().join("points.fodp");
    doc.save_as(&mut worker, &crate::file_url(&out), "fodp")
        .expect("saves");
    // Join the points with a line, as another editor would.
    let mut xml = std::fs::read_to_string(&out).expect("reads");
    let (start, end) = series_style(&xml);
    let style = xml[start..end].replace(r#"draw:stroke="none""#, r#"draw:stroke="solid""#);
    assert!(style.contains(r#"draw:stroke="solid""#), "{style}");
    xml.replace_range(start..end, &style);
    let lines = dir.path().join("lines.fodp");
    std::fs::write(&lines, xml).expect("writes");
    let deck = worker.load(&crate::file_url(&lines)).expect("loads");
    select_chart(&mut worker, &deck);
    let titled = crate::sheet::Look {
        title: "Spread".into(),
        ..chart(&mut worker, &deck).look
    };
    let json = show::chart_json(&titled, None);
    worker.run(&deck, "show.chart", &[&json]).expect("retitles");
    let again = dir.path().join("again.fodp");
    deck.save_as(&mut worker, &crate::file_url(&again), "fodp")
        .expect("saves");
    let xml = std::fs::read_to_string(&again).expect("reads");
    let (start, end) = series_style(&xml);
    assert!(
        !xml[start..end].contains(r#"draw:stroke="none""#),
        "{}",
        &xml[start..end]
    );
}
