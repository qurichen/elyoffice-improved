use std::time::Duration;

use gpui_kit::{
    FocusHandle, Focusable, Modifiers, TestAppContext, VisualTestContext, test::TestWindowExt,
};

use crate::desk::{focused, open, press};

/// Draws until `done` holds; fails after a second.
fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut VisualTestContext) -> bool) {
    for _ in 0..50 {
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the window did not settle");
}

fn kit_dialog_open(cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find("dialog").is_some())
}

/// Opens Kit's dialog, tabs inside it, closes it.
fn kit_dialog_round_trip(
    cx: &mut TestAppContext,
    open_with: impl FnOnce(&mut VisualTestContext, &Handles),
    back_to: impl Fn(&Handles) -> FocusHandle,
) {
    let (desk, mut cx) = open(cx, true);
    let handles = cx.update(|_, cx| {
        let desk = desk.read(cx);
        Handles {
            button: desk.open.clone(),
            line: desk.line.read(cx).focus_handle(cx),
            code: desk.code.read(cx).focus_handle(cx),
        }
    });
    open_with(&mut cx, &handles);
    settle(&mut cx, kit_dialog_open);
    for _ in 0..3 {
        cx.simulate_keystrokes("tab");
        for handle in [&handles.button, &handles.line, &handles.code] {
            assert!(!focused(&mut cx, handle), "Tab stays in Kit's dialog");
        }
    }
    cx.simulate_keystrokes("escape");
    settle(&mut cx, |cx| !kit_dialog_open(cx));
    assert!(
        focused(&mut cx, &back_to(&handles)),
        "focus returns where it was"
    );
}

struct Handles {
    button: FocusHandle,
    line: FocusHandle,
    code: FocusHandle,
}

#[gpui_kit::test]
fn enter_on_an_ely_button_opens_a_kit_dialog_that_hands_focus_back(cx: &mut TestAppContext) {
    kit_dialog_round_trip(
        cx,
        |cx, handles| {
            cx.update(|window, cx| window.focus(&handles.button, cx));
            press(cx, "enter");
        },
        |handles| handles.button.clone(),
    );
}

#[gpui_kit::test]
fn a_press_on_an_ely_button_leaves_focus_for_the_kit_dialog_to_return(cx: &mut TestAppContext) {
    kit_dialog_round_trip(
        cx,
        |cx, handles| {
            cx.update(|window, cx| window.focus(&handles.line, cx));
            let bounds = cx.debug_bounds("ely-open").expect("Ely's button drew");
            cx.simulate_click(bounds.center(), Modifiers::none());
        },
        |handles| handles.line.clone(),
    );
}

#[gpui_kit::test]
fn an_ely_dialog_over_kit_fields_traps_tab_and_hands_focus_back(cx: &mut TestAppContext) {
    let (desk, mut cx) = open(cx, true);
    let (button, line, code) = cx.update(|_, cx| {
        let desk = desk.read(cx);
        (
            desk.open.clone(),
            desk.line.read(cx).focus_handle(cx),
            desk.code.read(cx).focus_handle(cx),
        )
    });
    cx.update(|window, cx| window.focus(&line, cx));
    desk.update(&mut cx, |desk, cx| {
        desk.ely_dialog = true;
        cx.notify();
    });
    settle(&mut cx, |cx| !focused(cx, &line));
    for _ in 0..3 {
        cx.simulate_keystrokes("tab");
        for handle in [&button, &line, &code] {
            assert!(!focused(&mut cx, handle), "Tab stays in Ely's dialog");
        }
    }
    cx.simulate_keystrokes("escape");
    settle(&mut cx, |cx| focused(cx, &line));
    assert!(
        !desk.read_with(&cx, |desk, _| desk.ely_dialog),
        "Escape closed it"
    );
}
