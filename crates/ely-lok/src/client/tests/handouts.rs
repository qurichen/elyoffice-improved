use super::output::pdf_text;
use super::*;

/// Five slides, the fourth hidden.
const FIVE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:automatic-styles>
  <style:style style:name="dp2" style:family="drawing-page">
   <style:drawing-page-properties presentation:visibility="hidden"/>
  </style:style>
 </office:automatic-styles>
 <office:body><office:presentation>
 <draw:page draw:name="A"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Alpha</text:p></draw:text-box></draw:frame></draw:page>
 <draw:page draw:name="B"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Bravo</text:p></draw:text-box></draw:frame></draw:page>
 <draw:page draw:name="C"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Charlie</text:p></draw:text-box></draw:frame></draw:page>
 <draw:page draw:name="D" draw:style-name="dp2"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Delta</text:p></draw:text-box></draw:frame></draw:page>
 <draw:page draw:name="E"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Echo</text:p></draw:text-box></draw:frame></draw:page>
 </office:presentation></office:body>
</office:document>"#;

fn handouts(pages: Option<(u32, u32)>, per: u8) -> crate::Pdf {
    crate::Pdf {
        pages,
        handouts: per,
        ..crate::Pdf::default()
    }
}

#[test]
fn handouts_lay_shown_slides_in_range_per_page() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install().expect("LOK"), &dir.path().join("p")).expect("up");
    let path = dir.path().join("five.fodp");
    std::fs::write(&path, FIVE).expect("writes");
    let doc = worker.load(&crate::file_url(&path)).expect("loads");
    let pdf = dir.path().join("two.pdf");
    let url = crate::file_url(&pdf);
    doc.export_pdf(&mut worker, &url, &handouts(None, 2)).expect("exports");
    let some = dir.path().join("some.pdf");
    doc.export_pdf(&mut worker, &crate::file_url(&some), &handouts(Some((2, 3)), 4))
        .expect("exports a range");
    assert!(
        doc.export_pdf(&mut worker, &url, &handouts(None, 5)).is_err(),
        "five a page is not a handout layout"
    );
    let (Some(all), Some(part)) = (pdf_text(&pdf), pdf_text(&some)) else {
        eprintln!("SKIP: no pdftotext to read the PDF");
        return;
    };
    let pages: Vec<&str> = all.split('\x0c').filter(|page| !page.trim().is_empty()).collect();
    assert_eq!(pages.len(), 2, "four shown slides, two a page: {all:?}");
    assert!(pages[0].contains("Alpha") && pages[0].contains("Bravo"));
    assert!(pages[1].contains("Charlie") && pages[1].contains("Echo"));
    assert!(!all.contains("Delta"), "hidden slides stay out");
    let pages: Vec<&str> = part.split('\x0c').filter(|page| !page.trim().is_empty()).collect();
    assert_eq!(pages.len(), 1);
    assert!(part.contains("Bravo") && part.contains("Charlie") && !part.contains("Alpha"));
}
