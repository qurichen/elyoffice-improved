//! Regressions for the C10.2 review.

use ely_lok::show::{Objects, Paint};
use gpui::{Entity, TestAppContext};

use super::objects::{act, blank, objects, selection};
use super::undo_steps;
use crate::deck::Deck;
use crate::object::{NO_FILL, NO_LINE, commit, twips};

/// Commits `value` for `field` the way a panel field does.
fn send(
    cx: &mut TestAppContext,
    deck: &Entity<Deck>,
    field: &'static str,
    command: &'static str,
    value: i64,
    now: Option<i64>,
) {
    let panel = cx.update(|app| deck.read(app).panel.clone());
    cx.update(|app| {
        commit(
            &panel,
            field,
            (value, now),
            move |deck, cx| deck.object_command(command, &[&value.to_string()], cx),
            app,
        )
    });
}

#[test]
fn clear_buttons_clear_fill_and_outline() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    for (_, command, value) in [NO_FILL, NO_LINE] {
        update(&mut cx, &deck, command, value);
    }
    super::settle(&cx, |app| {
        selection(app, &deck).is_some_and(|s| s.fill == Paint::None && s.line == Paint::None)
    });
}

fn update(cx: &mut TestAppContext, deck: &Entity<Deck>, command: &str, value: &str) {
    super::update(cx, deck, |deck, cx| {
        deck.object_command(command, &[value], cx)
    });
}

#[test]
fn a_value_sent_again_after_undo_goes_out() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    let see = |n: i64| {
        move |o: &Objects| {
            o.selection
                .as_ref()
                .is_some_and(|s| s.transparency == Some(n))
        }
    };
    send(
        &mut cx,
        &deck,
        "transparency",
        "object.transparency",
        50,
        Some(0),
    );
    super::settle(&cx, |app| objects(app, &deck).as_ref().is_some_and(see(50)));
    // Undo may drop the selection; pick the shape again.
    super::update(&mut cx, &deck, |deck, cx| {
        deck.object_command("edit.undo", &[], cx)
    });
    super::update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["2".into()], cx)
    });
    super::settle(&cx, |app| objects(app, &deck).as_ref().is_some_and(see(0)));
    send(
        &mut cx,
        &deck,
        "transparency",
        "object.transparency",
        50,
        Some(0),
    );
    super::settle(&cx, |app| objects(app, &deck).as_ref().is_some_and(see(50)));
}

#[test]
fn twin_objects_each_take_the_same_value() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let count = |n: usize| move |o: &Objects| o.objects.len() == n;
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        count(3),
    );
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        count(4),
    );
    let x = |at: usize, n: i64| move |o: &Objects| (o.objects[at].rect.0 - n).abs() <= 1;
    for at in [2, 3] {
        super::update(&mut cx, &deck, move |deck, cx| {
            deck.select_objects(&[at.to_string()], cx)
        });
        super::settle(&cx, |app| {
            objects(app, &deck).is_some_and(|o| o.objects[at].selected)
        });
        let now = cx.update(|app| selection(app, &deck)).expect("one").rect.0;
        let panel = cx.update(|app| deck.read(app).panel.clone());
        cx.update(|app| {
            commit(
                &panel,
                "x",
                (3000, Some(now)),
                |deck, cx| deck.object_command("object.x", &[&twips(3000).to_string()], cx),
                app,
            )
        });
        super::settle(&cx, |app| {
            objects(app, &deck).as_ref().is_some_and(x(at, 3000))
        });
    }
}

#[test]
fn lines_take_an_outline_color_and_width() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(&mut cx, &deck, ("draw.line", &[]), |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "line")
    });
    act(&mut cx, &deck, ("object.line_color", &["16711680"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.line == Paint::Color(0xFF0000))
    });
    act(&mut cx, &deck, ("object.line_width", &["176"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.line_width == Some(176))
    });
}

#[test]
fn an_empty_list_choice_deselects() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        |o| o.selection.is_some(),
    );
    super::update(&mut cx, &deck, |deck, cx| deck.select_objects(&[], cx));
    super::settle(&cx, |app| {
        objects(app, &deck).is_some_and(|o| o.selection.is_none())
    });
}

#[test]
fn a_mixed_value_can_be_set_to_zero() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let count = |n: usize| move |o: &Objects| o.objects.len() == n;
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["rectangle", "true"]),
        count(3),
    );
    act(&mut cx, &deck, ("object.transparency", &["25"]), |o| {
        o.selection
            .as_ref()
            .is_some_and(|s| s.transparency == Some(25))
    });
    act(
        &mut cx,
        &deck,
        ("draw.shape", &["ellipse", "true"]),
        count(4),
    );
    super::update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["2".into(), "3".into()], cx)
    });
    super::settle(&cx, |app| {
        selection(app, &deck).is_some_and(|s| s.count == 2 && s.transparency.is_none())
    });
    let steps = undo_steps(&mut cx, &deck);
    send(
        &mut cx,
        &deck,
        "transparency",
        "object.transparency",
        0,
        None,
    );
    super::settle(&cx, |app| {
        selection(app, &deck).is_some_and(|s| s.transparency == Some(0))
    });
    assert_eq!(undo_steps(&mut cx, &deck), steps + 1);
}

#[test]
fn a_mixed_autofit_takes_one_rule_for_all() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let fit = |rule: &'static str| {
        move |o: &Objects| o.selection.as_ref().is_some_and(|s| s.autofit == rule)
    };
    super::update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["0".into()], cx)
    });
    act(&mut cx, &deck, ("text.fit", &["shrink"]), fit("shrink"));
    super::update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["1".into()], cx)
    });
    act(&mut cx, &deck, ("text.fit", &["grow"]), fit("grow"));
    super::update(&mut cx, &deck, |deck, cx| {
        deck.select_objects(&["0".into(), "1".into()], cx)
    });
    super::settle(&cx, |app| {
        objects(app, &deck).as_ref().is_some_and(fit("mixed"))
    });
    let steps = undo_steps(&mut cx, &deck);
    act(&mut cx, &deck, ("text.fit", &["off"]), fit("off"));
    assert_eq!(undo_steps(&mut cx, &deck), steps + 1, "one step for both");
}
