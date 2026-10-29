use gpui_kit::{Focusable, TestAppContext};

use crate::desk::{CODE, focused, open};

fn tab_runs_through_both(cx: &mut TestAppContext, ely_last: bool) {
    let (desk, mut cx) = open(cx, ely_last);
    let (open_button, line, code) = cx.update(|_, cx| {
        let desk = desk.read(cx);
        (desk.open.clone(), desk.line.clone(), desk.code.clone())
    });
    cx.update(|window, cx| window.focus(&open_button, cx));
    cx.simulate_keystrokes("tab");
    let line_focus = cx.update(|_, cx| line.read(cx).focus_handle(cx));
    assert!(
        focused(&mut cx, &line_focus),
        "Tab leaves Ely's button for Kit's field"
    );
    cx.simulate_keystrokes("tab");
    let code_focus = cx.update(|_, cx| code.read(cx).focus_handle(cx));
    assert!(
        focused(&mut cx, &code_focus),
        "Tab reaches Kit's code editor"
    );
    cx.simulate_keystrokes("tab");
    assert!(
        focused(&mut cx, &code_focus),
        "Tab stays in the code editor"
    );
    let text = cx.update(|_, cx| code.read(cx).value().to_string());
    assert!(
        text.len() > CODE.len() && text.trim() == CODE,
        "Tab indents: {text:?}"
    );
    cx.simulate_keystrokes("shift-tab");
    let text = cx.update(|_, cx| code.read(cx).value().to_string());
    assert_eq!(text, CODE, "Shift-Tab outdents");
    cx.update(|window, cx| window.focus(&line_focus, cx));
    cx.simulate_keystrokes("shift-tab");
    assert!(
        focused(&mut cx, &open_button),
        "Shift-Tab returns to Ely's button"
    );
}

#[gpui_kit::test]
fn tab_runs_through_both_with_ely_started_last(cx: &mut TestAppContext) {
    tab_runs_through_both(cx, true);
}

#[gpui_kit::test]
fn tab_runs_through_both_with_kit_started_last(cx: &mut TestAppContext) {
    tab_runs_through_both(cx, false);
}
