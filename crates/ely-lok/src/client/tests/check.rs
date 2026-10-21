use super::*;
use crate::show;

/// A lost font, a lost linked picture, an empty object.
fn trouble() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/show/trouble.fodp")
}

#[test]
fn the_check_finds_fonts_links_and_broken_objects_by_slide() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    // Beside it: an undecodable picture, a linked text.
    let path = dir.path().join("trouble.fodp");
    std::fs::copy(trouble(), &path).expect("copies");
    std::fs::write(dir.path().join("broken.png"), "not a valid image").expect("writes");
    let linked = dir.path().join("linked.odt");
    let blank = worker.load("private:factory/swriter").expect("a text");
    blank.save_as(&mut worker, &crate::file_url(&linked), "odt").expect("saves");
    worker.close_doc(blank).expect("closes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let json = worker.values(&doc, show::SLIDE_CHECK).expect("reads");
    let found = show::check(&json).expect("parses");
    assert_eq!(found.slides.len(), 5);
    assert!(found.slides[0].fonts.iter().any(|f| f == "Nowhere Sans"), "{json}");
    assert!(!found.slides[1].fonts.iter().any(|f| f == "Nowhere Sans"));
    assert_eq!(found.slides[1].links, [("file:///nowhere/gone.png".to_string(), true)], "{json}");
    assert!(found.slides[1].broken.is_empty(), "said once, as a link");
    assert_eq!(found.slides[2].broken, ["Empty"], "{json}");
    assert!(!found.masters.is_empty());
    let mut bad = found.slides[3].broken.clone();
    bad.sort();
    assert_eq!(bad, ["Bad", "Junk"], "{json}");
    assert_eq!(found.slides[3].links.iter().map(|(_, m)| *m).collect::<Vec<_>>(), [false]);
    let ole = |worker: &mut Worker| {
        let json = worker.values(&doc, show::SLIDE_CHECK).expect("reads");
        show::check(&json).expect("parses").slides[4].links.clone()
    };
    let there = ole(&mut worker);
    assert_eq!(there.len(), 1, "the object is a link");
    assert!(there[0].0.ends_with("linked.odt") && !there[0].1);
    std::fs::remove_file(&linked).expect("deletes");
    assert!(ole(&mut worker)[0].1, "a deleted source is missing");
}
