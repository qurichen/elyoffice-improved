//! The selected objects' panel (SHOW-03 to 06).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::{PropertyGrid, PropertyGroup},
    forms::{NumberInput, Switch},
    primitives::IconName,
    theme::ControlSize,
};
use ely_lok::show::Selection;
use ely_office::panel::{self as kernel, palette, row};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement as _, SharedString, Styled as _,
    Window, div,
};

use crate::deck::Deck;
use crate::design::{Panel, on_deck};

/// Centimetres: 1/100 mm per cm, step, places.
const CM: Unit = (1000.0, 0.1, 2, twips);
/// The swatches' clear buttons.
pub(crate) const NO_FILL: kernel::Clear = ("show.object.no_fill", "object.fill_none", "0");
pub(crate) const NO_LINE: kernel::Clear = ("show.object.no_line", "object.line_none", "0");
/// 1/100 mm in a point.
const POINT: f64 = 2540.0 / 72.0;

/// Shown units: scale, step, places, then what the kernel takes.
type Unit = (f64, f64, usize, fn(i64) -> i64);

/// LOK reads transform arguments as twips.
pub(crate) fn twips(mm100: i64) -> i64 {
    (mm100 * 72 + 63).div_euclid(127)
}

fn same(value: i64) -> i64 {
    value
}

impl kernel::Panel for Panel {
    fn state(&self, uno: &str, cx: &App) -> Option<SharedString> {
        let deck = self.deck.upgrade()?;
        let tab = deck.read(cx).active_tab()?;
        let view = tab.view.read(cx);
        view.state(uno).map(|state| state.to_string().into())
    }

    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        on_deck(&self.deck, cx, |deck, cx| {
            deck.object_command(id, values, cx)
        });
    }
}

/// Sends `value` unless it is there or on its way.
pub(crate) fn commit(
    this: &Entity<Panel>,
    field: impl Into<String>,
    (value, now): (i64, Option<i64>),
    send: impl FnOnce(&mut Deck, &mut Context<Deck>) + 'static,
    cx: &mut App,
) {
    let field = field.into();
    this.update(cx, |panel, cx| {
        let read = panel.deck.upgrade().and_then(|deck| {
            deck.read(cx)
                .active_tab()
                .map(|tab| (tab.id, tab.extra.reads))
        });
        if panel.seen != read {
            panel.sent.clear();
            panel.seen = read;
        }
        let mark = (value, now);
        if now == Some(value) || panel.sent.get(&field) == Some(&mark) {
            log::info!("show: {field} already holds {value}");
            return;
        }
        panel.sent.insert(field, mark);
        on_deck(&panel.deck, cx, send);
    });
}

impl Panel {
    /// Groups for the selection on the shown slide.
    pub(crate) fn object_grid(
        &self,
        selection: &Selection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PropertyGrid {
        let mut grid = PropertyGrid::new("show-object").group(self.geometry(selection, cx));
        let kind = selection.kind.as_str();
        // Lines and pictures have an outline but no fill.
        let fills = !matches!(kind, "image" | "line" | "connector");
        grid = grid.group(self.look(selection, fills, cx));
        if matches!(kind, "text" | "shape" | "mixed") {
            grid = grid.group(self.words(selection, window, cx));
        }
        if kind == "image" {
            grid = grid.group(self.picture(cx));
        }
        if kind == "table" {
            grid = grid.group(self.table_group(cx));
        }
        if kind == "chart" {
            grid = grid.group(self.chart_group(cx));
        }
        if let Some(group) = (kind == "media").then(|| self.media_group(cx)).flatten() {
            grid = grid.group(group);
        }
        grid.group(self.arrange(selection, cx))
    }

    /// A number field: kernel units over `scale`, `step`, `places`.
    fn field(
        &self,
        (field, command): (&'static str, &'static str),
        (now, (scale, step, places, kernel)): (Option<i64>, Unit),
        range: (f64, f64),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let this = cx.entity();
        let label = Deck::text(cx, &format!("show.object.{field}"), &[]);
        let id = SharedString::from(format!("show-{field}"));
        let mut input = NumberInput::new(id, now.unwrap_or(0) as f64 / scale)
            .label(label)
            .range(range.0, range.1)
            .step(step)
            .precision(places)
            .size(ControlSize::Sm)
            .on_commit(move |shown, _, cx| {
                let value = (shown * scale).round() as i64;
                let send = move |deck: &mut Deck, cx: &mut Context<Deck>| {
                    deck.object_command(command, &[&kernel(value).to_string()], cx)
                };
                commit(&this, field, (value, now), send, cx)
            });
        if now.is_none() {
            input = input.blank(Deck::text(cx, "office.props.mixed", &[]));
        }
        div().w_full().max_w_32().child(input).into_any_element()
    }

    fn geometry(&self, sel: &Selection, cx: &mut Context<Self>) -> PropertyGroup {
        let (x, y, w, h) = sel.rect;
        let (cm, at, long) = (CM, (-300.0, 300.0), (0.01, 300.0));
        let place = row([
            self.field(("x", "object.x"), (Some(x), cm), at, cx),
            self.field(("y", "object.y"), (Some(y), cm), at, cx),
        ]);
        let extent = row([
            self.field(("width", "object.width"), (Some(w), cm), long, cx),
            self.field(("height", "object.height"), (Some(h), cm), long, cx),
        ]);
        let this = cx.entity();
        let (now, center) = (sel.rotation, sel.rotation_center);
        let label = Deck::text(cx, "show.object.rotation", &[]);
        let turn = NumberInput::new("show-rotation", now as f64 / 100.0)
            .label(label)
            .range(0.0, 359.9)
            .step(1.0)
            .precision(1)
            .size(ControlSize::Sm)
            .on_commit(move |degrees, _, cx| {
                let angle = (degrees * 100.0).round() as i64 % 36_000;
                let values = [angle, twips(center.0), twips(center.1)].map(|n| n.to_string());
                commit(
                    &this,
                    "rotation",
                    (angle, Some(now)),
                    move |deck, cx| {
                        let values: Vec<&str> = values.iter().map(String::as_str).collect();
                        deck.object_command("object.rotate", &values, cx)
                    },
                    cx,
                )
            });
        let locked = sel.lock_position == Some(true);
        let lock = Switch::new("show-lock", locked)
            .name(Deck::text(cx, "show.object.lock", &[]))
            .on_change({
                let deck = self.deck.clone();
                move |on, _, cx| {
                    let on = on.to_string();
                    on_deck(&deck, cx, |deck, cx| {
                        deck.object_command("object.lock", &[&on, &on], cx)
                    })
                }
            });
        let tip = |key: &str| Deck::text(cx, key, &[]);
        PropertyGroup::new(tip("show.object.geometry"))
            .row(tip("show.object.position"), place)
            .row(tip("show.object.size"), extent)
            .row(tip("show.object.rotation"), div().w_full().max_w_32().child(turn))
            .row(tip("show.object.lock"), lock)
    }

    fn look(&self, sel: &Selection, fills: bool, cx: &mut Context<Self>) -> PropertyGroup {
        let this = cx.entity();
        let fill = palette(
            &this,
            "object.fill",
            (sel.fill, "show.object.no_fill"),
            Some(NO_FILL),
            cx,
        );
        let line = palette(
            &this,
            "object.line_color",
            (sel.line, "show.object.no_line"),
            Some(NO_LINE),
            cx,
        );
        let see = self.field(
            ("transparency", "object.transparency"),
            (sel.transparency, (1.0, 1.0, 0, same)),
            (0.0, 100.0),
            cx,
        );
        let width = self.field(
            ("line_width", "object.line_width"),
            (sel.line_width, (POINT, 0.25, 2, same)),
            (0.0, 20.0),
            cx,
        );
        let shadow = Switch::new("show-shadow", sel.shadow == Some(true))
            .name(Deck::text(cx, "show.object.shadow", &[]))
            .on_change({
                let deck = self.deck.clone();
                move |on, _, cx| {
                    let on = on.to_string();
                    on_deck(&deck, cx, |deck, cx| {
                        deck.object_command("object.shadow", &[&on], cx)
                    })
                }
            });
        let tip = |key: &str| Deck::text(cx, key, &[]);
        let title = if fills {
            "show.object.look"
        } else {
            "show.object.line"
        };
        let mut group = PropertyGroup::new(tip(title));
        if fills {
            group = group
                .row(tip("show.object.fill"), fill)
                .row(tip("show.object.transparency"), see)
                .row(tip("show.object.shadow"), shadow);
        }
        group
            .row(tip("show.object.line"), line)
            .row(tip("show.object.line_width"), width)
    }

    fn chart_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let deck = self.deck.clone();
        let change = Button::new("show-chart-edit", Deck::text(cx, "show.chart.edit", &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| on_deck(&deck, cx, |deck, cx| deck.ask_chart(cx)));
        let name = Deck::text(cx, "show.kind.chart", &[]);
        PropertyGroup::new(name.clone()).row(name, change)
    }

    fn picture(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let deck = self.deck.clone();
        let replace = Button::new("show-replace", Deck::text(cx, "show.object.replace", &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| on_deck(&deck, cx, |deck, cx| deck.pick_picture(true, cx)));
        let crop = kernel::button(
            self,
            "image.crop",
            "show.object.crop",
            "show.object.why_picture",
            cx,
        );
        PropertyGroup::new(Deck::text(cx, "show.object.picture", &[])).row(
            Deck::text(cx, "show.object.picture", &[]),
            row([replace.into_any_element(), crop]),
        )
    }

    fn arrange(&self, sel: &Selection, cx: &mut Context<Self>) -> PropertyGroup {
        let act = |id: &'static str, (icon, key): (Option<IconName>, &str), on: bool| {
            let deck = self.deck.clone();
            let run = move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                on_deck(&deck, cx, |deck, cx| deck.object_command(id, &[], cx))
            };
            let label = Deck::text(cx, key, &[]);
            match icon {
                Some(icon) => IconButton::new(id, icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(label)
                    .disabled(!on)
                    .on_click(run)
                    .into_any_element(),
                None => Button::new(id, label)
                    .size(ControlSize::Sm)
                    .disabled(!on)
                    .on_click(run)
                    .into_any_element(),
            }
        };
        let many = sel.count >= 3;
        let order = row([
            act("image.front", (None, "show.object.front"), true),
            act("image.forward", (None, "show.object.forward"), true),
            act("image.backward", (None, "show.object.backward"), true),
            act("image.back", (None, "show.object.back"), true),
        ]);
        let icon = |icon, key| (Some(icon), key);
        let align = row([
            act(
                "object.align_left",
                icon(IconName::AlignStartVertical, "show.object.align_left"),
                true,
            ),
            act(
                "object.align_center",
                icon(IconName::AlignCenterVertical, "show.object.align_center"),
                true,
            ),
            act(
                "object.align_right",
                icon(IconName::AlignEndVertical, "show.object.align_right"),
                true,
            ),
            act(
                "object.align_top",
                icon(IconName::AlignStartHorizontal, "show.object.align_top"),
                true,
            ),
            act(
                "object.align_middle",
                icon(IconName::AlignCenterHorizontal, "show.object.align_middle"),
                true,
            ),
            act(
                "object.align_bottom",
                icon(IconName::AlignEndHorizontal, "show.object.align_bottom"),
                true,
            ),
        ]);
        let spread = row([
            act(
                "object.spread_across",
                icon(
                    IconName::AlignHorizontalDistributeCenter,
                    "show.object.spread_across",
                ),
                many,
            ),
            act(
                "object.spread_down",
                icon(
                    IconName::AlignVerticalDistributeCenter,
                    "show.object.spread_down",
                ),
                many,
            ),
        ]);
        let group = row([
            act("object.group", (None, "show.object.group"), sel.can_group),
            act(
                "object.ungroup",
                (None, "show.object.ungroup"),
                sel.can_ungroup,
            ),
        ]);
        let tip = |key: &str| Deck::text(cx, key, &[]);
        PropertyGroup::new(tip("show.object.arrange"))
            .row(tip("show.object.order"), order)
            .row(tip("show.object.align_to"), align)
            .row(tip("show.object.spread"), spread)
            .row(tip("show.object.grouping"), group)
    }
}
