use gpui::{EntityInputHandler, MouseButton, MouseDownEvent, PlatformInput, TestAppContext};

use super::{focus, open};

#[gpui::test]
fn an_external_clear_has_no_character_hit_in_the_old_frame(cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    field.update(cx, |input, cx| input.set_text("abcdefghijklmnop", cx));
    cx.run_until_parked();
    let hit = field.read_with(cx, |input, _| input.bounds_for(11)).unwrap().origin;
    cx.update(|window, cx| field.update(cx, |input, cx| {
        input.set_text("", cx);
        assert_eq!(input.character_index_for_point(hit, window, cx), None);
        assert_eq!(input.selection(), 0..0);
    }));
}

#[gpui::test]
fn a_replacement_never_reuses_an_old_ascii_offset_inside_unicode(cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    field.update(cx, |input, cx| input.set_text("abcdefgh", cx));
    cx.run_until_parked();
    let hit = field.read_with(cx, |input, _| input.bounds_for(1)).unwrap().origin;
    cx.update(|window, cx| field.update(cx, |input, cx| {
        input.set_text("😀", cx);
        assert_eq!(input.character_index_for_point(hit, window, cx), None);
        assert_eq!(input.selection(), 4..4);
    }));
}

#[gpui::test]
fn typing_a_replacement_does_not_hit_the_previous_text(cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    field.update(cx, |input, cx| input.set_text("abcdefghijklmnop", cx));
    cx.run_until_parked();
    let hit = field.read_with(cx, |input, _| input.bounds_for(11)).unwrap().origin;
    cx.update(|window, cx| field.update(cx, |input, cx| {
        input.select(0..16, cx);
        input.insert("中", cx);
        assert_eq!(input.character_index_for_point(hit, window, cx), None);
        assert_eq!(input.selection(), 3..3);
    }));
}

#[gpui::test]
fn composition_replacement_does_not_hit_the_previous_text(cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    field.update(cx, |input, cx| input.set_text("abcdefghijklmnop", cx));
    cx.run_until_parked();
    let hit = field.read_with(cx, |input, _| input.bounds_for(11)).unwrap().origin;
    cx.update(|window, cx| field.update(cx, |input, cx| {
        input.replace_and_mark_text_in_range(Some(0..16), "文", None, window, cx);
        assert_eq!(input.character_index_for_point(hit, window, cx), None);
        assert_eq!(input.selection(), 3..3);
    }));
}

fn click_after_clear(button: MouseButton, clicks: usize, cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    focus(field, cx);
    field.update(cx, |input, cx| input.set_text("abcdefghijklmnop", cx));
    cx.run_until_parked();
    let hit = field.read_with(cx, |input, _| input.bounds_for(11)).unwrap().origin;
    cx.update(|window, cx| {
        field.update(cx, |input, cx| input.set_text("", cx));
        window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent {
            button, position: hit, click_count: clicks, ..Default::default()
        }), cx);
        assert_eq!(field.read(cx).selection(), 0..0);
    });
    cx.simulate_keystrokes("end");
    assert_eq!(field.read_with(cx, |input, _| input.selection()), 0..0);
}

#[gpui::test]
fn a_left_press_after_clear_cannot_create_an_out_of_range_caret(cx: &mut TestAppContext) {
    click_after_clear(MouseButton::Left, 1, cx);
}

#[gpui::test]
fn a_right_press_after_clear_cannot_create_an_out_of_range_caret(cx: &mut TestAppContext) {
    click_after_clear(MouseButton::Right, 1, cx);
}

#[gpui::test]
fn a_triple_press_after_clear_cannot_slice_the_old_line(cx: &mut TestAppContext) {
    click_after_clear(MouseButton::Left, 3, cx);
}

#[gpui::test]
fn the_repainted_text_has_live_unicode_hit_testing(cx: &mut TestAppContext) {
    let (fields, cx) = open(1, false, cx);
    let field = &fields[0];
    field.update(cx, |input, cx| input.set_text("old", cx));
    cx.run_until_parked();
    field.update(cx, |input, cx| input.set_text("a中😀z", cx));
    cx.run_until_parked();
    for (byte, utf16) in [(1, 1), (4, 2), (8, 4)] {
        let hit = field.read_with(cx, |input, _| input.bounds_for(byte)).unwrap().origin;
        let found = cx.update(|window, cx| field.update(cx, |input, cx| {
            input.character_index_for_point(hit, window, cx)
        }));
        assert_eq!(found, Some(utf16));
    }
}
