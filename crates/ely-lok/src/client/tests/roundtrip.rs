//! QA-10: edit, save as DOCX, reopen, compare.

use super::input::{open, paragraphs};
use super::*;

/// Round-trips `doc` through DOCX to flat XML.
fn round(dir: &tempfile::TempDir, worker: &mut Worker, doc: &Doc, name: &str) -> String {
    let docx = dir.path().join(format!("{name}.docx"));
    doc.save_as(worker, &crate::file_url(&docx), "docx")
        .expect("the DOCX saves");
    let back = worker
        .load(&crate::file_url(&docx))
        .expect("the DOCX reopens");
    let flat = dir.path().join(format!("{name}.fodt"));
    back.save_as(worker, &crate::file_url(&flat), "fodt")
        .expect("flattens");
    worker.close_doc(back).expect("closes");
    std::fs::read_to_string(&flat).expect("reads")
}

fn run(worker: &mut Worker, doc: &Doc, id: &str, values: &[&str]) {
    worker
        .run(doc, id, values)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

pub(super) fn type_in(worker: &mut Worker, doc: &Doc, text: &str) {
    for key in text.chars() {
        let code = if key == '\n' { 1280 } else { 0 };
        let char_code = if key == '\n' { 13 } else { key as u32 };
        worker.post_key(doc, char_code, code).expect("a key");
    }
}

#[test]
fn a_contract_keeps_numbering_comments_and_changes() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "list.numbering", &[]);
    type_in(&mut worker, &doc, "Term one\nTerm two\n");
    run(&mut worker, &doc, "comment.insert", &["Check the term"]);
    run(&mut worker, &doc, "track.toggle", &[]);
    type_in(&mut worker, &doc, "Added clause");
    let text = round(&dir, &mut worker, &doc, "contract");
    for mark in [
        "<text:list",
        "Term one",
        "Term two",
        "Check the term",
        "<text:changed-region",
        "Added clause",
    ] {
        assert!(text.contains(mark), "{mark} survives DOCX");
    }
}

#[test]
fn a_long_report_keeps_headings_contents_and_a_split_table() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open(FODT);
    click(&mut worker, &doc, 150, 150);
    run(
        &mut worker,
        &doc,
        "format.style",
        &["Heading 1", "ParagraphStyles"],
    );
    type_in(&mut worker, &doc, " report\n");
    run(&mut worker, &doc, "table.insert", &["2", "80"]);
    run(&mut worker, &doc, "table.repeat_heading", &[]);
    click(&mut worker, &doc, 150, 150);
    run(&mut worker, &doc, "index.insert", &["Contents"]);
    let pages = doc.pages(&mut worker).expect("pages");
    assert!(
        pages.len() >= 3,
        "the table runs over pages: {}",
        pages.len()
    );
    let text = round(&dir, &mut worker, &doc, "report");
    for mark in [
        "<text:table-of-content",
        "text:outline-level=\"1\"",
        "<table:table-header-rows>",
    ] {
        assert!(text.contains(mark), "{mark} survives DOCX");
    }
    assert_eq!(
        text.matches("<table:table-row").count(),
        80,
        "every row survives"
    );
    let heading = paragraphs(
        &text
            .replace("<text:h", "<text:p")
            .replace("</text:h>", "</text:p>"),
    );
    assert!(
        heading.iter().any(|line| line == "Report"),
        "the heading text survives"
    );
}
