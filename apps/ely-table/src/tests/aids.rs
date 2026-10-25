use super::data::{cells, open, run};
use super::*;
use ely_gpui_component::theme::{ActiveTheme as _, ControlSize};
use ely_lok::sheet::Validation;
use gpui::{Modifiers, MouseButton, ParentElement as _, Styled as _, VisualTestContext, point, px};

struct Grid(Entity<Book>);

impl gpui::Render for Grid {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let grid = self.0.update(cx, |book, cx| book.grid(window, cx));
        gpui::div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(grid)
    }
}

fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

#[test]
fn validation_help_and_arrow_do_not_select_the_cells_behind_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let _quit = ely_ui::testing::quit_on_drop(&cx);
    let Some((_window, book)) = book(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    cx.update(ely_gpui_component::init_for_tests);
    open(&mut cx, &book);
    run(&mut cx, &book, "cell.goto", &["E2"]);
    let rule = Validation {
        kind: "list".into(),
        operator: "equal".into(),
        list: vec!["Red".into(), "Green".into()],
        dropdown: true,
        show_input: true,
        input_message: "Pick a color".into(),
        error_style: "stop".into(),
        ..Validation::default()
    };
    run(&mut cx, &book, "rules.validation", &[&rule.to_json()]);
    settle(&cx, |app| {
        book.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.rule.as_ref())
            .is_some_and(|(at, rule)| at.1 == "E2" && rule.input_message == "Pick a color")
    });
    let window = cx.add_window(|_, _| Grid(book.clone()));
    let mut shown = VisualTestContext::from_window(window.into(), &cx);
    frames(&mut shown);
    let (view, help, arrow, handle) = shown.update(|window, app| {
        let view = book.read(app).active_tab().expect("a tab").view.clone();
        let read = view.read(app);
        let (x, y, w, h) = read.cell().cursor.expect("the cell cursor");
        let help = read.origin() + read.place((x, y + h)) + point(px(8.0), px(8.0));
        // The arrow's lower corner hangs below a default row.
        let tall = app
            .theme()
            .control_height(ControlSize::Sm)
            .to_pixels(window.rem_size());
        let corner = point(tall - px(1.0), tall - px(1.0));
        let arrow = read.origin() + read.place((x + w, y)) + corner;
        let handle = read.origin() + read.place((x + w, y + h)) - point(px(1.0), px(1.0));
        (view.clone(), help, arrow, handle)
    });
    let sent = shown.update(|_, app| view.read(app).sent());
    shown.simulate_mouse_down(handle, MouseButton::Left, Modifiers::none());
    assert!(
        shown.update(|_, app| matches!(book.read(app).drag, Some(crate::grid::Drag::Fill { .. })))
    );
    shown.simulate_mouse_move(help, MouseButton::Left, Modifiers::none());
    shown.simulate_mouse_up(help, MouseButton::Left, Modifiers::none());
    assert!(
        shown.update(|_, app| book.read(app).drag.is_none()),
        "releasing over help must end the drag"
    );
    shown.simulate_click(help, Modifiers::none());
    assert_eq!(
        shown.update(|_, app| view.read(app).sent()),
        sent,
        "help never sends a grid click"
    );
    shown.simulate_mouse_down(arrow, MouseButton::Left, Modifiers::none());
    assert_eq!(
        shown.update(|_, app| view.read(app).sent()),
        sent,
        "the arrow press stays outside the grid"
    );
    shown.simulate_mouse_up(arrow, MouseButton::Left, Modifiers::none());
    settle(&shown, |app| book.read(app).choices.is_some());
    frames(&mut shown);
    shown.update(|_, app| {
        assert_eq!(view.read(app).cell().address, "E2");
        assert_eq!(
            view.read(app).sent(),
            sent,
            "the arrow release also stays outside the grid"
        );
        assert_eq!(
            book.read(app).choices.as_ref().expect("the menu").values,
            ["Red", "Green"]
        );
    });
    let red = shown
        .debug_bounds("menu-row-Red")
        .expect("the Red menu item");
    shown.simulate_click(red.center(), Modifiers::none());
    // An empty F2 adds nothing after E2's tab.
    assert_eq!(
        cells(&mut cx, &book, "E2:F2"),
        "Red",
        "the value lands in E2 and F2 stays empty"
    );
}
