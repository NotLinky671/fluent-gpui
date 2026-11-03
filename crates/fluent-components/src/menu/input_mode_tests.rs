//! Pointer and keyboard sharing one current row, plus what happens when the rows
//! around the current one change.
//!
//! Action logs check activation, while painted quads independently check which
//! rows actually receive the current-row background. Dynamic rows are inserted
//! and removed from the element tree, not simulated by disabling them.

use super::*;
use crate::root::FluentRoot;
use gpui::{
    AppContext, Context, FocusHandle, KeyBinding, KeyDownEvent, Keystroke, Modifiers,
    MouseMoveEvent, PlatformInput, Point, Render, TestAppContext, Window, WindowHandle, point,
    size,
};

type Log = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;

const ROWS: [&str; 3] = ["One", "Two", "Three"];

/// Row geometry, from `spec::menu`: 4px panel padding, 32px rows, 2px gaps.
const ROW_MIDDLE: [f32; 3] = [20., 52., 86.];

struct View {
    focus: FocusHandle,
    log: Log,
    disabled: Vec<usize>,
    rows: Vec<usize>,
    /// Adds a header row above the items, as the gallery menu has.
    header: bool,
}

impl Render for View {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = Menu::new("coordination-menu").track_focus(&self.focus);
        if self.header {
            menu = menu.header(MenuHeader::new("Actions"));
        }
        for &index in &self.rows {
            let label = &ROWS[index];
            let log = self.log.clone();
            menu = menu.item(
                MenuItem::new(*label, label.to_string())
                    .disabled(self.disabled.contains(&index))
                    .on_click(move |_, _| log.borrow_mut().push(label)),
            );
        }

        FluentRoot::new().child(div().size_full().child(menu))
    }
}

fn bind_keys(cx: &mut TestAppContext) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| {
        cx.bind_keys([
            KeyBinding::new("down", crate::MenuNext, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("up", crate::MenuPrev, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("enter", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
            KeyBinding::new("space", crate::MenuActivate, Some(crate::MENU_KEY_CONTEXT)),
        ]);
    });
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<View>, Log) {
    open_with(cx, false)
}

/// A menu with a header row above the items, shifting every row down by 34px.
fn open_with_header(cx: &mut TestAppContext) -> (WindowHandle<View>, Log) {
    open_with(cx, true)
}

fn open_with(cx: &mut TestAppContext, header: bool) -> (WindowHandle<View>, Log) {
    bind_keys(cx);
    let log: Log = Default::default();
    let log_for_view = log.clone();
    let handle = cx.open_window(size(px(600.), px(500.)), move |_, cx| View {
        focus: cx.focus_handle().tab_stop(true),
        log: log_for_view,
        disabled: Vec::new(),
        rows: vec![0, 1, 2],
        header,
    });
    cx.run_until_parked();

    // Focus and the first two draws: tab stops and hitboxes are recorded while
    // painting and are only usable on the frame after that.
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

    (handle, log)
}

fn settle(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    for _ in 0..2 {
        cx.update_window(handle.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<View>, key: &'static str) {
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
    settle(cx, handle);
}

/// Moves the pointer into the middle of the row at `index` and lets hover settle.
fn hover_row(cx: &mut TestAppContext, handle: WindowHandle<View>, index: usize) {
    let position: Point<gpui::Pixels> = point(px(40.), px(ROW_MIDDLE[index]));
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
    settle(cx, handle);
}

fn set_disabled(cx: &mut TestAppContext, handle: WindowHandle<View>, rows: Vec<usize>) {
    cx.update_window(handle.into(), move |view, _, cx| {
        view.downcast::<View>().unwrap().update(cx, |view, cx| {
            view.disabled = rows;
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

fn set_rows(cx: &mut TestAppContext, handle: WindowHandle<View>, rows: Vec<usize>) {
    cx.update_window(handle.into(), move |view, _, cx| {
        view.downcast::<View>().unwrap().update(cx, |view, cx| {
            view.rows = rows;
            cx.notify();
        });
    })
    .unwrap();
    // Draw directly after the caller's update, before advancing deferred work.
    // Callers inspect this scene, then settle separately to check later frames.
    draw_once(cx, handle);
}

fn draw_once(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}

/// Check actual row-sized painted quads, including colour and scaled bounds.
/// Transparent inactive rows emit no fill; this also catches a second local hover.
fn assert_current_fill(cx: &mut TestAppContext, handle: WindowHandle<View>, row: Option<usize>) {
    cx.update_window(handle.into(), move |_, window, cx| {
        let scale = window.scale_factor();
        let theme = FluentTheme::global(cx);
        let spec = menu::item(&theme, &MenuItemProps::default());
        let fills: Vec<_> = window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                quad.bounds.size.height.0 == f32::from(spec.height) * scale
                    && quad.bounds.size.width.0 >= 100. * scale
            })
            .collect();
        assert_eq!(
            fills.len(),
            usize::from(row.is_some()),
            "row fills: {fills:?}"
        );
        if let Some(row) = row {
            assert_eq!(
                fills[0].background,
                gpui::Background::from(spec.hover_background)
            );
            assert_eq!(fills[0].bounds.origin.y.0 / scale, 4. + row as f32 * 34.);
        }
    })
    .unwrap();
}

#[gpui::test]
fn removing_an_earlier_row_under_a_stationary_pointer_keeps_identity(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    hover_row(cx, handle, 1);
    assert_current_fill(cx, handle, Some(1));
    set_rows(cx, handle, vec![1, 2]);
    assert_current_fill(cx, handle, Some(0));
    settle(cx, handle);
    assert_current_fill(cx, handle, Some(0));
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["Two"]);
}

#[gpui::test]
fn inserting_before_the_current_row_under_a_stationary_pointer_keeps_identity(
    cx: &mut TestAppContext,
) {
    let (handle, log) = open(cx);
    set_rows(cx, handle, vec![1, 2]);
    hover_row(cx, handle, 1); // Three, with One absent.
    assert_current_fill(cx, handle, Some(1));
    set_rows(cx, handle, vec![0, 1, 2]);
    assert_current_fill(cx, handle, Some(2));
    settle(cx, handle);
    assert_current_fill(cx, handle, Some(2));
    press(cx, handle, "space");
    assert_eq!(*log.borrow(), vec!["Three"]);
}

#[gpui::test]
fn keyboard_and_pointer_paint_one_current_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    assert_current_fill(cx, handle, Some(0));
    hover_row(cx, handle, 1);
    assert_current_fill(cx, handle, Some(1));
    press(cx, handle, "down");
    assert_current_fill(cx, handle, Some(2));
    // Moving again inside the same row must reclaim it without crossing an edge.
    hover_row(cx, handle, 1);
    assert_current_fill(cx, handle, Some(1));
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["Two"]);
}

/// Moves the pointer well outside the menu and lets hover settle.
fn leave_menu(cx: &mut TestAppContext, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(520.), px(430.)),
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

#[gpui::test]
fn hovering_moves_the_row_that_enter_activates(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    // Precondition: the keyboard's row is One.
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["One"], "precondition: Down selects One");
    log.borrow_mut().clear();

    hover_row(cx, handle, 1);
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "after hovering Two, Enter must activate Two rather than the stale keyboard row"
    );
}

#[gpui::test]
fn space_also_follows_the_hovered_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    hover_row(cx, handle, 2);
    press(cx, handle, "space");
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "Space must activate the same row Enter would"
    );
}

#[gpui::test]
fn keyboard_continues_from_the_hovered_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    hover_row(cx, handle, 1);
    press(cx, handle, "down");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Three"],
        "once the pointer selected Two, Down must move to Three"
    );
}

#[gpui::test]
fn hovering_a_disabled_row_changes_nothing(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    set_disabled(cx, handle, vec![1]);
    press(cx, handle, "down");
    log.borrow_mut().clear();

    hover_row(cx, handle, 1);
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "a disabled row must not become the current row through the pointer"
    );
}

#[gpui::test]
fn a_disabled_row_is_never_activated_by_enter(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    set_disabled(cx, handle, vec![0]);
    // The first Down must skip One entirely and land on Two.
    press(cx, handle, "down");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "the keyboard must never land on a disabled row"
    );
}

#[gpui::test]
fn redrawing_keeps_the_current_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    // Redraw with no input at all; a state that only lived for one frame would
    // lose the selection here.
    settle(cx, handle);
    settle(cx, handle);
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "the current row must survive redraws"
    );
}

#[gpui::test]
fn disabling_an_earlier_row_keeps_the_current_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    press(cx, handle, "down");
    log.borrow_mut().clear();

    // One disappears from the selectable rows; Two must stay current even though
    // its position in that list changed.
    set_disabled(cx, handle, vec![0]);
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["Two"],
        "disabling an earlier row must not retarget the current row to a different action"
    );
}

#[gpui::test]
fn disabling_the_current_row_leaves_no_action(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    press(cx, handle, "down");
    log.borrow_mut().clear();

    set_disabled(cx, handle, vec![1]);
    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "a current row that became disabled must not silently activate its neighbour, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn removing_the_current_row_leaves_no_action(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    press(cx, handle, "down");
    log.borrow_mut().clear();

    // Actually remove Two from the element tree, including its hitbox.
    set_rows(cx, handle, vec![0, 2]);
    assert_current_fill(cx, handle, None);
    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "with the current row gone, Enter must do nothing, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn leaving_the_menu_clears_a_pointer_selected_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    hover_row(cx, handle, 1);
    log.borrow_mut().clear();

    leave_menu(cx, handle);
    // Painted result, not just the callback: no row may still carry the current fill.
    assert_current_fill(cx, handle, None);
    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "with the pointer gone, the row it selected must stop being current and Enter must do nothing, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn a_pointer_leave_does_not_wipe_out_a_keyboard_selection(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    press(cx, handle, "down");
    log.borrow_mut().clear();

    // The pointer never entered a row, so it owns nothing and must not clear the
    // row the keyboard put in place.
    leave_menu(cx, handle);
    assert_current_fill(cx, handle, Some(0));
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "a pointer that only passes outside the menu must not clear the keyboard selection"
    );
}

#[gpui::test]
fn keyboard_navigation_after_leaving_starts_from_the_top(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    hover_row(cx, handle, 2);
    leave_menu(cx, handle);
    log.borrow_mut().clear();

    press(cx, handle, "down");
    press(cx, handle, "enter");
    assert_eq!(
        *log.borrow(),
        vec!["One"],
        "after the pointer-selected row was cleared, Down must start from the first row again"
    );
}

/// Moves the pointer to an arbitrary y inside the menu and lets it settle.
fn hover_at(cx: &mut TestAppContext, handle: WindowHandle<View>, y: f32) {
    cx.update_window(handle.into(), move |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position: point(px(40.), px(y)),
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    settle(cx, handle);
}

/// Counts painted row-sized fills, whatever their position.
fn row_fill_count(cx: &mut TestAppContext, handle: WindowHandle<View>) -> usize {
    let mut count = 0;
    cx.update_window(handle.into(), |_, window, cx| {
        let scale = window.scale_factor();
        let theme = FluentTheme::global(cx);
        let spec = menu::item(&theme, &MenuItemProps::default());
        count = window
            .painted_quads()
            .into_iter()
            .filter(|quad| {
                quad.bounds.size.height.0 == f32::from(spec.height) * scale
                    && quad.bounds.size.width.0 >= 100. * scale
            })
            .count();
    })
    .unwrap();
    count
}

#[gpui::test]
fn hovering_a_header_clears_a_pointer_selected_row(cx: &mut TestAppContext) {
    let (handle, log) = open_with_header(cx);

    // With a header the first item sits 34px lower: header [4,36), gap, rows from 38.
    hover_at(cx, handle, 52.);
    assert_eq!(
        row_fill_count(cx, handle),
        1,
        "hovering the first item fills it"
    );

    // Onto the header: not a selectable row, so nothing may look current.
    hover_at(cx, handle, 20.);
    assert_eq!(
        row_fill_count(cx, handle),
        0,
        "moving onto the header must clear the row the pointer selected"
    );

    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "with the pointer on a header, Enter must do nothing, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn a_header_does_not_clear_a_keyboard_selection(cx: &mut TestAppContext) {
    let (handle, log) = open_with_header(cx);
    press(cx, handle, "down");
    assert_eq!(row_fill_count(cx, handle), 1);

    hover_at(cx, handle, 20.);
    assert_eq!(
        row_fill_count(cx, handle),
        1,
        "the keyboard's row must survive the pointer resting on a header"
    );
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["One"]);
}

#[gpui::test]
fn hovering_a_disabled_row_clears_a_pointer_selected_row(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    set_disabled(cx, handle, vec![1]);

    // Row three (y=86) is enabled and selectable.
    hover_at(cx, handle, 86.);
    assert_eq!(row_fill_count(cx, handle), 1, "hovering Delete fills it");

    // Onto the disabled row two: it cannot become current, but it is still a place
    // the pointer moved to, so the previous highlight must go.
    hover_at(cx, handle, 52.);
    assert_eq!(
        row_fill_count(cx, handle),
        0,
        "moving onto a disabled row must clear the row the pointer selected, without selecting the disabled one"
    );

    press(cx, handle, "enter");
    assert!(
        log.borrow().is_empty(),
        "with the pointer on a disabled row, Enter must do nothing, got {:?}",
        *log.borrow()
    );
}

#[gpui::test]
fn a_disabled_row_does_not_clear_a_keyboard_selection(cx: &mut TestAppContext) {
    let (handle, log) = open(cx);
    set_disabled(cx, handle, vec![1]);
    press(cx, handle, "down");
    assert_eq!(row_fill_count(cx, handle), 1);

    hover_at(cx, handle, 52.);
    assert_eq!(
        row_fill_count(cx, handle),
        1,
        "the keyboard's row must survive the pointer resting on a disabled row"
    );
    press(cx, handle, "enter");
    assert_eq!(*log.borrow(), vec!["One"]);
}
