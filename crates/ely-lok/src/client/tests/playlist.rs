use super::*;
use crate::show;

/// Footer and number on; slide three hidden.
const FIELDS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
 xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:automatic-styles>
  <style:style style:name="dp1" style:family="drawing-page">
   <style:drawing-page-properties presentation:display-footer="true" presentation:display-page-number="true" presentation:display-date-time="false"/>
  </style:style>
  <style:style style:name="dp2" style:family="drawing-page">
   <style:drawing-page-properties presentation:visibility="hidden"/>
  </style:style>
 </office:automatic-styles>
 <office:master-styles>
  <style:master-page style:name="Plain">
   <draw:frame presentation:class="footer" svg:x="1cm" svg:y="13.5cm" svg:width="10cm" svg:height="1.5cm"><draw:text-box><text:p><presentation:footer/></text:p></draw:text-box></draw:frame>
   <draw:frame presentation:class="page-number" svg:x="20cm" svg:y="13.5cm" svg:width="6cm" svg:height="1.5cm"><draw:text-box><text:p><text:page-number>&lt;number&gt;</text:page-number></text:p></draw:text-box></draw:frame>
  </style:master-page>
 </office:master-styles>
 <office:body><office:presentation>
  <presentation:footer-decl presentation:name="ftr1">Ely footer</presentation:footer-decl>
  <draw:page draw:name="One" draw:style-name="dp1" draw:master-page-name="Plain" presentation:use-footer-name="ftr1"/>
  <draw:page draw:name="Two" draw:style-name="dp1" draw:master-page-name="Plain" presentation:use-footer-name="ftr1"/>
  <draw:page draw:name="Three" draw:style-name="dp2" draw:master-page-name="Plain"/>
 </office:presentation></office:body>
</office:document>"#;

fn open_fields() -> (tempfile::TempDir, Worker, Doc) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("fields.fodp");
    std::fs::write(&path, FIELDS).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    (dir, worker, doc)
}

#[test]
fn the_show_reads_every_slide_with_notes_and_hidden() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_fields();
    let now = worker.now(&doc).expect("the state");
    worker.run(&now, "notes.set", &["Say hello", "1"]).expect("sets");
    let json = worker.values(&doc, show::SLIDE_SHOW).expect("reads");
    let slides = show::slide_show(&json).expect("parses");
    let seen: Vec<(bool, &str)> = slides.iter().map(|s| (s.hidden, s.notes.as_str())).collect();
    assert_eq!(seen, [(false, ""), (false, "Say hello"), (true, "")]);
    assert!(slides.iter().all(|s| s.effects.is_empty() && s.media.is_empty()));
    assert_eq!(slides[0].transition.preset, "");
}

#[test]
fn field_text_lands_where_its_placeholder_is() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (_dir, mut worker, doc) = open_fields();
    // Rows with ink, not white, per layer.
    let inked_rows = |part: usize, worker: &mut Worker| {
        let layers = doc.slide_layers(worker, part, (480, 270)).expect("layers");
        assert!(layers.list.iter().all(|l| l.part == show::Part::Still));
        (layers.list.iter())
            .map(|layer| {
                (0..270)
                    .filter(|y| {
                        (0..480).any(|x| {
                            let at = (y * 480 + x) * 4;
                            layer.pixels[at + 3] > 0 && layer.pixels[at..at + 3] != [255, 255, 255]
                        })
                    })
                    .collect::<Vec<usize>>()
            })
            .collect::<Vec<_>>()
    };
    let shown = inked_rows(1, &mut worker);
    assert_eq!(shown.len(), 3, "background, then footer and number");
    assert!(
        shown[1..].iter().all(|rows| !rows.is_empty() && rows.iter().all(|y| *y > 230)),
        "both sit at the bottom: {shown:?}"
    );
    assert_eq!(inked_rows(2, &mut worker).len(), 1, "fields off, none drawn");
}

#[test]
fn a_pptx_keeps_notes_motion_and_media_editable() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = super::show::open_deck();
    let fade = r#"{"preset":"fade-smoothly","duration":1.5,"advance":"click","time":0,"all":false}"#;
    worker.run(&doc, "slide.transition", &[fade]).expect("a transition");
    worker.run(&doc, "draw.shape", &["rectangle", "true"]).expect("a shape");
    let add = serde_json::json!({ "preset": "ooo-entrance-fly-in" }).to_string();
    worker.run(&doc, "animation.add", &[&add]).expect("an effect");
    let now = worker.now(&doc).expect("the state");
    worker.run(&now, "notes.set", &["Speak slowly", "0"]).expect("notes");
    let clip = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/clip.mp4");
    let url = crate::file_url(&clip);
    worker.run(&doc, "media.insert", &[&url, "false", "8000", "4500"]).expect("media");
    let out = dir.path().join("kept.pptx");
    doc.save_as(&mut worker, &crate::file_url(&out), "pptx").expect("saves");
    let again = worker.load(&crate::file_url(&out)).expect("loads");
    let slides = show::slide_show(&worker.values(&again, show::SLIDE_SHOW).expect("reads")).expect("parses");
    let first = &slides[0];
    assert_eq!((first.transition.preset.as_str(), first.notes.as_str()), ("fade-smoothly", "Speak slowly"));
    assert_eq!(first.effects.iter().map(|e| e.preset.as_str()).collect::<Vec<_>>(), ["ooo-entrance-fly-in"]);
    assert_eq!(first.media.len(), 1);
    assert!(!first.media[0].missing, "{:?}", first.media[0]);
    // Still editable: effect and notes change.
    let change = serde_json::json!({ "index": 0, "duration": 3.0 }).to_string();
    worker.run(&again, "animation.change", &[&change]).expect("changes");
    let now = worker.now(&again).expect("the state");
    worker.run(&now, "notes.set", &["Louder", "0"]).expect("edits notes");
    let slides = show::slide_show(&worker.values(&again, show::SLIDE_SHOW).expect("reads")).expect("parses");
    assert_eq!((slides[0].effects[0].duration, slides[0].notes.as_str()), (3.0, "Louder"));
}
