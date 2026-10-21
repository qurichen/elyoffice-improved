//! The blank workbook a new window starts from (TABLE-01).

use gpui::App;

/// A one-sheet flat ODF workbook with a local name.
pub fn blank(cx: &App) -> String {
    let name = ely_ui::i18n::text(cx, "table.sheet.default", &[("n", "1")]);
    let name = name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.spreadsheet\"><office:body><office:spreadsheet><table:table table:name=\"{name}\"/></office:spreadsheet></office:body></office:document>\n"
    )
}
