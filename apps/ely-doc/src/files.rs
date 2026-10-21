//! The bundled templates (DOC-01).

use gpui::App;

/// The three bundled templates (DOC-01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Template {
    Blank,
    Report,
    Letter,
}

pub const TEMPLATES: [Template; 3] = [Template::Blank, Template::Report, Template::Letter];

impl Template {
    pub fn key(self) -> &'static str {
        match self {
            Template::Blank => "blank",
            Template::Report => "report",
            Template::Letter => "letter",
        }
    }

    /// The paragraphs, as (heading level, text); 0 is body.
    fn lines(self, cx: &App) -> Vec<(u8, String)> {
        let text = |key: &str| ely_ui::i18n::text(cx, key, &[]).to_string();
        match self {
            Template::Blank => vec![(0, String::new())],
            Template::Report => vec![
                (9, text("doc.report.title")),
                (1, text("doc.report.summary")),
                (0, text("doc.body")),
                (1, text("doc.report.details")),
                (0, text("doc.body")),
            ],
            Template::Letter => vec![
                (0, text("doc.letter.sender")),
                (0, text("doc.letter.date")),
                (0, String::new()),
                (0, text("doc.letter.recipient")),
                (0, String::new()),
                (0, text("doc.letter.greeting")),
                (0, text("doc.body")),
                (0, String::new()),
                (0, text("doc.letter.closing")),
                (0, text("doc.letter.sender")),
            ],
        }
    }

    /// Flat ODF text; level 9 is the Title style.
    pub fn fodt(self, cx: &App) -> String {
        let body: String = self
            .lines(cx)
            .into_iter()
            .map(|(level, text)| {
                let text = escape(&text);
                match level {
                    0 => format!("<text:p>{text}</text:p>"),
                    9 => format!("<text:p text:style-name=\"Title\">{text}</text:p>"),
                    level => format!("<text:h text:outline-level=\"{level}\">{text}</text:h>"),
                }
            })
            .collect();
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.3\" office:mimetype=\"application/vnd.oasis.opendocument.text\"><office:body><office:text>{body}</office:text></office:body></office:document>\n"
        )
    }

    /// The template as the picker's markdown preview.
    pub fn preview(self, cx: &App) -> String {
        self.lines(cx)
            .into_iter()
            .map(|(level, text)| match level {
                0 => text,
                9 => format!("# {text}"),
                level => format!("{} {text}", "#".repeat(usize::from(level) + 1)),
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// XML-escapes template text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_text_is_escaped() {
        assert_eq!(escape("a<b & c>"), "a&lt;b &amp; c&gt;");
    }
}
