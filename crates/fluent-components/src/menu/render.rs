//! Root render phases: normalize state, install keyboard/IME handlers, reveal.
use super::*;

pub(super) fn prepare_state(
    tree: &MenuTree,
    levels: &[LevelSnapshot],
    reveal: Option<u64>,
    window: &mut Window,
    cx: &mut App,
) -> MenuState {
    // Read, reset a new session, normalize and persist the normalized state.
    // Every later reader in this frame — and every event handler — reads the
    // real path, not a corrected copy that only exists in render.
    let mut state = tree.read(cx).clone();
    let original = state.clone();
    if let Some(session) = reveal
        && state.session != Some(session)
    {
        state = MenuState {
            session: Some(session),
            ..MenuState::default()
        };
    }
    normalize_state(&mut state, levels);

    // A hover that has waited out the delay opens its submenu now. This runs
    // during render rather than in a timer: the deadline lives in state, and
    // any other hover replaces or clears it, so a late deadline cannot open a
    // branch the pointer has already left.
    if let Some(pending) = state.pending.clone() {
        let now = cx.background_executor().now();
        if now >= pending.deadline {
            state.pending = None;
            let opener = level_by_path(levels, &pending.path)
                .and_then(|level| level.row(&pending.row))
                .filter(|row| !row.disabled && row.has_submenu)
                .map(|row| (row.id.clone(), row.first_child.clone()));
            if let Some((row, first)) = opener
                && pending.path.len() == state.open.len()
            {
                // The pointer asked for this level, so its provisional row is
                // pointer-owned: leaving clears it, while a level opened with
                // the keyboard keeps its selection.
                open_below(&mut state, &pending.path, &row, first, true);
                normalize_state(&mut state, levels);
            }
        } else {
            window.request_animation_frame();
        }
    }
    if state != original {
        tree.update(cx, |slot, _| *slot = state.clone());
    }
    state
}

pub(super) fn input_handlers(
    mut container: gpui::Stateful<gpui::Div>,
    tree: &MenuTree,
    levels: &Rc<Vec<LevelSnapshot>>,
    typeahead: &gpui::Entity<Typeahead>,
    ime_sink: &gpui::Entity<ime::MenuImeSink>,
) -> gpui::Stateful<gpui::Div> {
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuNext, _, cx| navigate(&tree, &levels, cx, NavStep::Next)
    });
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuPrev, _, cx| navigate(&tree, &levels, cx, NavStep::Prev)
    });
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuFirst, _, cx| navigate(&tree, &levels, cx, NavStep::First)
    });
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuLast, _, cx| navigate(&tree, &levels, cx, NavStep::Last)
    });

    // Enter and Space run the current row, or open its submenu; Right never
    // runs a leaf, it only opens a level that actually exists.
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        let ime_sink = ime_sink.clone();
        move |_: &crate::MenuActivate, window, cx| {
            // Enter during an active composition commits text; it must not
            // also run the menu's current action.
            if ime_sink.read(cx).has_marked() {
                return;
            }
            activate_row(&tree, &levels, window, cx)
        }
    });
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuSubmenuOpen, _, cx| open_current_row(&tree, &levels, cx)
    });

    container = container.on_key_down({
        let tree = tree.clone();
        let levels = levels.clone();
        let typeahead = typeahead.clone();
        let ime_sink = ime_sink.clone();
        move |event, window, cx| {
            let Some(character) = typed_character(&event.keystroke) else {
                return;
            };
            let text = character.to_string();
            // On platforms that follow KeyDown with insertText, remember
            // this text so the commit does not apply it a second time. The
            // marker lasts one dispatch only: a key-only path must not
            // swallow a later independent commit of the same character.
            ime_sink.update(cx, |sink, _| sink.note_key(&text));
            let sink_for_next_frame = ime_sink.clone();
            let text_for_next_frame = text.clone();
            window.on_next_frame(move |_, cx| {
                sink_for_next_frame.update(cx, |sink, _| sink.clear_key(&text_for_next_frame));
            });
            apply_typeahead(&tree, &levels, &typeahead, &text, cx);
        }
    });

    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        move |_: &crate::MenuSubmenuClose, _, cx| {
            let (mut state, mut changed) = read_normalized(&tree, cx, &levels);
            changed |= state.pending.take().is_some();
            changed |= close_level(&mut state);
            commit(&tree, cx, &state, changed);
        }
    });

    // Escape closes one level; at the root it closes the whole menu. Tab is
    // separate: it closes the entire chain whatever level it is pressed on.
    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        let typeahead = typeahead.clone();
        move |_: &crate::MenuEscape, window, cx| {
            let (mut state, mut changed) = read_normalized(&tree, cx, &levels);
            changed |= state.pending.take().is_some();
            // Esc ends the typing session even at the root, where the open
            // path does not change and the epoch alone would not catch it.
            reset_typeahead(&typeahead, cx);
            if state.open.is_empty() {
                commit(&tree, cx, &state, changed);
                window.dispatch_action(Box::new(crate::Dismiss), cx);
            } else {
                changed |= close_level(&mut state);
                commit(&tree, cx, &state, changed);
            }
        }
    });

    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        let typeahead = typeahead.clone();
        move |_: &crate::MenuTabOut, window, cx| {
            tab_out(&tree, &levels, &typeahead, true, window, cx);
        }
    });

    container = container.on_action({
        let tree = tree.clone();
        let levels = levels.clone();
        let typeahead = typeahead.clone();
        move |_: &crate::MenuTabBackward, window, cx| {
            tab_out(&tree, &levels, &typeahead, false, window, cx);
        }
    });
    container
}

pub(super) fn reveal(
    container: gpui::Div,
    reveal: Option<u64>,
    menu_id: &SharedString,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // Reveal: the panel appears at its final size and fades in. Measuring the
    // Fluent docs menu shows width, edges and top staying constant while only
    // the opacity ramps, so this is a fade and not a grow.
    //
    // Only a menu that a dropdown opens reveals. A menu sitting in a page is
    // already there; fading it in on mount would flash every time its host
    // rebuilt the tree.
    let Some(generation) = reveal else {
        return container.into_any_element();
    };
    if cx.reduce_motion() {
        return container.into_any_element();
    }

    let reveal_state = window.use_keyed_state(
        SharedString::from(format!("menu-reveal-{}", menu_id)),
        cx,
        |_, _| (u64::MAX, None::<std::time::Instant>),
    );
    let now = cx.background_executor().now();
    let (stored_generation, stored_start) = *reveal_state.read(cx);
    let started = match stored_start {
        Some(start) if stored_generation == generation => start,
        _ => {
            reveal_state.update(cx, |state, _| {
                state.0 = generation;
                state.1 = Some(now);
            });
            now
        }
    };
    let elapsed = now.saturating_duration_since(started).as_secs_f32();
    let duration = fluent_core::spec::motion::DURATION_MENU_REVEAL.as_secs_f32();
    let progress = (elapsed / duration).clamp(0.0, 1.0);
    if progress >= 1.0 {
        return container.into_any_element();
    }
    window.request_animation_frame();
    let opacity = fluent_core::spec::motion::menu_reveal_opacity(progress);
    // Measured from the same recording: the content also settles downward by a
    // few pixels, so it starts slightly closer to the trigger and drops into
    // place while the opacity ramps.
    let rise = fluent_core::spec::motion::menu_reveal_offset(progress);
    container
        .relative()
        .top(px(rise))
        .opacity(opacity)
        .into_any_element()
}
