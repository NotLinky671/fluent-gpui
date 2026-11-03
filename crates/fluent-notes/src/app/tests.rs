//! GPUI behaviour tests for the notes app.
//!
//! These tests drive the same view entity the real window uses: real input
//! states, the real popover/menu composition and real key dispatch. They do not
//! prove GPU rasterisation, native IME composition or pointer hit-testing in a
//! platform window; those stay on the manual checklist.

use super::*;
use std::cell::Cell;
use std::rc::Rc;

use fluent_ui::gpui::{
    Bounds, Entity, FocusHandle, KeyDownEvent, Keystroke, Modifiers, Pixels, PlatformInput,
    TestAppContext, VisualTestContext, WindowHandle, canvas, point, px, size,
};

type Handle = WindowHandle<NotesApp>;

fn open(cx: &mut TestAppContext) -> (Handle, Entity<NotesApp>) {
    cx.update(fluent_ui::init);
    let window = cx.open_window(size(px(1100.), px(760.)), |window, cx| {
        NotesApp::new(window, cx)
    });
    cx.run_until_parked();
    let app = cx
        .update_window(window.into(), |root, _, _| {
            root.downcast::<NotesApp>()
                .expect("the notes view is the root")
        })
        .unwrap();
    (window, app)
}

/// Draw twice so tab stops registered during paint are visible to focus
/// navigation, mirroring the component tests.
fn settle(cx: &mut TestAppContext, window: Handle) {
    for _ in 0..2 {
        cx.update_window(window.into(), |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        })
        .unwrap();
        cx.run_until_parked();
    }
}

fn focus_title(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) {
    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.focus_title(window, cx));
    })
    .unwrap();
    settle(cx, window);
}

fn focus_body(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) {
    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| {
            app.body_input
                .update(cx, |state, cx| state.focus(window, cx));
        });
    })
    .unwrap();
    settle(cx, window);
}

fn title_focused(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) -> bool {
    cx.update_window(window.into(), |_, window, cx| {
        let input = app.read(cx).title_input.clone();
        input
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window)
    })
    .unwrap()
}

fn body_focused(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) -> bool {
    cx.update_window(window.into(), |_, window, cx| {
        let input = app.read(cx).body_input.clone();
        input
            .read(cx)
            .presentation()
            .focus_handle()
            .is_focused(window)
    })
    .unwrap()
}

/// The handle that currently owns keyboard focus. Comparing handles across
/// presses proves focus actually moved instead of merely staying in a subtree.
fn focused_handle(cx: &mut TestAppContext, window: Handle) -> Option<FocusHandle> {
    cx.update_window(window.into(), |_, window, cx| window.focused(cx))
        .unwrap()
}

fn settings_contains_focus(
    cx: &mut TestAppContext,
    window: Handle,
    app: &Entity<NotesApp>,
) -> bool {
    let panel = cx.update(|cx| app.read(cx).settings_focus.clone());
    cx.update_window(window.into(), |_, window, cx| {
        panel.contains_focused(window, cx)
    })
    .unwrap()
}

fn menu_contains_focus(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) -> bool {
    let panel = cx.update(|cx| app.read(cx).menu_focus.clone());
    cx.update_window(window.into(), |_, window, cx| {
        panel.contains_focused(window, cx)
    })
    .unwrap()
}

fn values(cx: &TestAppContext, app: &Entity<NotesApp>) -> (String, String) {
    cx.read(|cx| app.read(cx).draft(cx))
}

fn type_text(cx: &mut TestAppContext, window: Handle, text: &str) {
    cx.simulate_input(window.into(), text);
    cx.run_until_parked();
}

fn press(cx: &mut TestAppContext, window: Handle, keys: &str) {
    cx.simulate_keystrokes(window.into(), keys);
    cx.run_until_parked();
}

fn save(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>) {
    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.save(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
}

fn select(cx: &mut TestAppContext, window: Handle, app: &Entity<NotesApp>, id: NoteId) {
    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.request_select(window, cx, id));
    })
    .unwrap();
    cx.run_until_parked();
}

fn set_editor_text(
    cx: &mut TestAppContext,
    window: Handle,
    app: &Entity<NotesApp>,
    title: &str,
    body: &str,
) {
    let title = title.to_string();
    let body = body.to_string();
    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| {
            app.title_input
                .update(cx, |state, cx| state.set_value(title, window, cx));
            app.body_input
                .update(cx, |state, cx| state.set_value(body, window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
}

fn note_snapshot(cx: &mut TestAppContext, app: &Entity<NotesApp>, id: NoteId) -> Option<Note> {
    cx.update(|cx| app.read(cx).model.get(id).cloned())
}

fn selected(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> Option<NoteId> {
    cx.update(|cx| app.read(cx).selected)
}

fn pending(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> Option<PendingAction> {
    cx.update(|cx| app.read(cx).pending)
}

fn status_text(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> String {
    cx.update(|cx| app.read(cx).status.text.to_string())
}

fn is_menu_open(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> bool {
    cx.update(|cx| app.read(cx).menu_open)
}

fn is_settings_open(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> bool {
    cx.update(|cx| app.read(cx).settings_open)
}

fn is_dirty(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> bool {
    cx.update(|cx| {
        let app = app.read(cx);
        app.is_dirty(cx)
    })
}

fn range(cx: &mut TestAppContext, app: &Entity<NotesApp>) -> std::ops::Range<usize> {
    cx.update(|cx| app.read(cx).title_input.read(cx).selected_range())
}

#[gpui::test]
fn typing_saving_switching_and_returning_keeps_both_fields(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "会议 Meeting notes");
    focus_body(cx, window, &app);
    type_text(cx, window, "第一段 English text");
    save(cx, window, &app);

    let model = note_snapshot(cx, &app, NoteId(1)).expect("note 1 exists");
    assert_eq!(model.title, "会议 Meeting notes");
    assert_eq!(model.body, "第一段 English text");
    assert!(
        status_text(cx, &app).contains("已保存"),
        "the save must be reported in the status line"
    );

    let other_body = note_snapshot(cx, &app, NoteId(2)).unwrap().body;
    select(cx, window, &app, NoteId(2));
    assert_eq!(
        values(cx, &app),
        ("会议记录 Meeting notes".to_string(), other_body)
    );

    select(cx, window, &app, NoteId(1));
    assert_eq!(
        values(cx, &app),
        (
            "会议 Meeting notes".to_string(),
            "第一段 English text".to_string()
        ),
        "returning to a note must show what was saved earlier"
    );
}

#[gpui::test]
fn cancel_restores_the_last_saved_content(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);
    type_text(cx, window, "临时修改");
    assert!(is_dirty(cx, &app));

    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.cancel(window, cx));
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(
        values(cx, &app).0,
        "欢迎使用便签",
        "cancel must restore the saved title"
    );
    assert!(!is_dirty(cx, &app), "cancel clears the dirty state");
    assert!(
        status_text(cx, &app).contains("已还原"),
        "cancel has visible feedback"
    );
}

#[gpui::test]
fn switching_with_an_unsaved_draft_requires_an_explicit_choice(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "没有保存的标题");

    select(cx, window, &app, NoteId(2));
    assert_eq!(
        pending(cx, &app),
        Some(PendingAction::SwitchTo(NoteId(2))),
        "a dirty draft must not be discarded silently"
    );
    assert_eq!(
        values(cx, &app).0,
        "没有保存的标题",
        "the draft stays in the editor while the confirmation is shown"
    );

    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.save_and_continue(window, cx));
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(selected(cx, &app), Some(NoteId(2)));
    assert_eq!(
        note_snapshot(cx, &app, NoteId(1)).unwrap().title,
        "没有保存的标题",
        "save-and-switch must persist the draft before moving"
    );
    assert_eq!(pending(cx, &app), None);
}

#[gpui::test]
fn creating_a_note_with_an_unsaved_draft_requires_confirmation(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "new-note draft");

    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.request_new_note(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(
        pending(cx, &app),
        Some(PendingAction::NewNote),
        "creating a note must not discard the current draft silently"
    );
    assert_eq!(
        cx.update(|cx| app.read(cx).model.len()),
        4,
        "no note is created before the user chooses"
    );

    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.save_and_continue(window, cx));
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(cx.update(|cx| app.read(cx).model.len()), 5);
    assert_eq!(
        note_snapshot(cx, &app, NoteId(1)).unwrap().title,
        "new-note draft",
        "Save and create persists the draft first"
    );
    assert_eq!(values(cx, &app), (String::new(), String::new()));
}

#[gpui::test]
fn discarding_a_draft_keeps_the_original_note(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);
    type_text(cx, window, "不要保存");
    select(cx, window, &app, NoteId(2));
    assert!(pending(cx, &app).is_some());

    cx.update_window(window.into(), |_, window, cx| {
        app.update(cx, |app, cx| app.confirm_pending(window, cx));
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(selected(cx, &app), Some(NoteId(2)));
    assert_eq!(
        note_snapshot(cx, &app, NoteId(1)).unwrap().title,
        "欢迎使用便签",
        "discarding must not write the draft"
    );
}

#[gpui::test]
fn theme_switch_and_overlays_preserve_draft_caret_and_selection(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "ABCDE");

    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.title_input
                .update(cx, |state, cx| state.set_selected_range(1..3, cx));
            app.apply_appearance(Appearance::Dark, cx);
        });
    })
    .unwrap();
    settle(cx, window);

    assert_eq!(
        values(cx, &app).0,
        "ABCDE",
        "the draft survives a theme switch"
    );
    assert_eq!(
        range(cx, &app),
        1..3,
        "the selection survives a theme switch"
    );
    assert!(title_focused(cx, window, &app));

    // Opening the settings popover moves focus into the panel; Escape must
    // close it and give the title field its focus back without touching text.
    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.settings_open = true;
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);
    let panel_focused = cx
        .update_window(window.into(), |_, window, cx| {
            app.read(cx).settings_focus.contains_focused(window, cx)
        })
        .unwrap();
    assert!(
        panel_focused,
        "opening the popover hands focus to the panel"
    );
    assert!(!title_focused(cx, window, &app));

    press(cx, window, "escape");
    assert!(!is_settings_open(cx, &app), "Escape closes the popover");
    settle(cx, window);
    assert!(
        title_focused(cx, window, &app),
        "closing the popover returns focus to the element that opened it"
    );
    assert_eq!(values(cx, &app).0, "ABCDE");
    assert_eq!(range(cx, &app), 1..3);
}

#[gpui::test]
fn settings_popover_wraps_tab_between_controls(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);
    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.settings_open = true;
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);

    let panel = cx.update(|cx| app.read(cx).settings_focus.clone());
    assert!(
        settings_contains_focus(cx, window, &app),
        "opening the settings popover moves focus into the panel"
    );
    assert_eq!(
        focused_handle(cx, window),
        Some(panel.clone()),
        "the panel container is the initial focus target"
    );
    assert!(
        !title_focused(cx, window, &app) && !body_focused(cx, window, &app),
        "opening the panel takes focus out of the page behind it"
    );

    // The panel itself is not a tab stop. Tab from the container must therefore
    // land on a real control, not stay on the container.
    press(cx, window, "tab");
    let first = focused_handle(cx, window).expect("the first settings control must take focus");
    assert_ne!(
        Some(first.clone()),
        Some(panel.clone()),
        "Tab must move from the panel container to a control"
    );
    assert!(settings_contains_focus(cx, window, &app));
    assert!(!title_focused(cx, window, &app) && !body_focused(cx, window, &app));

    // Walk forward until the first control repeats. Every step must move to a
    // different handle and stay inside the panel; the final step is last -> first.
    let mut cycle = vec![first.clone()];
    loop {
        press(cx, window, "tab");
        let next = focused_handle(cx, window).expect("a settings control must keep focus");
        assert!(
            settings_contains_focus(cx, window, &app),
            "Tab escaped the open settings panel"
        );
        assert!(
            !title_focused(cx, window, &app) && !body_focused(cx, window, &app),
            "Tab reached the page behind the panel"
        );
        assert_ne!(
            &next,
            cycle.last().expect("cycle is non-empty"),
            "each Tab press must move focus to a different control"
        );
        if next == first {
            break;
        }
        cycle.push(next);
        assert!(
            cycle.len() < 32,
            "the settings panel should reach a short tab cycle"
        );
    }
    assert!(
        cycle.len() >= 2,
        "the settings panel must expose more than one control"
    );
    assert!(
        cycle.iter().all(|handle| handle != &panel),
        "the panel container must not be one of the control tab stops"
    );
    let last = cycle.last().expect("cycle is non-empty").clone();
    assert_eq!(
        focused_handle(cx, window),
        Some(first.clone()),
        "the last control wraps forward to the first"
    );

    press(cx, window, "shift-tab");
    assert_eq!(
        focused_handle(cx, window),
        Some(last.clone()),
        "Shift+Tab from the first control wraps backward to the last"
    );
    press(cx, window, "tab");
    assert_eq!(
        focused_handle(cx, window),
        Some(first.clone()),
        "Tab from the last control wraps forward to the first"
    );

    // More than one full cycle: the page behind the panel must never be reached.
    for step in 0..(cycle.len() * 2) {
        press(cx, window, "tab");
        assert!(
            focused_handle(cx, window).is_some(),
            "focus was lost on step {step}"
        );
        assert!(
            settings_contains_focus(cx, window, &app),
            "focus escaped the panel on step {step}"
        );
        assert!(
            !title_focused(cx, window, &app) && !body_focused(cx, window, &app),
            "focus reached a background input on step {step}"
        );
    }
}

#[gpui::test]
fn outside_click_closes_the_settings_popover(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);
    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.settings_open = true;
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);

    let mut visual = VisualTestContext::from_window(window.into(), cx);
    visual.simulate_click(point(px(5.), px(5.)), Modifiers::default());
    visual.run_until_parked();
    assert!(
        !visual.read(|cx| app.read(cx).settings_open),
        "a press outside the panel must request a close"
    );
}

#[gpui::test]
fn menu_keyboard_toggles_pin_and_closes(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    assert!(cx.update(|cx| app.read(cx).current().unwrap().pinned));

    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.menu_open = true;
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);

    // Row order: Save (selectable), Cancel (disabled while clean), Pin. Two
    // Downs reach Pin; Enter must activate the row that looks current.
    press(cx, window, "down down enter");
    assert!(
        !cx.update(|cx| app.read(cx).current().unwrap().pinned),
        "Enter on the Pin row must toggle the note's pinned flag"
    );
    assert!(!is_menu_open(cx, &app), "choosing a row closes the menu");
}

#[gpui::test]
fn menu_cancel_restores_the_draft_closes_and_returns_focus(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    let id = selected(cx, &app).unwrap();
    let saved = note_snapshot(cx, &app, id).unwrap();
    set_editor_text(cx, window, &app, "草稿标题", "草稿正文");
    focus_title(cx, window, &app);

    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.menu_open = true;
            cx.notify();
        });
    })
    .unwrap();
    settle(cx, window);
    assert!(is_menu_open(cx, &app));

    // Dirty menu rows: Save, Cancel, Pin, ... Two Downs reach Cancel. Enter
    // must restore the saved content, close the menu and return focus to the
    // element that owned it before the menu opened.
    press(cx, window, "down down enter");
    settle(cx, window);

    assert!(
        !is_menu_open(cx, &app),
        "Cancel must close the menu; MenuDropdown does not auto-close"
    );
    assert_eq!(
        values(cx, &app),
        (saved.title, saved.body),
        "Cancel restores the saved title and body"
    );
    assert!(
        title_focused(cx, window, &app),
        "closing the menu returns focus to the pre-open focus target"
    );
    assert!(
        !menu_contains_focus(cx, window, &app),
        "focus must leave the menu panel when it closes"
    );
}

#[gpui::test]
fn tab_moves_focus_forward_and_back(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);
    assert!(title_focused(cx, window, &app));

    press(cx, window, "tab");
    assert!(
        !title_focused(cx, window, &app),
        "Tab must leave the title field"
    );

    press(cx, window, "shift-tab");
    assert!(
        title_focused(cx, window, &app),
        "Shift+Tab must return to the previous stop"
    );
}

#[gpui::test]
fn deleting_every_note_leaves_an_empty_selection_and_editor(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    // Delete the current note repeatedly; each confirmation picks the next
    // neighbour until the store is empty.
    for _ in 0..4 {
        cx.update_window(window.into(), |_, window, cx| {
            app.update(cx, |app, cx| {
                app.request_delete(cx);
                app.confirm_pending(window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
    }

    assert!(cx.update(|cx| app.read(cx).model.is_empty()));
    assert_eq!(selected(cx, &app), None);
    assert_eq!(
        cx.update(|cx| app.read(cx).editor_note),
        None,
        "the editor must not keep pointing at a deleted note"
    );
    assert_eq!(values(cx, &app), (String::new(), String::new()));
    assert!(
        status_text(cx, &app).contains("已删除"),
        "each delete reports its result"
    );
}

#[gpui::test]
fn empty_title_save_reports_the_placeholder_name(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "只有正文");
    save(cx, window, &app);

    let note = note_snapshot(cx, &app, NoteId(1)).unwrap();
    assert_eq!(note.title, "", "the editor value is kept as-is");
    assert_eq!(note.display_title(), "无标题便签");
    let status = status_text(cx, &app);
    assert!(status.contains("无标题便签"), "status: {status}");
}

#[gpui::test]
fn pinned_and_tone_actions_reach_the_model_from_the_popover(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| {
            app.set_pinned(false, cx);
            app.set_tone(NoteTone::Warning, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    assert!(!note_snapshot(cx, &app, NoteId(1)).unwrap().pinned);
    assert_eq!(
        note_snapshot(cx, &app, NoteId(1)).unwrap().tone,
        NoteTone::Warning
    );
    assert!(!is_menu_open(cx, &app));
}

#[gpui::test]
fn dispatching_a_movement_key_keeps_the_input_state_entity(cx: &mut TestAppContext) {
    // Guards against "recreate the input each render": the entity identity must
    // stay stable while the app re-renders around it.
    let (window, app) = open(cx);
    let before = cx.update(|cx| app.read(cx).title_input.entity_id());
    focus_title(cx, window, &app);
    type_text(cx, window, "abc");
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("home").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();

    let after = cx.update(|cx| app.read(cx).title_input.entity_id());
    assert_eq!(before, after);
    assert_eq!(
        cx.update(|cx| app.read(cx).title_input.read(cx).cursor()),
        0,
        "Home moves the caret to the start of the single-line field"
    );
}

#[gpui::test]
fn tab_from_title_reaches_a_control_and_back(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    focus_title(cx, window, &app);

    press(cx, window, "tab");
    let on_body = body_focused(cx, window, &app);
    assert!(
        !title_focused(cx, window, &app),
        "Tab leaves the title field (next stop is a toolbar button or the body)"
    );
    assert!(
        !on_body,
        "sanity: this layout puts the toolbar before the body in tab order"
    );

    press(cx, window, "shift-tab");
    assert!(
        title_focused(cx, window, &app),
        "Shift+Tab returns to the title field"
    );
}

// ---------------------------------------------------------------------------
// Probe: a controlled MenuDropdown opened by a pointer click on its trigger.
// This reproduces the real-window `...` button path at different scale factors.
// ---------------------------------------------------------------------------

struct MenuTriggerProbe {
    open: bool,
    focus: FocusHandle,
    trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    panel_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Render for MenuTriggerProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let trigger_bounds = self.trigger_bounds.clone();
        let panel_bounds = self.panel_bounds.clone();
        let trigger = div()
            .id("probe-trigger")
            .w(px(120.))
            .h(px(32.))
            .bg(fluent_ui::gpui::rgb(0x336699))
            .on_click(cx.listener(|this, _, _, cx| {
                this.open = true;
                cx.notify();
            }))
            .child(
                canvas(
                    move |bounds, _, _| trigger_bounds.set(Some(bounds)),
                    |_, _, _, _| {},
                )
                .size_full(),
            );
        let weak = cx.entity().downgrade();
        let panel = Menu::new("probe-menu-panel")
            .item(MenuItem::new("probe-item", "Item"))
            .child(
                canvas(
                    move |bounds, _, _| panel_bounds.set(Some(bounds)),
                    |_, _, _, _| {},
                )
                .w(px(80.))
                .h(px(20.)),
            );
        div().size_full().p(px(40.)).child(
            MenuDropdown::new("probe-menu", trigger, panel)
                .open(self.open)
                .track_focus(&self.focus)
                .on_open_change(move |open, _, cx| {
                    weak.update(cx, |this, cx| {
                        this.open = open;
                        cx.notify();
                    })
                    .ok();
                }),
        )
    }
}

fn menu_probe(
    cx: &mut TestAppContext,
    scale: f32,
) -> (WindowHandle<MenuTriggerProbe>, Entity<MenuTriggerProbe>) {
    cx.update(fluent_ui::init);
    let window = cx.open_window(size(px(400.), px(300.)), |_window, cx| MenuTriggerProbe {
        open: false,
        focus: cx.focus_handle().tab_stop(true),
        trigger_bounds: Rc::new(Cell::new(None)),
        panel_bounds: Rc::new(Cell::new(None)),
    });
    cx.run_until_parked();
    let entity = cx
        .update_window(window.into(), |root, _, _| {
            root.downcast::<MenuTriggerProbe>().unwrap()
        })
        .unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        window.set_scale_factor(scale);
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
    (window, entity)
}

#[gpui::test]
fn menu_dropdown_trigger_click_opens_the_panel(cx: &mut TestAppContext) {
    for scale in [1.0, 1.5] {
        let (window, entity) = menu_probe(cx, scale);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        let bounds = visual.read(|cx| entity.read(cx).trigger_bounds.get());
        let bounds = bounds.expect("trigger measured");
        let centre = bounds.center();
        visual.simulate_click(centre, Modifiers::default());
        visual.run_until_parked();
        visual
            .update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .unwrap();
        visual.run_until_parked();
        let open = visual.read(|cx| entity.read(cx).open);
        assert!(
            open,
            "click at {centre:?} must open the menu at scale {scale}"
        );
        let panel = visual.read(|cx| entity.read(cx).panel_bounds.get());
        assert!(
            panel.is_some(),
            "the menu panel must be drawn at scale {scale}"
        );
        let focused = visual
            .update_window(window.into(), |_, window, cx| {
                entity.read(cx).focus.contains_focused(window, cx)
            })
            .unwrap();
        assert!(
            focused,
            "opening must move focus into the menu at scale {scale}"
        );
    }
}

// ---------------------------------------------------------------------------
// Pending decisions now arrive as real dialogs. These tests check the modal
// paths the in-page bar could not offer: Escape/scrim cancellation for the
// switch dialog, mandatory choice for the delete alert, and focus return.
// ---------------------------------------------------------------------------

fn press_escape(cx: &mut TestAppContext, window: Handle) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: Keystroke::parse("escape").unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn click_scrim(cx: &mut TestAppContext, window: Handle) {
    let mut visual = VisualTestContext::from_window(window.into(), cx);
    // The dialog card is centred; a corner is always scrim.
    visual.simulate_click(point(px(8.), px(8.)), Modifiers::default());
    visual.run_until_parked();
}

#[gpui::test]
fn escape_cancels_the_switch_dialog_and_keeps_the_draft(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "unsaved draft");
    select(cx, window, &app, NoteId(2));
    assert_eq!(pending(cx, &app), Some(PendingAction::SwitchTo(NoteId(2))));
    settle(cx, window);

    press_escape(cx, window);
    assert_eq!(pending(cx, &app), None, "Escape must dismiss the dialog");
    assert_eq!(selected(cx, &app), Some(NoteId(1)), "no switch happened");
    assert_eq!(values(cx, &app).0, "unsaved draft", "the draft stays");
}

#[gpui::test]
fn a_scrim_press_cancels_the_switch_dialog(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "unsaved draft");
    select(cx, window, &app, NoteId(2));
    assert!(pending(cx, &app).is_some());
    settle(cx, window);

    click_scrim(cx, window);
    assert_eq!(
        pending(cx, &app),
        None,
        "a scrim press must dismiss the switch dialog"
    );
    assert_eq!(selected(cx, &app), Some(NoteId(1)));
    assert_eq!(values(cx, &app).0, "unsaved draft");
}

#[gpui::test]
fn escape_does_not_dismiss_the_delete_alert(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    cx.update_window(window.into(), |_, _, cx| {
        app.update(cx, |app, cx| app.request_delete(cx));
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(pending(cx, &app), Some(PendingAction::Delete(NoteId(1))));
    settle(cx, window);

    press_escape(cx, window);
    assert_eq!(
        pending(cx, &app),
        Some(PendingAction::Delete(NoteId(1))),
        "an alert dialog requires an explicit choice"
    );
    click_scrim(cx, window);
    assert_eq!(
        pending(cx, &app),
        Some(PendingAction::Delete(NoteId(1))),
        "a scrim press must not dismiss the alert either"
    );
    assert!(note_snapshot(cx, &app, NoteId(1)).is_some());
}

#[gpui::test]
fn the_pending_dialog_traps_and_returns_focus(cx: &mut TestAppContext) {
    let (window, app) = open(cx);
    set_editor_text(cx, window, &app, "", "");
    focus_title(cx, window, &app);
    type_text(cx, window, "unsaved draft");
    select(cx, window, &app, NoteId(2));
    settle(cx, window);

    assert!(
        !title_focused(cx, window, &app),
        "opening the dialog moves focus into it"
    );

    // Tab cycles inside the dialog and never returns to the title field.
    for _ in 0..4 {
        press(cx, window, "tab");
        assert!(
            !title_focused(cx, window, &app),
            "Tab must not reach the page behind the dialog"
        );
    }

    press_escape(cx, window);
    assert_eq!(pending(cx, &app), None);
    assert!(
        title_focused(cx, window, &app),
        "closing returns focus to the title field that had it"
    );
}
