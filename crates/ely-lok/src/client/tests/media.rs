use super::analysis::steps;
use super::show::open_deck;
use super::*;
use crate::show;

fn clip() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/clip.mp4")
}

fn media(worker: &mut Worker, doc: &Doc) -> Vec<show::Media> {
    show::media(&worker.values(doc, show::MEDIA).expect("reads")).expect("parses")
}

fn insert(worker: &mut Worker, doc: &Doc, path: &std::path::Path, link: bool) -> io::Result<()> {
    let url = crate::file_url(path);
    worker.run(
        doc,
        "media.insert",
        &[&url, &link.to_string(), "8000", "4500"],
    )
}

#[test]
fn embedded_media_goes_in_mid_slide_and_survives_a_save() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let before = steps(&mut worker, &doc);
    insert(&mut worker, &doc, &clip(), false).expect("inserts");
    assert_eq!(steps(&mut worker, &doc), before + 1);
    let list = media(&mut worker, &doc);
    assert_eq!(list.len(), 1);
    assert!(!list[0].link && !list[0].missing, "{:?}", list[0]);
    assert!(
        list[0].url.starts_with("vnd.sun.star.Package:"),
        "{}",
        list[0].url
    );
    let objects =
        show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    let object = &objects.objects[list[0].index];
    assert_eq!(object.kind, "media");
    let out = dir.path().join("media.odp");
    doc.save_as(&mut worker, &crate::file_url(&out), "odp")
        .expect("saves");
    let again = worker.load(&crate::file_url(&out)).expect("loads");
    let list = media(&mut worker, &again);
    assert_eq!(list.len(), 1);
    assert!(
        !list[0].link && !list[0].missing,
        "the copy plays after a reload: {:?}",
        list[0]
    );
}

#[test]
fn a_linked_file_that_goes_away_reads_as_missing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let copy = dir.path().join("linked.mp4");
    std::fs::copy(clip(), &copy).expect("copies");
    insert(&mut worker, &doc, &copy, true).expect("inserts");
    let list = media(&mut worker, &doc);
    assert!(list[0].link && !list[0].missing, "{:?}", list[0]);
    std::fs::remove_file(&copy).expect("removes");
    assert!(media(&mut worker, &doc)[0].missing);
    let before = steps(&mut worker, &doc);
    let gone = dir.path().join("never.mp4");
    assert!(
        insert(&mut worker, &doc, &gone, false).is_err(),
        "nothing to embed"
    );
    assert_eq!(steps(&mut worker, &doc), before);
}
