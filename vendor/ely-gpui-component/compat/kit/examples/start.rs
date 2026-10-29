use ely_gpui_component::{
    Assets,
    buttons::Button,
    primitives::FocusScope,
    theme::{ActiveTheme, Mode, Theme},
    typography::{Heading, Paragraph},
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, IntoElement, ParentElement, Render, Styled,
    Window, WindowOptions,
    component::{
        WindowExt,
        button::Button as KitButton,
        input::{Input, InputState},
        theme::{Theme as KitTheme, ThemeMode},
    },
    div, px,
};

struct Start {
    focus: FocusHandle,
    name: Entity<InputState>,
}

impl Render for Start {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let dark = theme.is_dark();
        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .p_8()
            .bg(theme.colors.bg)
            .child(column(dark, &self.name))
    }
}

fn column(dark: bool, name: &Entity<InputState>) -> impl IntoElement {
    div()
        .w(px(480.))
        .flex()
        .flex_col()
        .gap_4()
        .child(Heading::h1("Ely and GPUI Kit"))
        .child(Paragraph::new(
            "One window, one focus order, one asset source.",
        ))
        .child(Input::new(name))
        .child(
            KitButton::new("kit-dialog")
                .label("Kit dialog")
                .on_click(|_, window, cx| {
                    window.open_dialog(cx, |dialog, _, _| dialog.title("From GPUI Kit"));
                }),
        )
        .child(
            Button::new("mode", if dark { "Light mode" } else { "Dark mode" }).on_click(
                move |_, window, cx| {
                    let (ely, kit) = if dark {
                        (Mode::Light, ThemeMode::Light)
                    } else {
                        (Mode::Dark, ThemeMode::Dark)
                    };
                    Theme::set_mode(ely, cx);
                    KitTheme::change(kit, Some(window), cx);
                },
            ),
        )
}

fn main() {
    gpui_kit::application()
        .with_assets(Assets::before(gpui_kit::assets::Assets))
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            ely_gpui_component::init(cx).expect("Ely failed to start");
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Start {
                        focus,
                        name: cx.new(|cx| InputState::new(window, cx).placeholder("Name")),
                    }
                })
            })
            .expect("window failed to open");
        });
}
