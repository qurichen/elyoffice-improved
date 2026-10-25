use anyhow::{Context as _, Result, ensure};
use windows::{Win32::UI::Shell::IFileSaveDialog, core::HSTRING};

pub(crate) fn configure(
    dialog: &IFileSaveDialog,
    name: Option<&str>,
    extension: Option<&str>,
) -> Result<()> {
    if let Some(extension) = extension {
        validate_extension(extension)?;
    }
    unsafe {
        if let Some(name) = name {
            dialog.SetFileName(&HSTRING::from(name)).context("setting suggested file name")?;
        }
        if let Some(extension) = extension {
            dialog
                .SetDefaultExtension(&HSTRING::from(extension))
                .context("setting default file extension")?;
        }
    }
    Ok(())
}

fn validate_extension(extension: &str) -> Result<()> {
    ensure!(
        !extension.is_empty()
            && !extension.starts_with('.')
            && !extension.ends_with(['.', ' '])
            && !extension.chars().any(|c| c.is_control() || r#"/\:*?"<>|"#.contains(c)),
        "invalid default file extension"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_extension;

    #[test]
    fn office_extensions_are_accepted() {
        for extension in ["docx", "xlsx", "pptx", "odt", "ods", "odp"] {
            assert!(validate_extension(extension).is_ok());
        }
    }

    #[test]
    fn unicode_and_compound_extensions_are_accepted() {
        for extension in ["文本", "tar.gz", "PDF", "my_ext"] {
            assert!(validate_extension(extension).is_ok());
        }
    }

    #[test]
    fn empty_dot_prefixed_and_trailing_dot_extensions_are_refused() {
        for extension in ["", ".docx", ".", "docx.", "docx "] {
            assert!(validate_extension(extension).is_err(), "{extension}");
        }
    }

    #[test]
    fn paths_and_wildcards_are_refused() {
        for extension in ["a/b", r"a\b", "C:txt", "*", "?", "txt\0", "<txt>", "txt|md", "\"txt"] {
            assert!(validate_extension(extension).is_err(), "{extension:?}");
        }
    }
}
