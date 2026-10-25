use super::*;

fn compare_tiles(extension: &str, body: &str) {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join(format!("input.{extension}"));
    std::fs::write(&path, body).expect("synthetic document");
    let mut worker = Worker::spawn(&install, &dir.path().join("p")).expect("worker");
    let doc = worker.load(&crate::file_url(&path)).expect("document");
    worker
        .run(&doc, "view.theme", &["COLOR_SCHEME_ELY_LIGHT"])
        .expect("light paper");
    for side in [224u32, 256, 448] {
        doc.visible_area(&mut worker, (0, 0, 7680, 7680), (side, 3840))
            .expect("viewport");
        let whole = doc
            .paint_tile(&mut worker, (0, 0, 7680, 7680), (side * 2, side * 2))
            .expect("whole area");
        let ink = whole
            .pixels
            .chunks_exact(4)
            .filter(|px| px[..3].iter().any(|v| *v < 128))
            .count();
        assert!(ink > 100, "{extension}/{side}: nonempty reference: {ink}");
        if doc.is_sheet() {
            let geometry = crate::sheet::geometry(
                &worker
                    .values(&doc, crate::sheet::GEOMETRY)
                    .expect("sheet geometry"),
            )
            .expect("parsed geometry");
            let ppt = f64::from(side) / 3840.0;
            let expected = (
                (f64::from(geometry.columns.tile(geometry.columns.start(1), ppt)) * ppt).round()
                    as i32,
                (f64::from(geometry.rows.tile(geometry.rows.start(1), ppt)) * ppt).round() as i32,
            );
            let mut edge = (-1i32, -1i32);
            for (index, pixel) in whole.pixels.chunks_exact(4).enumerate() {
                let x = (index as u32 % (side * 2)) as i32;
                let y = (index as u32 / (side * 2)) as i32;
                if pixel[1] > 200 && pixel[0] < 40 && pixel[2] < 40 {
                    edge.0 = edge.0.max(x);
                    edge.1 = edge.1.max(y);
                }
            }
            assert!(
                edge.0 >= 0 && edge.1 >= 0,
                "{side}: the colored cell was painted"
            );
            assert!(
                (edge.0 + 1 - expected.0).abs() <= 3 && (edge.1 + 1 - expected.1).abs() <= 3,
                "{side}: cell paint edge {edge:?} must match header geometry {expected:?}"
            );
        }
        let mut different = 0usize;
        for y in 0..2 {
            for x in 0..2 {
                let tile = doc
                    .paint_tile(&mut worker, (x * 3840, y * 3840, 3840, 3840), (side, side))
                    .expect("separate tile");
                for py in 2..side - 2 {
                    for px in 2..side - 2 {
                        let at = ((py * side + px) * 4) as usize;
                        let to = (((y as u32 * side + py) * side * 2 + x as u32 * side + px) * 4)
                            as usize;
                        if (0..3).any(|c| tile.pixels[at + c].abs_diff(whole.pixels[to + c]) > 48) {
                            different += 1;
                        }
                    }
                }
            }
        }
        let total = (side * 2).pow(2) as usize;
        assert!(
            different * 100 < total,
            "{extension}/{side}: {different}/{total} mismatched pixels"
        );
    }
    worker.close_doc(doc).expect("close");
}

#[test]
fn writer_tiles_match_the_same_whole_region() {
    let text = "<text:p>Round14 WWWW tiled text across boundaries 中文</text:p>".repeat(24);
    compare_tiles("fodt", &FODT.replace("<text:p>Hello 世界</text:p>", &text));
}

#[test]
fn calc_tiles_match_the_same_whole_region() {
    let row = r#"<table:table-row><table:table-cell office:value-type="string"><text:p>Round14 WWWW tiled text across boundaries 中文</text:p></table:table-cell></table:table-row>"#;
    let first = row.replacen(
        "<table:table-cell ",
        "<table:table-cell table:style-name=\"Green\" ",
        1,
    );
    let body = format!(
        r##"<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet"><office:automatic-styles><style:style style:name="Green" style:family="table-cell"><style:table-cell-properties fo:background-color="#00ff00"/></style:style></office:automatic-styles><office:body><office:spreadsheet><table:table table:name="Grid">{}{}</table:table></office:spreadsheet></office:body></office:document>"##,
        first,
        row.repeat(23)
    );
    compare_tiles(
        "fods",
        &format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{body}"),
    );
}

#[test]
fn impress_tiles_match_the_same_whole_region() {
    let text = "WWWW tiled text across boundaries 中文 ".repeat(6);
    compare_tiles("fodp", &show::DECK.replace("First", &text));
}
