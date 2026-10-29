use super::*;

fn guessed(text: &str) -> char {
    Pending::new(PathBuf::from("source.csv"), text.as_bytes().to_vec(), false).how.separator
}

#[test]
fn semicolons_in_later_quoted_fields_do_not_split_comma_data() {
    assert_eq!(guessed("Name,\"a;b;c\"\n"), ',');
    assert_eq!(guessed("Name, \"a;b;c\"\n"), ',');
    assert_eq!(guessed("Name,\"a;b\nc;d\"\n"), ',');
    assert_eq!(guessed("Name,\"say \"\"a;b;c\"\"\"\n"), ',');
}

#[test]
fn commas_in_later_quoted_fields_do_not_split_semicolon_data() {
    assert_eq!(guessed("Name;\"a,b,c\"\n"), ';');
    assert_eq!(guessed("Name; \"a,b,c\"\n"), ';');
    assert_eq!(guessed("Name;\"a,b\nc,d\"\n"), ';');
}

#[test]
fn a_quote_inside_unquoted_text_does_not_hide_the_record_end() {
    assert_eq!(guessed("say\"hello,007\nnext;a;b;c;d\n"), ',');
    assert_eq!(guessed("say\"hello;007\nnext,a,b,c,d\n"), ';');
    assert_eq!(guessed("Name\t\"a,b,c\"\n"), '\t');
}
