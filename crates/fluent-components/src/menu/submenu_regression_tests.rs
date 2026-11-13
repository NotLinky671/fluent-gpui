//! Regression tests for the six reproduced submenu defects.
//!
//! Every test drives real key/pointer events through a window. Visibility,
//! placement, fills and hit results are asserted from painted quads separately
//! from the activation callbacks, so "the handler ran" alone can never pass a
//! test whose point is that the row was (or was not) on screen and inside the
//! menu chain.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Background, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, Point, Render,
    TestAppContext, Window, WindowHandle, point, size,
};
use std::cell::Cell;

type Log = Rc<RefCell<Vec<&'static str>>>;
type OpenLog = Rc<RefCell<Vec<bool>>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Case {
    Basic,
    Nested,
    Siblings,
    DuplicateIds,
    Branches,
    Pending(Blocker),
    Rich,
    /// Root opener and its child opener share the id `more`.
    ConsecutiveSameId,
    /// A root menu with padding, a row gap and non-selectable custom content.
    Pad,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Blocker {
    Header,
    Separator,
    Disabled,
}

struct View {
    focus: FocusHandle,
    log: Log,
    opens: OpenLog,
    case: Case,
    dropdown: bool,
    /// When set, the dropdown is controlled with this value.
    open: Option<bool>,
    hide_more: bool,
    drop_more_submenu: bool,
    /// Disables the row that opens the third level while keeping its submenu.
    disable_inner: bool,
    /// Prepends a header to the root menu, shifting every root row down.
    extra_header: bool,
    /// Moves the plain menu close to the right window edge.
    right_edge: bool,
    beneath: Rc<Cell<usize>>,
}

fn leaf(log: &Log, id: &'static str, label: &'static str) -> MenuItem {
    let log = log.clone();
    MenuItem::new(id, label).on_click(move |_, _| log.borrow_mut().push(label))
}

fn basic_child(log: &Log) -> Menu {
    Menu::new("reg-child")
        .item(leaf(log, "child-one", "child-one"))
        .item(leaf(log, "child-two", "child-two"))
}

fn nested_child(log: &Log, disable_inner: bool) -> Menu {
    Menu::new("reg-child")
        .item(
            MenuItem::new("nested", "Nested")
                .disabled(disable_inner)
                .submenu(Menu::new("reg-grandchild").item(leaf(log, "deep", "deep"))),
        )
        .item(leaf(log, "child-two", "child-two"))
}

impl View {
    fn menu(&self, accent: Hsla) -> Menu {
        let log = self.log.clone();
        let mut menu = Menu::new("reg-menu").track_focus(&self.focus);

        match self.case {
            Case::Basic => {
                if self.extra_header {
                    menu = menu.header(MenuHeader::new("Actions"));
                }
                if !self.hide_more {
                    let more = if self.drop_more_submenu {
                        leaf(&log, "more", "More")
                    } else {
                        MenuItem::new("more", "More").submenu(basic_child(&log))
                    };
                    menu = menu.item(more);
                }
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Nested => {
                menu = menu.item(
                    MenuItem::new("more", "More").submenu(nested_child(&log, self.disable_inner)),
                );
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Siblings => {
                menu = menu.item(
                    MenuItem::new("more-a", "More A").submenu(
                        Menu::new("reg-child-a")
                            .item(leaf(&log, "a-one", "a-one"))
                            .item(leaf(&log, "a-two", "a-two")),
                    ),
                );
                menu = menu.item(
                    MenuItem::new("more-b", "More B").submenu(
                        Menu::new("reg-child-b")
                            .item(leaf(&log, "b-one", "b-one"))
                            .item(leaf(&log, "b-two", "b-two")),
                    ),
                );
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::DuplicateIds => {
                let child_log = log.clone();
                menu = menu.item(
                    MenuItem::new("same", "Root Same")
                        .on_click(move |_, _| child_log.borrow_mut().push("root-same"))
                        .submenu(
                            Menu::new("reg-child")
                                .item(MenuItem::new("same", "Child Same").on_click({
                                    let log = log.clone();
                                    move |_, _| log.borrow_mut().push("child-same")
                                }))
                                .item(leaf(&log, "child-two", "child-two")),
                        ),
                );
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Branches => {
                menu = menu.item(
                    MenuItem::new("more-a", "More A").submenu(
                        Menu::new("reg-child-a")
                            .item(leaf(&log, "beta", "Beta"))
                            .item(leaf(&log, "alpha", "Alpha"))
                            .item(leaf(&log, "charlie-a", "Charlie")),
                    ),
                );
                menu = menu.item(
                    MenuItem::new("more-b", "More B").submenu(
                        Menu::new("reg-child-b")
                            .item(leaf(&log, "bravo", "Bravo"))
                            .item(leaf(&log, "charlie-b", "Charlie"))
                            .item(leaf(&log, "delta", "Delta")),
                    ),
                );
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Pending(blocker) => {
                if blocker == Blocker::Header {
                    menu = menu.header(MenuHeader::new("Actions"));
                }
                menu = menu.item(MenuItem::new("more", "More").submenu(basic_child(&log)));
                match blocker {
                    Blocker::Header => {}
                    Blocker::Separator => menu = menu.separator(),
                    Blocker::Disabled => {
                        menu = menu.item(MenuItem::new("disabled", "Disabled").disabled(true));
                    }
                }
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Rich => {
                let child = Menu::new("reg-rich")
                    .header(MenuHeader::new("Section"))
                    .item(leaf(&log, "child-one", "child-one"))
                    .separator()
                    .child(div().id("reg-custom").w(px(120.)).h(px(12.)).bg(accent))
                    .item(leaf(&log, "child-two", "child-two"))
                    // A submenu's own style is applied to that level's panel.
                    .bg(accent);
                menu = menu.item(MenuItem::new("more", "More").submenu(child));
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::ConsecutiveSameId => {
                let grandchild = Menu::new("reg-same-grandchild").item(leaf(&log, "deep", "deep"));
                let child = Menu::new("reg-same-child")
                    .item(MenuItem::new("more", "More again").submenu(grandchild))
                    .item(leaf(&log, "alpha", "Alpha"));
                menu = menu.item(MenuItem::new("more", "More").submenu(child));
                menu = menu.item(leaf(&log, "two", "two"));
            }
            Case::Pad => {
                menu = menu.item(MenuItem::new("more", "More").submenu(basic_child(&log)));
                menu = menu.item(leaf(&log, "two", "two"));
                // Non-selectable custom content is a place the pointer can rest.
                menu = menu.child(div().w(px(120.)).h(px(12.)));
            }
        }

        menu
    }
}

impl Render for View {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let accent = FluentTheme::global(cx)
            .tokens()
            .color
            .palette_blue_background_2;
        let menu = self.menu(accent);
        let content = if self.dropdown {
            let mut dropdown = MenuDropdown::new(
                "reg-dropdown",
                div().id("reg-trigger").w(px(120.)).h(px(28.)).child("open"),
                menu,
            )
            .track_focus(&self.focus);
            dropdown = match self.open {
                Some(open) => dropdown.open(open),
                None => dropdown.default_open(true),
            };
            let opens = self.opens.clone();
            dropdown
                .on_open_change(move |next, _, _| opens.borrow_mut().push(next))
                .into_any_element()
        } else {
            div()
                .w(px(240.))
                .h(px(320.))
                .when(self.right_edge, |el| el.ml(px(300.)))
                .child(menu)
                .into_any_element()
        };

        let beneath = self.beneath.clone();
        FluentRoot::new().child(
            div().size_full().p(px(24.)).child(
                div()
                    .relative()
                    .child(
                        div()
                            .id("reg-beneath")
                            .absolute()
                            .left(px(280.))
                            .top(px(20.))
                            .w(px(280.))
                            .h(px(340.))
                            .on_mouse_down(MouseButton::Left, move |_, _, _| {
                                beneath.set(beneath.get() + 1)
                            }),
                    )
                    .child(content),
            ),
        )
    }
}

// ---------------------------------------------------------------- harness

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("up", crate::MenuPrev, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("home", crate::MenuFirst, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("end", crate::MenuLast, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "right",
                crate::MenuSubmenuOpen,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            KeyBinding::new(
                "left",
                crate::MenuSubmenuClose,
                Some(crate::MENU_KEY_CONTEXT),
            ),
            KeyBinding::new("escape", crate::MenuEscape, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("tab", crate::MenuTabOut, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new(
                "shift-tab",
                crate::MenuTabBackward,
                Some(crate::MENU_KEY_CONTEXT),
            ),
        ]);
    });
}

fn open_case(
    cx: &mut TestAppContext,
    case: Case,
    dropdown: bool,
    open: Option<bool>,
) -> (WindowHandle<View>, Log, OpenLog, Rc<Cell<usize>>) {
    bind_keys(cx);
    let log: Log = Default::default();
    let opens: OpenLog = Default::default();
    let beneath = Rc::new(Cell::new(0));
    let view_log = log.clone();
    let view_opens = opens.clone();
    let view_beneath = beneath.clone();
    let handle = cx.open_window(size(px(600.), px(500.)), move |_, cx| View {
        focus: cx.focus_handle().tab_stop(true),
        log: view_log,
        opens: view_opens,
        case,
        dropdown,
        open,
        hide_more: false,
        drop_more_submenu: false,
        disable_inner: false,
        extra_header: false,
        right_edge: false,
        beneath: view_beneath,
    });
    cx.run_until_parked();

    for _ in 0..2 {
        cx.update_window(handle.into(), |view, window, cx| {
            let view = view.downcast::<View>().unwrap();
            let focus = view.read(cx).focus.clone();
            focus.focus(window, cx);
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
    if dropdown {
        // Let the open reveal finish; geometry and fills are already final, but
        // steady state keeps these tests independent of the animation.
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(400));
        cx.run_until_parked();
        draw(cx, handle);
    }
    (handle, log, opens, beneath)
}

fn draw(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<View>, keys: &[&'static str]) {
    for key in keys {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.update_window(handle.into(), move |_, window, cx| {
            window.dispatch_event(
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        // One frame is enough: placement is computed in the same frame as the
        // row measurement, so tests must not hide a broken first frame.
        draw(cx, handle);
    }
}

fn mouse_move(cx: &mut TestAppContext, handle: WindowHandle<View>, position: Point<Pixels>) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

fn click(cx: &mut TestAppContext, handle: WindowHandle<View>, position: Point<Pixels>) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    draw(cx, handle);
}

fn advance(cx: &mut TestAppContext, handle: WindowHandle<View>, millis: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(millis));
    cx.run_until_parked();
    draw(cx, handle);
}

fn update_view(
    cx: &mut TestAppContext,
    handle: WindowHandle<View>,
    update: impl FnOnce(&mut View),
) {
    cx.update_window(handle.into(), |view, _, cx| {
        view.downcast::<View>().unwrap().update(cx, |view, cx| {
            update(view);
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
}

// ------------------------------------------------------------- painted state

#[derive(Clone, Copy, Debug)]
struct Fill {
    bounds: Bounds<Pixels>,
    background: Background,
}

struct Palette {
    hover: Background,
    hint: Background,
    surface: Background,
    row_height: Pixels,
    surface_min_width: Pixels,
}

fn palette(cx: &mut TestAppContext) -> Palette {
    cx.update(|cx| {
        let theme = FluentTheme::global(cx);
        let item = menu::item(&theme, &MenuItemProps::default());
        let surface = menu::surface(&theme);
        Palette {
            hover: Background::from(item.hover_background),
            hint: Background::from(item.path_hint_background),
            surface: Background::from(surface.background),
            row_height: item.height,
            surface_min_width: surface.min_width,
        }
    })
}

fn painted(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Vec<Fill> {
    let mut quads = Vec::new();
    cx.update_window(handle.into(), |_, window, _| {
        let scale = window.scale_factor();
        quads = window
            .painted_quads()
            .into_iter()
            .map(|quad| Fill {
                bounds: Bounds {
                    origin: point(
                        px(quad.bounds.origin.x.0 / scale),
                        px(quad.bounds.origin.y.0 / scale),
                    ),
                    size: size(
                        px(quad.bounds.size.width.0 / scale),
                        px(quad.bounds.size.height.0 / scale),
                    ),
                },
                background: quad.background,
            })
            .collect();
    })
    .unwrap();
    quads
}

/// Row-sized fills. Inactive rows are transparent, so these are exactly the
/// current-row and path-hint rows.
fn row_fills(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Vec<Fill> {
    let height = f32::from(menu_row_height(cx));
    painted(cx, handle)
        .into_iter()
        .filter(|quad| {
            (f32::from(quad.bounds.size.height) - height).abs() <= 0.5
                && f32::from(quad.bounds.size.width) >= 100.
        })
        .collect()
}

fn menu_row_height(cx: &mut TestAppContext) -> Pixels {
    palette(cx).row_height
}

/// Menu surfaces, one per open level, sorted left to right.
fn panels(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Vec<Fill> {
    let palette = palette(cx);
    let mut panels: Vec<Fill> = painted(cx, handle)
        .into_iter()
        .filter(|quad| {
            f32::from(quad.bounds.size.width) >= f32::from(palette.surface_min_width) - 0.5
                && f32::from(quad.bounds.size.width) < 400.
                && f32::from(quad.bounds.size.height) > 30.
                && quad.background == palette.surface
        })
        .collect();
    panels.sort_by(|a, b| {
        a.bounds
            .origin
            .x
            .partial_cmp(&b.bounds.origin.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    panels
}

fn active_rows(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Vec<Fill> {
    let palette = palette(cx);
    row_fills(cx, handle)
        .into_iter()
        .filter(|fill| fill.background == palette.hover)
        .collect()
}

fn hint_rows(cx: &mut TestAppContext, handle: WindowHandle<View>) -> Vec<Fill> {
    let palette = palette(cx);
    row_fills(cx, handle)
        .into_iter()
        .filter(|fill| fill.background == palette.hint)
        .collect()
}

fn centre(bounds: Bounds<Pixels>) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.,
        bounds.origin.y + bounds.size.height / 2.,
    )
}

fn row_centre(panel: Bounds<Pixels>, row_index: usize) -> Point<Pixels> {
    let theme = TestTheme::default();
    point(
        panel.origin.x + px(40.),
        panel.origin.y
            + theme.padding
            + (theme.row_height + px(2.)) * row_index as f32
            + theme.row_height / 2.,
    )
}

struct TestTheme {
    padding: Pixels,
    row_height: Pixels,
}

impl Default for TestTheme {
    fn default() -> Self {
        Self {
            padding: px(4.),
            row_height: px(32.),
        }
    }
}

fn assert_contains(panel: &Fill, row: &Fill) {
    assert!(
        panel.bounds.contains(&centre(row.bounds)),
        "row {:?} is not inside panel {:?}",
        row.bounds,
        panel.bounds
    );
}

// ------------------------------------------------------------------- tests

#[gpui::test]
fn clicking_a_submenu_child_runs_once_without_a_close_or_fall_through(cx: &mut TestAppContext) {
    let (handle, log, opens, beneath) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "right"]);

    let fills = active_rows(cx, handle);
    assert_eq!(fills.len(), 1, "one active row in the submenu: {fills:?}");
    click(cx, handle, centre(fills[0].bounds));

    assert_eq!(
        *log.borrow(),
        vec!["child-one"],
        "the child callback must run exactly once"
    );
    assert!(
        opens.borrow().is_empty(),
        "an inside press must not request a close: {:?}",
        *opens.borrow()
    );
    assert_eq!(
        beneath.get(),
        0,
        "the press must not fall through to the element behind the submenu"
    );
}

#[gpui::test]
fn clicking_outside_requests_one_close_and_a_controlled_dropdown_stays_open(
    cx: &mut TestAppContext,
) {
    let (handle, _, opens, _) = open_case(cx, Case::Basic, true, None);
    click(cx, handle, point(px(560.), px(460.)));
    assert_eq!(
        *opens.borrow(),
        vec![false],
        "outside press requests a close"
    );

    // The trigger keeps owning the press: no second report for the same click.
    press(cx, handle, &["escape"]);
    assert_eq!(*opens.borrow(), vec![false]);
    let (handle, _, opens, _) = open_case(cx, Case::Basic, true, None);
    click(cx, handle, point(px(24. + 60.), px(24. + 14.)));
    assert_eq!(
        *opens.borrow(),
        vec![false],
        "one press on the trigger reports exactly one close"
    );

    // A controlled caller that ignores the request still shows the open menu.
    let (handle, _, opens, _) = open_case(cx, Case::Basic, true, Some(true));
    click(cx, handle, point(px(560.), px(460.)));
    assert_eq!(*opens.borrow(), vec![false]);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "the controlled caller did not update `open`, so the menu must stay painted"
    );
}

#[gpui::test]
fn hovering_a_root_sibling_makes_enter_run_that_sibling(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "right"]);
    let hint = hint_rows(cx, handle);
    assert_eq!(
        hint.len(),
        1,
        "the parent row shows the path hint: {hint:?}"
    );

    // The root sibling sits directly below the anchor row.
    let sibling = point(
        hint[0].bounds.origin.x + px(30.),
        hint[0].bounds.origin.y + px(32. + 2. + 16.),
    );
    mouse_move(cx, handle, sibling);

    let fills = row_fills(cx, handle);
    assert_eq!(
        fills.len(),
        1,
        "the root sibling is the only current row after hovering it: {fills:?}"
    );
    assert!(
        (fills[0].bounds.origin.y - sibling.y + px(16.)).abs() <= px(0.5),
        "the pointer-selected row is the sibling, not the old parent"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["two"],
        "Enter must run the root sibling, not the stale submenu row"
    );
}

#[gpui::test]
fn switching_between_sibling_submenus_moves_the_path_and_the_target(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Siblings, true, None);
    press(cx, handle, &["down", "right"]);
    assert_eq!(panels(cx, handle).len(), 2);

    let hint_a = hint_rows(cx, handle).pop().expect("More A path hint");
    // More B is the next root row.
    let more_b = point(
        hint_a.bounds.origin.x + px(30.),
        hint_a.bounds.origin.y + px(32. + 2. + 16.),
    );
    mouse_move(cx, handle, more_b);
    advance(cx, handle, 400);

    let hints = hint_rows(cx, handle);
    assert_eq!(hints.len(), 1, "only More B is on the open path: {hints:?}");
    assert!(
        (hints[0].bounds.origin.y - more_b.y + px(16.)).abs() <= px(0.5),
        "the hint must move to More B"
    );
    let panels = panels(cx, handle);
    assert_eq!(panels.len(), 2, "one root panel and one submenu panel");
    let active = active_rows(cx, handle);
    assert_eq!(active.len(), 1);
    assert_contains(&panels[1], &active[0]);

    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["b-one"],
        "Enter must run the new branch's first child"
    );
}

#[gpui::test]
fn moving_from_the_parent_row_into_the_submenu_keeps_the_branch(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "right"]);
    assert_eq!(panels(cx, handle).len(), 2);

    let hint = hint_rows(cx, handle).pop().expect("parent row");
    mouse_move(cx, handle, centre(hint.bounds));
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "hovering the parent row must keep the open branch"
    );

    let child = active_rows(cx, handle).pop().expect("submenu current row");
    mouse_move(cx, handle, centre(child.bounds));
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "moving into the submenu is not leaving the menu chain"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["child-one"],
        "the deepest current row stays the activation target"
    );
}

#[gpui::test]
fn three_levels_paint_and_the_deepest_row_is_the_target(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right"]);
    assert_eq!(panels(cx, handle).len(), 2, "level one paints immediately");

    press(cx, handle, &["right"]);
    let panels = panels(cx, handle);
    assert_eq!(
        panels.len(),
        3,
        "the third level must be painted on the first frame after Right"
    );

    let hints = hint_rows(cx, handle);
    assert_eq!(
        hints.len(),
        2,
        "two parent rows form the open path: {hints:?}"
    );
    for hint in &hints {
        let expected_left = hint.bounds.origin.x + hint.bounds.size.width;
        let panel = panels
            .iter()
            .find(|panel| (panel.bounds.origin.x - expected_left).abs() <= px(0.5))
            .unwrap_or_else(|| {
                panic!(
                    "no panel flush with row {:?}; hints {:?}; panels {:?}",
                    hint.bounds, hints, panels
                )
            });
        assert!(
            (panel.bounds.origin.y - hint.bounds.origin.y).abs() <= px(0.5),
            "the panel must be level with the row that opened it"
        );
    }

    let active = active_rows(cx, handle);
    assert_eq!(active.len(), 1, "only the deepest level has a current row");
    assert_contains(&panels[2], &active[0]);

    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["deep"],
        "the visible deepest row is the one Enter runs"
    );
}

#[gpui::test]
fn the_same_row_id_at_two_levels_keeps_the_paths_distinct(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::DuplicateIds, true, None);
    press(cx, handle, &["down", "right"]);

    let panels = panels(cx, handle);
    assert_eq!(panels.len(), 2);
    let active = active_rows(cx, handle);
    assert_eq!(active.len(), 1);
    assert_contains(&panels[1], &active[0]);

    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["child-same"],
        "the submenu row with the same id as its parent must activate the child"
    );
}

#[gpui::test]
fn removing_or_losing_the_parent_row_truncates_the_real_state(cx: &mut TestAppContext) {
    // A removed root parent: the next Escape closes the whole menu.
    let (handle, _, opens, _) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "right"]);
    assert_eq!(panels(cx, handle).len(), 2);
    update_view(cx, handle, |view| view.hide_more = true);
    draw(cx, handle);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "the open submenu must disappear with its parent row"
    );
    press(cx, handle, &["escape"]);
    assert_eq!(
        opens.borrow().last(),
        Some(&false),
        "Escape at the now-root level must close the menu, not pop a phantom level"
    );

    // A parent that loses its submenu: Right cannot open, Escape closes.
    let (handle, _, opens, _) = open_case(cx, Case::Basic, true, None);
    update_view(cx, handle, |view| view.drop_more_submenu = true);
    draw(cx, handle);
    press(cx, handle, &["down", "right"]);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "a leaf must not create a level"
    );
    press(cx, handle, &["escape"]);
    assert_eq!(opens.borrow().last(), Some(&false));
}

#[gpui::test]
fn disabling_an_inner_parent_truncates_only_the_levels_below_it(cx: &mut TestAppContext) {
    let (handle, _, opens, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right", "right"]);
    assert_eq!(panels(cx, handle).len(), 3);

    update_view(cx, handle, |view| view.disable_inner = true);
    draw(cx, handle);
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "the level opened by the disabled row and deeper must be truncated"
    );

    press(cx, handle, &["escape"]);
    assert!(
        opens.borrow().is_empty(),
        "the remaining submenu absorbs the first Escape"
    );
    press(cx, handle, &["escape"]);
    assert_eq!(
        opens.borrow().last(),
        Some(&false),
        "the second Escape reaches the root now that the path is real"
    );
}

#[gpui::test]
fn leaving_the_chain_or_reaching_a_non_selectable_row_cancels_the_pending_open(
    cx: &mut TestAppContext,
) {
    // Leave the whole menu.
    let (handle, log, _, _) = open_case(cx, Case::Basic, true, None);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);
    mouse_move(cx, handle, point(px(560.), px(460.)));
    advance(cx, handle, 400);
    assert_eq!(panels(cx, handle).len(), 1, "leaving must cancel the hover");
    press(cx, handle, &["enter"]);
    assert!(log.borrow().is_empty(), "no old branch may activate");

    for blocker in [Blocker::Header, Blocker::Separator, Blocker::Disabled] {
        let (handle, log, _, _) = open_case(cx, Case::Pending(blocker), true, None);
        let panel = panels(cx, handle)[0].bounds;
        let more_index = if blocker == Blocker::Header { 1 } else { 0 };
        mouse_move(cx, handle, row_centre(panel, more_index));
        advance(cx, handle, 150);
        // The blocker is above More for a header and directly after it otherwise.
        // Header and a disabled item are full rows; a separator is a 1px line
        // with 4px margins.
        let y = match blocker {
            Blocker::Header => panel.origin.y + px(4. + 16.),
            Blocker::Separator => panel.origin.y + px(4. + 32. + 2. + 4.5),
            Blocker::Disabled => panel.origin.y + px(4. + 32. + 2. + 16.),
        };
        mouse_move(cx, handle, point(panel.origin.x + px(40.), y));
        advance(cx, handle, 400);
        assert_eq!(
            panels(cx, handle).len(),
            1,
            "{blocker:?} must cancel the pending open"
        );
        press(cx, handle, &["enter"]);
        assert!(
            log.borrow().is_empty(),
            "{blocker:?}: nothing may activate after the cancelled hover"
        );
    }
}

#[gpui::test]
fn keyboard_takeover_cancels_a_pending_hover(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Basic, true, None);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);

    // One Down moves the keyboard from the hovered More to two.
    press(cx, handle, &["down"]);
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "keyboard navigation must replace the pending hover"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(*log.borrow(), vec!["two"]);
}

#[gpui::test]
fn right_on_a_leaf_creates_no_level_and_escape_still_closes(cx: &mut TestAppContext) {
    let (handle, log, opens, _) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "down", "right"]);
    assert_eq!(panels(cx, handle).len(), 1, "Right on a leaf opens nothing");
    press(cx, handle, &["escape"]);
    assert_eq!(opens.borrow().last(), Some(&false));
    assert!(log.borrow().is_empty(), "Right must not run the leaf");
}

#[gpui::test]
fn submenu_content_and_style_are_preserved(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Rich, true, None);
    press(cx, handle, &["down", "right"]);

    let accent = Background::from(cx.update(|cx| {
        FluentTheme::global(cx)
            .tokens()
            .color
            .palette_blue_background_2
    }));
    let quads = painted(cx, handle);
    let styled_panel = quads
        .iter()
        .find(|quad| {
            quad.background == accent
                && f32::from(quad.bounds.size.width) >= 160.
                && f32::from(quad.bounds.size.height) > 30.
        })
        .unwrap_or_else(|| panic!("submenu style was not applied: {quads:?}"));
    assert!(
        f32::from(styled_panel.bounds.size.height) > 40.,
        "the panel must include header, items, separator and custom content"
    );
    assert!(
        quads.iter().any(|quad| {
            quad.background == accent
                && (f32::from(quad.bounds.size.height) - 12.).abs() <= 0.5
                && (f32::from(quad.bounds.size.width) - 120.).abs() <= 0.5
        }),
        "custom content inside the submenu must be painted: {quads:?}"
    );

    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["child-one"],
        "header/separator/custom content must not become activation targets"
    );

    // A non-selectable row inside the submenu reports into its own level: a
    // pointer-owned current row clears and Enter has no target, while the parent
    // path hint stays. First take ownership with the pointer on the child row.
    let child = active_rows(cx, handle).pop().expect("submenu current row");
    mouse_move(cx, handle, centre(child.bounds));
    mouse_move(
        cx,
        handle,
        point(
            styled_panel.bounds.origin.x + px(40.),
            styled_panel.bounds.origin.y + px(4. + 16.),
        ),
    );
    assert_eq!(
        active_rows(cx, handle).len(),
        0,
        "hovering the submenu header must clear the pointer-owned row"
    );
    assert_eq!(hint_rows(cx, handle).len(), 1);
    press(cx, handle, &["enter"]);
    assert_eq!(*log.borrow(), vec!["child-one"]);
}

#[gpui::test]
fn escape_and_left_step_one_level_back_while_tab_closes_the_chain(cx: &mut TestAppContext) {
    let (handle, _, opens, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right", "right"]);
    assert_eq!(panels(cx, handle).len(), 3);
    press(cx, handle, &["escape"]);
    assert_eq!(panels(cx, handle).len(), 2);
    assert!(opens.borrow().is_empty());
    press(cx, handle, &["escape"]);
    assert_eq!(panels(cx, handle).len(), 1);
    press(cx, handle, &["escape"]);
    assert_eq!(opens.borrow().last(), Some(&false));

    let (handle, _, _opens, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right", "right", "left"]);
    assert_eq!(panels(cx, handle).len(), 2);
    press(cx, handle, &["left"]);
    assert_eq!(panels(cx, handle).len(), 1);
    press(cx, handle, &["left"]);
    assert_eq!(panels(cx, handle).len(), 1, "Left at the root is a no-op");

    let (handle, _, opens, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right", "right", "tab"]);
    assert_eq!(
        *opens.borrow(),
        vec![false],
        "Tab must close the whole chain with one request"
    );
    assert_eq!(panels(cx, handle).len(), 0);

    let (handle, _, opens, _) = open_case(cx, Case::Nested, true, None);
    press(cx, handle, &["down", "right", "right", "shift-tab"]);
    assert_eq!(*opens.borrow(), vec![false]);
    assert_eq!(panels(cx, handle).len(), 0);
}

#[gpui::test]
fn typeahead_does_not_inherit_a_prefix_across_branches(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Branches, false, None);
    // More A: "b" selects Beta.
    press(cx, handle, &["down", "right", "b", "enter"]);
    assert_eq!(*log.borrow(), vec!["Beta"]);

    // Back to the root, then into More B and type "c": with the old "b" prefix it
    // would search for "bc" and match nothing.
    press(cx, handle, &["left", "down", "right", "c", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Beta", "Charlie"],
        "the prefix must restart for the new branch at the same depth"
    );
}

#[gpui::test]
fn typeahead_does_not_inherit_a_prefix_after_leaving_and_returning(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Branches, false, None);
    press(cx, handle, &["down", "down", "right", "c", "enter"]);
    assert_eq!(*log.borrow(), vec!["Charlie"]);

    // Back out, leave the menu, come back in and type "b": a surviving "c"
    // prefix would look for "cb" and match nothing.
    press(cx, handle, &["left", "home"]);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, centre(panel));
    mouse_move(cx, handle, point(px(560.), px(460.)));
    press(cx, handle, &["home", "down", "right", "b", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Charlie", "Bravo"],
        "leaving the menu must end the typing session"
    );
}

#[gpui::test]
fn a_submenu_flips_left_on_its_first_frame_at_the_window_edge(cx: &mut TestAppContext) {
    let (handle, _, _, _) = open_case(cx, Case::Basic, false, None);
    update_view(cx, handle, |view| view.right_edge = true);
    draw(cx, handle);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "precondition: a single standalone menu"
    );

    press(cx, handle, &["down", "right"]);
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "the submenu must paint on the first frame even when it flips"
    );
    let hint = hint_rows(cx, handle).pop().expect("anchor row");
    let anchor_left = hint.bounds.origin.x;
    let submenu = panels(cx, handle)
        .into_iter()
        .find(|panel| panel.bounds.origin.x < anchor_left)
        .unwrap_or_else(|| panic!("expected a left-flipped panel, hint {hint:?}"));
    assert!(
        (submenu.bounds.origin.x + submenu.bounds.size.width - anchor_left).abs() <= px(0.5),
        "a flipped panel must meet the row's left edge: panel {:?}, row {:?}",
        submenu.bounds,
        hint.bounds
    );
}

#[gpui::test]
fn a_layout_change_repositions_the_open_submenu_on_the_next_frame(cx: &mut TestAppContext) {
    let (handle, _, _, _) = open_case(cx, Case::Basic, true, None);
    press(cx, handle, &["down", "right"]);
    let before = hint_rows(cx, handle).pop().expect("anchor row").bounds;

    // Insert a header above the anchor row. Rows move down; the panel must
    // follow on the first frame after the data change, without another input.
    update_view(cx, handle, |view| view.extra_header = true);
    draw(cx, handle);

    let after = hint_rows(cx, handle).pop().expect("anchor row").bounds;
    assert!(
        (after.origin.y - before.origin.y - px(34.)).abs() <= px(0.5),
        "the anchor row moved down by one row plus gap: {before:?} -> {after:?}"
    );
    let panels = panels(cx, handle);
    let submenu = panels
        .iter()
        .find(|panel| {
            (panel.bounds.origin.x - (after.origin.x + after.size.width)).abs() <= px(0.5)
        })
        .unwrap_or_else(|| panic!("no panel at the moved row: panels {panels:?}, row {after:?}"));
    assert!(
        (submenu.bounds.origin.y - after.origin.y).abs() <= px(0.5),
        "the panel must stay level with the row it hangs off"
    );
}

#[gpui::test]
fn identical_opener_ids_on_adjacent_levels_do_not_block_keyboard_descent(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::ConsecutiveSameId, false, None);
    press(cx, handle, &["down", "right"]);
    assert_eq!(panels(cx, handle).len(), 2, "first level opens");

    press(cx, handle, &["right"]);
    assert_eq!(
        panels(cx, handle).len(),
        3,
        "the child opener has the same id as its parent but belongs to another level"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["deep"],
        "the third level's leaf runs exactly once"
    );
}

#[gpui::test]
fn identical_opener_ids_do_not_block_enter_descent(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::ConsecutiveSameId, false, None);
    press(cx, handle, &["down", "enter"]);
    assert_eq!(panels(cx, handle).len(), 2);
    press(cx, handle, &["enter"]);
    assert_eq!(
        panels(cx, handle).len(),
        3,
        "Enter must open the same-id opener in the child level"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(*log.borrow(), vec!["deep"]);
}

#[gpui::test]
fn identical_opener_ids_do_not_block_hover_descent(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::ConsecutiveSameId, false, None);
    let root = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(root, 0));
    advance(cx, handle, 400);
    assert_eq!(panels(cx, handle).len(), 2, "hover opens the child level");

    let child = panels(cx, handle)[1].bounds;
    mouse_move(cx, handle, row_centre(child, 0));
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        3,
        "hover must open the same-id opener in the child level"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(*log.borrow(), vec!["deep"]);
}

#[gpui::test]
fn typeahead_restarts_after_left_and_right_reenter_the_same_branch(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Branches, false, None);
    press(
        cx,
        handle,
        &["down", "right", "b", "left", "right", "a", "enter"],
    );
    assert_eq!(
        *log.borrow(),
        vec!["Alpha"],
        "re-entering the same level path must start a new typeahead session"
    );
}

#[gpui::test]
fn typeahead_restarts_after_escape_reenters_the_same_branch(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Branches, false, None);
    press(
        cx,
        handle,
        &["down", "right", "b", "escape", "right", "a", "enter"],
    );
    assert_eq!(
        *log.borrow(),
        vec!["Alpha"],
        "Escape must end the previous typeahead session"
    );
}

#[gpui::test]
fn typeahead_still_builds_a_prefix_inside_one_visit(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Branches, false, None);
    press(cx, handle, &["down", "right", "a", "l", "enter"]);
    assert_eq!(
        *log.borrow(),
        vec!["Alpha"],
        "consecutive characters in one visit must still form the prefix al"
    );
}

fn open_pad(cx: &mut TestAppContext) -> WindowHandle<View> {
    let (handle, _, _, _) = open_case(cx, Case::Pad, false, None);
    handle
}

#[gpui::test]
fn moving_to_panel_padding_cancels_the_pending_open(cx: &mut TestAppContext) {
    let handle = open_pad(cx);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);
    mouse_move(
        cx,
        handle,
        point(panel.origin.x + px(40.), panel.origin.y + px(2.)),
    );
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "leaving the opener row for the panel padding must cancel the request"
    );
}

#[gpui::test]
fn moving_to_a_row_gap_cancels_the_pending_open(cx: &mut TestAppContext) {
    let handle = open_pad(cx);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);
    mouse_move(
        cx,
        handle,
        point(panel.origin.x + px(40.), panel.origin.y + px(4. + 32. + 1.)),
    );
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "a row gap is not the opener row"
    );
}

#[gpui::test]
fn moving_to_custom_content_cancels_the_pending_open(cx: &mut TestAppContext) {
    let handle = open_pad(cx);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);
    // padding + two 32px rows + two 2px gaps puts the custom row at y=72..84.
    mouse_move(
        cx,
        handle,
        point(panel.origin.x + px(40.), panel.origin.y + px(78.)),
    );
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "non-selectable custom content must cancel the opener hover"
    );
}

#[gpui::test]
fn resting_on_the_opener_row_still_opens_after_the_delay(cx: &mut TestAppContext) {
    let handle = open_pad(cx);
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "a stationary pointer on the opener must still open it"
    );
}

#[gpui::test]
fn a_padding_move_does_not_clear_a_keyboard_selected_row(cx: &mut TestAppContext) {
    let handle = open_pad(cx);
    press(cx, handle, &["down"]);
    assert_eq!(active_rows(cx, handle).len(), 1);

    let panel = panels(cx, handle)[0].bounds;
    mouse_move(
        cx,
        handle,
        point(panel.origin.x + px(40.), panel.origin.y + px(2.)),
    );
    assert_eq!(
        active_rows(cx, handle).len(),
        1,
        "an unrelated pointer move must not clear the keyboard row"
    );
    press(cx, handle, &["enter"]);
    assert_eq!(
        panels(cx, handle).len(),
        2,
        "the keyboard row still opens its submenu"
    );
}

#[gpui::test]
fn reopening_does_not_revive_the_previous_hover_request(cx: &mut TestAppContext) {
    let (handle, log, _, _) = open_case(cx, Case::Basic, true, Some(true));
    let panel = panels(cx, handle)[0].bounds;
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 150);

    update_view(cx, handle, |view| view.open = Some(false));
    draw(cx, handle);
    assert_eq!(
        panels(cx, handle).len(),
        0,
        "controlled close hides the menu"
    );

    update_view(cx, handle, |view| view.open = Some(true));
    draw(cx, handle);
    advance(cx, handle, 400);
    assert_eq!(
        panels(cx, handle).len(),
        1,
        "the reopened menu must not revive the old submenu request"
    );
    press(cx, handle, &["enter"]);
    assert!(
        log.borrow().is_empty(),
        "nothing may activate from the previous session"
    );

    // The menu is still usable: a fresh hover opens the submenu.
    mouse_move(cx, handle, row_centre(panel, 0));
    advance(cx, handle, 400);
    assert_eq!(panels(cx, handle).len(), 2);
    press(cx, handle, &["enter"]);
    assert_eq!(*log.borrow(), vec!["child-one"]);
}
