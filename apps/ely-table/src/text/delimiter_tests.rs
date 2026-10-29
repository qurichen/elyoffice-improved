use super::*;

fn guessed(bytes: &[u8], chinese: bool) -> char {
    Pending::new(PathBuf::from("source.csv"), bytes.to_vec(), chinese).how.separator
}

#[test]
fn commas_inside_a_quoted_field_do_not_split_semicolon_data() {
    assert_eq!(guessed(b"\"a,b,c\";00123\nnext;00456\n", false), ';');
}

#[test]
fn semicolons_inside_a_quoted_field_do_not_split_comma_data() {
    assert_eq!(guessed(b"\"a;b;c\",00123\nnext,00456\n", false), ',');
}

#[test]
fn pipes_inside_a_quoted_field_do_not_split_comma_data() {
    assert_eq!(guessed(b"\"a|b|c\",00123\n", false), ',');
}

#[test]
fn a_quoted_first_record_can_span_physical_lines() {
    assert_eq!(guessed(b"\"a,b,c\nmore,commas\";00123\n", false), ';');
}

#[test]
fn doubled_quotes_keep_their_separators_inside_the_field() {
    assert_eq!(guessed(b"\"say \"\"a,b,c\"\"\";00123\n", false), ';');
}

fn utf16(text: &str, big: bool) -> Vec<u8> {
    let mut bytes = if big { vec![0xfe, 0xff] } else { vec![0xff, 0xfe] };
    for ch in text.encode_utf16() {
        bytes.extend(if big { ch.to_be_bytes() } else { ch.to_le_bytes() });
    }
    bytes
}

#[test]
fn utf8_bom_does_not_hide_a_quoted_first_field() {
    assert_eq!(guessed(b"\xef\xbb\xbf\"a,b,c\";00123\n", false), ';');
}

#[test]
fn utf16_quoted_records_use_decoded_characters() {
    for big in [false, true] {
        let bytes = utf16("\"中文,a,b\";00123\r\n", big);
        assert_eq!(guessed(&bytes, false), ';', "big endian={big}");
    }
}

#[test]
fn a_pipe_byte_inside_gb18030_is_not_a_separator() {
    let bytes = [0x81, 0x7c, 0x81, 0x7c, b',', b'0', b'0', b'7', b'\n'];
    let pending = Pending::new(PathBuf::from("gb.csv"), bytes.to_vec(), true);
    assert_eq!(pending.how.encoding, Encoding::Gb18030);
    assert_eq!(pending.how.separator, ',');
}

#[test]
fn ordinary_defaults_and_the_tsv_extension_keep_their_meaning() {
    assert_eq!(guessed(b"a;b;c,d\n", false), ';');
    assert_eq!(guessed(b"one\n", false), ',');
    assert_eq!(guessed(b"", false), ',');
    let pending = Pending::new(PathBuf::from("named.TSV"), b"\"a,b,c\"\t007\n".to_vec(), false);
    assert_eq!(pending.how.separator, '\t');
}
