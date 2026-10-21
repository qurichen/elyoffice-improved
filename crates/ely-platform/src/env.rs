/// An environment variable's text; none when unset.
pub fn value(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
}

/// A 0 or 1 switch; anything else warns.
pub fn flag(name: &str) -> Option<bool> {
    flag_from(name, value(name).as_deref())
}

fn flag_from(name: &str, value: Option<&str>) -> Option<bool> {
    match value {
        None => None,
        Some("1") => Some(true),
        Some("0") => Some(false),
        Some(other) => {
            log::warn!("env: {name}={other} is not 0 or 1, ignored");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::flag_from;

    #[test]
    fn only_zero_and_one_are_answers() {
        assert_eq!(flag_from("X", Some("1")), Some(true));
        assert_eq!(flag_from("X", Some("0")), Some(false));
        assert_eq!(flag_from("X", None), None);
        for other in ["", "true", "false", "off", "yes", " 1", "01"] {
            assert_eq!(flag_from("X", Some(other)), None, "{other:?}");
        }
    }
}
