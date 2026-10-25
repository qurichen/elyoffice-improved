use ely_office::prelude::*;
use gpui::{AppContext as _, TestAppContext};

use super::{counts, deck, quiet, settle, update};

#[test]
fn an_edited_deck_saves_pptx_and_reopens_clean() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some(deck) = deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| counts(app, &deck) == (1, 1));
    update(&mut cx, &deck, |deck, cx| {
        deck.run_active("slide.insert", &[], cx)
    });
    settle(&cx, |app| counts(app, &deck) == (2, 2));
    quiet(&cx, &deck);
    let (id, window) = cx.update(|app| {
        let deck = deck.read(app);
        (
            deck.core.active.expect("an active tab"),
            deck.core.window.expect("a window"),
        )
    });
    let path = dir.path().join("edited.pptx");
    cx.update_window(window, |_, window, app| {
        deck.update(app, |deck, cx| {
            deck.write_as(id, path.clone(), None, window, cx)
        })
    })
    .expect("the window lives");
    settle(&cx, |app| {
        deck.read(app)
            .active_tab()
            .is_some_and(|tab| tab.name == "edited.pptx" && !tab.dirty(app))
    });
    assert!(
        std::fs::read(&path)
            .expect("a saved PPTX")
            .starts_with(b"PK")
    );
    update(&mut cx, &deck, |deck, cx| {
        deck.close_tab(id, cx);
        deck.open_file(path.clone(), cx);
    });
    settle(&cx, |app| {
        counts(app, &deck) == (2, 2)
            && deck
                .read(app)
                .active_tab()
                .is_some_and(|tab| tab.id != id && tab.name == "edited.pptx" && !tab.dirty(app))
    });
}
