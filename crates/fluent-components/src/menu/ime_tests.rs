//! Real `EntityInputHandler` path for menu typeahead.
//!
//! Composition must not move the menu; only the committed replacement text
//! reaches the shared matcher. A printable KeyDown followed by its insertText
//! commit must not apply the same text twice.

use super::ime::MenuImeSink;
use gpui::{
    AppContext, Context, Entity, EntityInputHandler, IntoElement, Render, TestAppContext, Window,
    WindowHandle, div, px, size,
};
use std::cell::RefCell;
use std::rc::Rc;

struct EmptyView;

impl Render for EmptyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn open(cx: &mut TestAppContext) -> (WindowHandle<EmptyView>, Entity<MenuImeSink>) {
    let sink_slot: Rc<RefCell<Option<Entity<MenuImeSink>>>> = Default::default();
    let slot = sink_slot.clone();
    let window = cx.open_window(size(px(200.), px(200.)), move |_, cx| {
        let sink = cx.new(|_| MenuImeSink::default());
        slot.replace(Some(sink));
        EmptyView
    });
    cx.run_until_parked();
    let sink = sink_slot.borrow().clone().expect("sink built");
    (window, sink)
}

#[gpui::test]
fn composition_waits_for_commit_and_committed_text_runs_once(cx: &mut TestAppContext) {
    let (window, sink) = open(cx);
    let log: Rc<RefCell<Vec<String>>> = Default::default();
    let log_for_handler = log.clone();
    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            sink.set_handler(
                Rc::new(move |text, _, _| log_for_handler.borrow_mut().push(text.to_string())),
                (Some(1), 0),
            )
        });
    });

    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, cx| {
            sink.replace_and_mark_text_in_range(None, "shan", None, window, cx);
            assert!(log.borrow().is_empty(), "composition must not match");
            assert!(sink.has_marked(), "composition is marked");

            sink.replace_text_in_range(None, "删", window, cx);
            assert_eq!(*log.borrow(), vec!["删"], "commit runs the matcher once");
            assert!(!sink.has_marked(), "commit ends composition");
        });
    })
    .unwrap();
}

#[gpui::test]
fn a_keydown_followed_by_its_commit_does_not_double_apply(cx: &mut TestAppContext) {
    let (window, sink) = open(cx);
    let log: Rc<RefCell<Vec<String>>> = Default::default();
    let log_for_handler = log.clone();
    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            sink.set_handler(
                Rc::new(move |text, _, _| log_for_handler.borrow_mut().push(text.to_string())),
                (Some(1), 0),
            )
        });
    });

    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, cx| {
            sink.note_key("a");
            sink.replace_text_in_range(None, "a", window, cx);
            assert!(
                log.borrow().is_empty(),
                "the commit is the KeyDown duplicate"
            );

            sink.replace_text_in_range(None, "删", window, cx);
            assert_eq!(*log.borrow(), vec!["删"]);
        });
    })
    .unwrap();
}

#[gpui::test]
fn two_independent_commits_of_the_same_character_both_apply(cx: &mut TestAppContext) {
    let (window, sink) = open(cx);
    let log: Rc<RefCell<Vec<String>>> = Default::default();
    let log_for_handler = log.clone();
    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            sink.set_handler(
                Rc::new(move |text, _, _| log_for_handler.borrow_mut().push(text.to_string())),
                (Some(1), 0),
            )
        });
    });

    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, cx| {
            sink.replace_text_in_range(None, "删", window, cx);
            sink.replace_text_in_range(None, "删", window, cx);
            assert_eq!(*log.borrow(), vec!["删", "删"]);
        });
    })
    .unwrap();
}

#[gpui::test]
fn a_key_only_path_does_not_swallow_a_later_commit(cx: &mut TestAppContext) {
    let (window, sink) = open(cx);
    let log: Rc<RefCell<Vec<String>>> = Default::default();
    let log_for_handler = log.clone();
    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            sink.set_handler(
                Rc::new(move |text, _, _| log_for_handler.borrow_mut().push(text.to_string())),
                (Some(1), 0),
            )
        });
    });

    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, cx| {
            sink.note_key("a");
            // No commit followed; the one-dispatch marker is cleared.
            sink.clear_key("a");
            sink.replace_text_in_range(None, "a", window, cx);
            assert_eq!(*log.borrow(), vec!["a"]);
        });
    })
    .unwrap();
}

#[gpui::test]
fn a_new_menu_generation_drops_stale_composition(cx: &mut TestAppContext) {
    let (window, sink) = open(cx);
    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            sink.set_handler(Rc::new(|_, _, _| {}), (Some(1), 0));
        });
    });

    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, cx| {
            sink.replace_and_mark_text_in_range(None, "shan", None, window, cx);
            assert!(sink.has_marked());
        });
    })
    .unwrap();

    cx.update(|cx| {
        sink.update(cx, |sink, _| {
            // Reopen / child-menu switch: the session+epoch differs.
            sink.set_handler(Rc::new(|_, _, _| {}), (Some(2), 0));
        });
    });
    cx.update_window(window.into(), |_, window, cx| {
        sink.update(cx, |sink, _| {
            assert!(
                !sink.has_marked(),
                "a new visit must not inherit a composition"
            );
        });
        let _ = (window, cx);
    })
    .unwrap();
}
