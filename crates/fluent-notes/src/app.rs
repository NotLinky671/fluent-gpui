//! The notes view: state, event handlers and rendering.
//!
//! The view owns two long-lived input entities (title and body) so a redraw,
//! a theme switch or an overlay never rebuilds the editing state. `set_value` is
//! called only from explicit navigation actions (create, select, cancel, delete);
//! everything else leaves the inputs alone.

use fluent_ui::gpui::{
    AnyElement, App, Context, Entity, FocusHandle, FontWeight, Render, SharedString, Subscription,
    Window, relative,
};
use fluent_ui::gpui_base::input::{InputState, TextareaState};
use fluent_ui::prelude::*;
use fluent_ui::{
    AlertDialog, Appearance, Badge, BadgeAppearance, BadgeSize, Button, ButtonAppearance, Checkbox,
    ControlSize, Dialog, Divider, Field, FluentIcon, FluentRoot, FluentTheme, Icon, Label, Menu,
    MenuDropdown, MenuHeader, MenuItem, Popover, Radio, Switch, TextInput, Textarea,
};

use crate::model::{Note, NoteId, NoteTone, NotesModel};

const BODY_PREVIEW_CHARS: usize = 70;

/// A decision the user has to make before the app changes the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingAction {
    SwitchTo(NoteId),
    NewNote,
    Delete(NoteId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusTone {
    Info,
    Success,
    Warning,
}

#[derive(Clone, Debug)]
struct Status {
    tone: StatusTone,
    text: SharedString,
}

impl Status {
    fn info(text: impl Into<SharedString>) -> Self {
        Self {
            tone: StatusTone::Info,
            text: text.into(),
        }
    }

    fn success(text: impl Into<SharedString>) -> Self {
        Self {
            tone: StatusTone::Success,
            text: text.into(),
        }
    }

    fn warning(text: impl Into<SharedString>) -> Self {
        Self {
            tone: StatusTone::Warning,
            text: text.into(),
        }
    }
}

/// The application view.
pub struct NotesApp {
    model: NotesModel,
    selected: Option<NoteId>,
    /// The note whose saved content the input entities currently hold.
    editor_note: Option<NoteId>,
    title_input: Entity<InputState>,
    body_input: Entity<TextareaState>,
    pending: Option<PendingAction>,
    menu_open: bool,
    menu_focus: FocusHandle,
    settings_open: bool,
    settings_focus: FocusHandle,
    props_open: bool,
    props_focus: FocusHandle,
    appearance: Appearance,
    show_counts: bool,
    compact_list: bool,
    status: Status,
    _subscriptions: Vec<Subscription>,
}

impl NotesApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = NotesModel::seeded();
        let selected = model.first();
        let title_input = cx.new(|cx| InputState::new(window, cx).placeholder("便签标题"));
        let body_input = cx.new(|cx| TextareaState::new(window, cx).placeholder("在这里写正文…"));

        let mut app = Self {
            model,
            selected,
            editor_note: None,
            title_input,
            body_input,
            pending: None,
            menu_open: false,
            menu_focus: cx.focus_handle(),
            settings_open: false,
            settings_focus: cx.focus_handle(),
            props_open: false,
            props_focus: cx.focus_handle(),
            appearance: FluentTheme::global(cx).appearance(),
            show_counts: true,
            compact_list: false,
            status: Status::info("就绪。所有便签只保存在内存中，关闭窗口后不会保留。"),
            _subscriptions: Vec::new(),
        };

        if let Some(id) = selected {
            app.load_into_editor(window, cx, id);
        }

        // The list shows a live dirty mark and character counts. Input changes
        // notify the app so those labels follow the draft without the input
        // states having to be rebuilt.
        let title_sub = cx.observe(&app.title_input, |_, _, cx| cx.notify());
        let body_sub = cx.observe(&app.body_input, |_, _, cx| cx.notify());
        app._subscriptions = vec![title_sub, body_sub];
        app
    }

    fn current(&self) -> Option<&Note> {
        self.selected.and_then(|id| self.model.get(id))
    }

    fn draft(&self, cx: &App) -> (String, String) {
        (
            self.title_input.read(cx).value().to_string(),
            self.body_input.read(cx).value().to_string(),
        )
    }

    fn is_dirty(&self, cx: &App) -> bool {
        let Some(id) = self.editor_note else {
            return false;
        };
        let (title, body) = self.draft(cx);
        !self.model.matches(id, &title, &body)
    }

    fn load_into_editor(&mut self, window: &mut Window, cx: &mut Context<Self>, id: NoteId) {
        let Some(note) = self.model.get(id).cloned() else {
            return;
        };
        self.editor_note = Some(id);
        self.title_input
            .update(cx, |state, cx| state.set_value(note.title, window, cx));
        self.body_input
            .update(cx, |state, cx| state.set_value(note.body, window, cx));
    }

    fn clear_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_note = None;
        self.title_input
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.body_input
            .update(cx, |state, cx| state.set_value("", window, cx));
    }

    fn focus_title(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.title_input
            .update(cx, |state, cx| state.focus(window, cx));
    }

    fn close_overlays(&mut self) {
        self.menu_open = false;
        self.props_open = false;
    }

    // ---- actions -------------------------------------------------------

    fn toggle_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.close_overlays();
        }
        cx.notify();
    }

    fn toggle_props(&mut self, cx: &mut Context<Self>) {
        if self.current().is_none() {
            return;
        }
        self.props_open = !self.props_open;
        if self.props_open {
            self.settings_open = false;
            self.menu_open = false;
        }
        cx.notify();
    }

    fn toggle_menu(&mut self, cx: &mut Context<Self>) {
        if self.current().is_none() {
            return;
        }
        self.menu_open = !self.menu_open;
        if self.menu_open {
            self.settings_open = false;
            self.props_open = false;
        }
        cx.notify();
    }

    fn request_select(&mut self, window: &mut Window, cx: &mut Context<Self>, id: NoteId) {
        if self.selected == Some(id) {
            self.pending = None;
            cx.notify();
            return;
        }
        if self.is_dirty(cx) {
            self.pending = Some(PendingAction::SwitchTo(id));
            self.close_overlays();
            cx.notify();
            return;
        }
        self.activate(window, cx, id);
    }

    fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>, id: NoteId) {
        let Some(note) = self.model.get(id) else {
            return;
        };
        let label = note.display_title().to_string();
        self.selected = Some(id);
        self.load_into_editor(window, cx, id);
        self.pending = None;
        self.close_overlays();
        self.status = Status::info(format!("已打开：{label}"));
        self.focus_title(window, cx);
        cx.notify();
    }

    fn request_new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dirty(cx) {
            self.pending = Some(PendingAction::NewNote);
            self.close_overlays();
            cx.notify();
            return;
        }
        self.create_note(window, cx);
    }

    fn create_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.model.create();
        self.selected = Some(id);
        self.load_into_editor(window, cx, id);
        self.pending = None;
        self.close_overlays();
        self.status = Status::info("已新建便签。输入标题与正文后保存；关闭窗口后内容不会保留。");
        self.focus_title(window, cx);
        cx.notify();
    }

    /// Writes the editor into the selected note. Returns the saved display title.
    fn save_current(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let id = self.editor_note.or(self.selected)?;
        let (title, body) = self.draft(cx);
        if !self.model.save(id, title, body) {
            return None;
        }
        self.model
            .get(id)
            .map(|note| note.display_title().to_string())
    }

    fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(display) = self.save_current(cx) else {
            self.status = Status::warning("没有可保存的便签。");
            cx.notify();
            return;
        };
        let empty_note = self
            .editor_note
            .and_then(|id| self.model.get(id))
            .is_some_and(|note| note.title.trim().is_empty());
        self.status = Status::success(if empty_note {
            format!("已保存：{display}（标题为空，列表显示为占位名）")
        } else {
            format!("已保存：{display}")
        });
        self.menu_open = false;
        cx.notify();
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.editor_note else {
            return;
        };
        if !self.is_dirty(cx) {
            self.status = Status::info("当前没有未保存的修改。");
            cx.notify();
            return;
        }
        self.load_into_editor(window, cx, id);
        self.status = Status::info("已还原上次保存的内容；光标回到文首，选择已清除。");
        // Menu items do not close the MenuDropdown themselves, so an action
        // that changes editor state has to close it explicitly. Without this,
        // Cancel restores the draft but leaves the (now disabled) row visible
        // and focus inside the panel.
        self.menu_open = false;
        cx.notify();
    }

    fn set_pinned(&mut self, pinned: bool, cx: &mut Context<Self>) {
        let Some(id) = self.selected else {
            return;
        };
        let Some(label) = self
            .model
            .get(id)
            .map(|note| note.display_title().to_string())
        else {
            return;
        };
        self.model.set_pinned(id, pinned);
        self.status = Status::info(if pinned {
            format!("已置顶：{label}")
        } else {
            format!("已取消置顶：{label}")
        });
        self.menu_open = false;
        cx.notify();
    }

    fn toggle_pinned(&mut self, cx: &mut Context<Self>) {
        let Some(pinned) = self.current().map(|note| note.pinned) else {
            return;
        };
        self.set_pinned(!pinned, cx);
    }

    fn set_tone(&mut self, tone: NoteTone, cx: &mut Context<Self>) {
        let Some(id) = self.selected else {
            return;
        };
        if self.model.set_tone(id, tone) {
            self.status = Status::info(format!("颜色标记已设为：{}", tone.label()));
            cx.notify();
        }
    }

    fn request_delete(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected else {
            return;
        };
        self.pending = Some(PendingAction::Delete(id));
        self.close_overlays();
        cx.notify();
    }

    fn confirm_pending(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        match pending {
            PendingAction::SwitchTo(id) => self.activate(window, cx, id),
            PendingAction::NewNote => self.create_note(window, cx),
            PendingAction::Delete(id) => {
                let label = self
                    .model
                    .get(id)
                    .map(|note| note.display_title().to_string())
                    .unwrap_or_else(|| "该便签".to_string());
                let fallback = self.model.delete(id);
                match fallback {
                    Some(next) => {
                        self.selected = Some(next);
                        self.load_into_editor(window, cx, next);
                    }
                    None => {
                        self.selected = None;
                        self.clear_editor(window, cx);
                    }
                }
                self.close_overlays();
                self.status = Status::info(format!(
                    "已删除「{label}」。当前共 {} 条便签。",
                    self.model.len()
                ));
                cx.notify();
            }
        }
    }

    /// Saves the draft and then carries out the pending switch/create.
    fn save_and_continue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(display) = self.save_current(cx) else {
            return;
        };
        let Some(pending) = self.pending.take() else {
            return;
        };
        let suffix = match pending {
            PendingAction::SwitchTo(id) => {
                self.activate(window, cx, id);
                "已切换"
            }
            PendingAction::NewNote => {
                self.create_note(window, cx);
                "已新建便签"
            }
            PendingAction::Delete(_) => "未删除",
        };
        self.status = Status::success(format!("已保存：{display}；{suffix}"));
        cx.notify();
    }

    fn cancel_pending(&mut self, cx: &mut Context<Self>) {
        if self.pending.take().is_some() {
            self.status = Status::info("已取消操作，当前草稿保留。");
            cx.notify();
        }
    }

    fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = false;
        cx.notify();
    }

    fn close_props(&mut self, cx: &mut Context<Self>) {
        self.props_open = false;
        cx.notify();
    }

    fn apply_appearance(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        if self.appearance == appearance {
            return;
        }
        self.appearance = appearance;
        FluentTheme::global(cx)
            .with_appearance(appearance)
            .install(cx);
        self.status = Status::info(format!("主题已切换：{}", appearance_label(appearance)));
        cx.notify();
    }

    fn set_show_counts(&mut self, show: bool, cx: &mut Context<Self>) {
        self.show_counts = show;
        cx.notify();
    }

    fn set_compact_list(&mut self, compact: bool, cx: &mut Context<Self>) {
        self.compact_list = compact;
        cx.notify();
    }

    // ---- render --------------------------------------------------------

    fn render_root(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let narrow = window.viewport_size().width < px(680.);
        let body = self.render_body(cx, narrow);
        FluentRoot::new()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_top_bar(cx, narrow))
            .child(body)
            // A pending decision mounts modal dialogs; they render themselves
            // deferred above everything here, dim the page and trap focus.
            // Both stay mounted so the closing frame can hand focus back.
            .children(self.render_pending_dialogs(cx))
            .into_any_element()
    }

    fn settings_popover(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let trigger = Button::new("notes-settings-trigger")
            .appearance(ButtonAppearance::Subtle)
            .icon(FluentIcon::Settings)
            .icon_only()
            .tooltip("设置")
            .accessibility_label("设置")
            .on_click(cx.listener(|this, _, _, cx| this.toggle_settings(cx)));
        let weak = cx.entity().downgrade();

        Popover::new("notes-settings-popover")
            .open(self.settings_open)
            .track_focus(&self.settings_focus)
            .on_open_change(move |open, _, cx| {
                weak.update(cx, |this, cx| {
                    if this.settings_open != open {
                        this.settings_open = open;
                        cx.notify();
                    }
                })
                .ok();
            })
            .trigger(trigger)
            .content(self.render_settings(cx))
    }

    fn render_top_bar(&self, cx: &mut Context<Self>, narrow: bool) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let mut bar = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(16.))
            .py(px(10.))
            .border_b_1()
            .border_color(t.color.neutral_stroke_1)
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("便签"),
            )
            .child(
                Badge::new("内存模式")
                    .appearance(BadgeAppearance::Informative)
                    .size(BadgeSize::Small),
            );

        if !narrow {
            bar = bar.child(
                div()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_3)
                    .child("关闭窗口后不保留；不做账号、联网或云同步。"),
            );
        }

        bar = bar.child(div().flex_1()).child(
            Button::new("notes-new-top")
                .appearance(ButtonAppearance::Primary)
                .icon(FluentIcon::Add)
                .tooltip("新建便签")
                .on_click(cx.listener(|this, _, window, cx| this.request_new_note(window, cx)))
                .child("新建便签"),
        );

        if narrow {
            bar = bar.child(self.settings_popover(cx));
        } else {
            bar = bar
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(t.color.neutral_foreground_3)
                        .child(format!("共 {} 条", self.model.len())),
                )
                .child(self.settings_popover(cx));
        }
        bar.into_any_element()
    }

    fn render_body(&self, cx: &mut Context<Self>, narrow: bool) -> AnyElement {
        div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .child(self.render_sidebar(cx, narrow))
            .child(self.render_editor(cx, narrow))
            .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>, narrow: bool) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let mut header = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .pt(px(12.))
            .pb(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("所有便签"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(format!("{}", self.model.len())),
            )
            .child(div().flex_1())
            .child(
                Button::new("notes-new-sidebar")
                    .appearance(ButtonAppearance::Subtle)
                    .icon(FluentIcon::Add)
                    .icon_only()
                    .tooltip("新建便签")
                    .accessibility_label("新建便签")
                    .on_click(cx.listener(|this, _, window, cx| this.request_new_note(window, cx))),
            );

        if !narrow {
            header = header.child(
                Button::new("notes-new-sidebar-text")
                    .appearance(ButtonAppearance::Secondary)
                    .size(ControlSize::Small)
                    .on_click(cx.listener(|this, _, window, cx| this.request_new_note(window, cx)))
                    .child("新建"),
            );
        }

        let mut list = div()
            .id("notes-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.))
            .px(px(8.))
            .pb(px(8.));

        if self.model.is_empty() {
            list = list.child(self.render_empty_list(cx));
        } else {
            for id in self.model.order_ids() {
                let Some(note) = self.model.get(id) else {
                    continue;
                };
                list = list.child(self.render_note_row(note, cx));
            }
        }

        div()
            .w(relative(0.32))
            .min_w(px(190.))
            .max_w(px(300.))
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(t.color.neutral_stroke_1)
            .bg(t.color.neutral_background_2)
            .child(header)
            .child(Divider::new())
            .child(list)
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(t.color.neutral_stroke_1)
                    .text_size(px(11.))
                    .text_color(t.color.neutral_foreground_3)
                    .whitespace_normal()
                    .child("仅存于内存；关闭窗口后便签不会保留。"),
            )
            .into_any_element()
    }

    fn render_empty_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.))
            .p(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.color.neutral_foreground_2)
                    .child("还没有便签"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_3)
                    .whitespace_normal()
                    .child("新建一条开始记录。内容只保存在内存中。"),
            )
            .child(
                Button::new("notes-empty-list-new")
                    .appearance(ButtonAppearance::Secondary)
                    .icon(FluentIcon::Add)
                    .on_click(cx.listener(|this, _, window, cx| this.request_new_note(window, cx)))
                    .child("新建便签"),
            )
            .into_any_element()
    }

    fn render_note_row(&self, note: &Note, cx: &mut Context<Self>) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let selected = self.selected == Some(note.id);
        let id = note.id;
        let tone_color = match note.tone {
            NoteTone::None => None,
            NoteTone::Brand => Some(t.color.brand_background),
            NoteTone::Warning => Some(t.color.status_warning_background_2),
        };
        let display_title = note.display_title().to_string();
        let preview = note.preview(BODY_PREVIEW_CHARS);
        let counts = format!(
            "标题 {} 字 · 正文 {} 字",
            note.title_chars(),
            note.body_chars()
        );

        let mut title_line = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .w_full()
            .min_w(px(0.))
            .when_some(tone_color, |el, color| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .w(px(3.))
                        .h(px(14.))
                        .rounded(px(2.))
                        .bg(color),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(display_title.clone()),
            );

        if note.pinned {
            title_line = title_line.child(
                Badge::new("置顶")
                    .appearance(BadgeAppearance::Brand)
                    .size(BadgeSize::Small),
            );
        }

        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .w_full()
            .min_w(px(0.))
            .child(title_line);

        if !self.compact_list {
            content = content.child(
                div()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(preview),
            );
        }

        if self.show_counts {
            content = content.child(
                div()
                    .truncate()
                    .text_size(px(11.))
                    .text_color(t.color.neutral_foreground_4)
                    .child(counts),
            );
        }

        Button::new(("notes-row", id.0))
            .appearance(if selected {
                ButtonAppearance::Secondary
            } else {
                ButtonAppearance::Subtle
            })
            .w_full()
            .justify_start()
            .px(px(8.))
            .py(if self.compact_list { px(4.) } else { px(6.) })
            .tooltip(format!("打开：{}", display_title.clone()))
            .on_click(cx.listener(move |this, _, window, cx| this.request_select(window, cx, id)))
            .child(content)
            .into_any_element()
    }

    fn render_editor(&self, cx: &mut Context<Self>, narrow: bool) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let Some(note) = self.current() else {
            return self.render_empty_editor(cx);
        };
        let pinned = note.pinned;
        let display_title = note.display_title().to_string();
        let dirty = self.is_dirty(cx);
        let (title_value, body_value) = self.draft(cx);
        let title_chars = title_value.chars().count();
        let body_chars = body_value.chars().count();
        let counts = if narrow {
            format!("{title_chars} / {body_chars} 字")
        } else {
            format!("标题 {title_chars} 字 · 正文 {body_chars} 字")
        };

        let weak = cx.entity().downgrade();
        let weak_pin = weak.clone();
        let weak_save = weak.clone();
        let weak_cancel = weak.clone();
        let weak_delete = weak.clone();

        let menu = Menu::new("notes-actions-menu")
            .header(MenuHeader::new("便签操作"))
            .item(
                MenuItem::new("notes-menu-save", "保存")
                    .icon(FluentIcon::CheckmarkCircle)
                    .on_click(move |window, cx| {
                        weak_save.update(cx, |this, cx| this.save(window, cx)).ok();
                    }),
            )
            .item(
                MenuItem::new("notes-menu-cancel", "取消修改")
                    .disabled(!dirty)
                    .on_click(move |window, cx| {
                        weak_cancel
                            .update(cx, |this, cx| this.cancel(window, cx))
                            .ok();
                    }),
            )
            .separator()
            .item(
                MenuItem::new("notes-menu-pin", if pinned { "取消置顶" } else { "置顶" })
                    .checked(pinned)
                    .on_click(move |_, cx| {
                        weak_pin.update(cx, |this, cx| this.toggle_pinned(cx)).ok();
                    }),
            )
            .separator()
            .item(MenuItem::new("notes-menu-move", "移动到文件夹…").disabled(true))
            .separator()
            .item(
                MenuItem::new("notes-menu-delete", "删除便签")
                    .danger(true)
                    .on_click(move |_, cx| {
                        weak_delete
                            .update(cx, |this, cx| this.request_delete(cx))
                            .ok();
                    }),
            );

        let menu_trigger = Button::new("notes-menu-trigger")
            .appearance(ButtonAppearance::Subtle)
            .icon(FluentIcon::MoreHorizontal)
            .icon_only()
            .tooltip("更多操作")
            .accessibility_label("更多操作")
            .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(cx)));

        let menu = MenuDropdown::new("notes-actions-dropdown", menu_trigger, menu)
            .open(self.menu_open)
            .track_focus(&self.menu_focus)
            .on_open_change({
                let weak = weak.clone();
                move |open, _, cx| {
                    weak.update(cx, |this, cx| {
                        if this.menu_open != open {
                            this.menu_open = open;
                            cx.notify();
                        }
                    })
                    .ok();
                }
            });

        let props_trigger = Button::new("notes-props-trigger")
            .appearance(ButtonAppearance::Secondary)
            .icon_after(FluentIcon::ChevronDown)
            .tooltip("编辑便签属性")
            .on_click(cx.listener(|this, _, _, cx| this.toggle_props(cx)))
            .child("属性");

        let props = Popover::new("notes-props-popover")
            .open(self.props_open)
            .track_focus(&self.props_focus)
            .on_open_change({
                let weak = weak.clone();
                move |open, _, cx| {
                    weak.update(cx, |this, cx| {
                        if this.props_open != open {
                            this.props_open = open;
                            cx.notify();
                        }
                    })
                    .ok();
                }
            })
            .trigger(props_trigger)
            .content(self.render_props(cx));

        let toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(
                Button::new("notes-save")
                    .appearance(ButtonAppearance::Primary)
                    .tooltip("保存当前便签")
                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))
                    .child("保存"),
            )
            .child(
                Button::new("notes-cancel")
                    .appearance(ButtonAppearance::Secondary)
                    .disabled(!dirty)
                    .tooltip("恢复上次保存的标题与正文")
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx)))
                    .child("取消"),
            )
            .child(
                Button::new("notes-pin")
                    .appearance(ButtonAppearance::Secondary)
                    .tooltip(if pinned {
                        "取消置顶当前便签"
                    } else {
                        "置顶当前便签"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pinned(cx)))
                    .child(if pinned { "取消置顶" } else { "置顶" }),
            )
            .child(props)
            .child(menu)
            .child(div().flex_1())
            .child(
                Button::new("notes-delete")
                    .appearance(ButtonAppearance::Subtle)
                    .text_color(t.color.status_danger_foreground_1)
                    .tooltip("删除当前便签")
                    .on_click(cx.listener(|this, _, _, cx| this.request_delete(cx)))
                    .child("删除"),
            );

        // The header states "编辑中 · 未保存/已保存" in text. A second Warning
        // badge is not used: its foreground is unreadable on the high-contrast
        // (white-on-white) and low-contrast in dark; see the app's known issues.
        // `dirty` still drives the header, Cancel and the menu.

        let body_editor = div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .child(Textarea::new(&self.body_input).flex_1().min_h(px(0.)));

        let mut panel = div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w(px(0.))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(12.))
                            .text_color(t.color.neutral_foreground_3)
                            .child(if narrow {
                                if dirty {
                                    "编辑中 · 未保存"
                                } else {
                                    "已保存"
                                }
                            } else {
                                if dirty {
                                    "编辑中 · 未保存"
                                } else {
                                    "编辑中 · 已保存"
                                }
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_right()
                            .text_size(px(12.))
                            .text_color(t.color.neutral_foreground_3)
                            .child(format!("当前：{display_title}")),
                    ),
            )
            .child(if narrow {
                Field::new("标题")
                    .hint("标题为空时显示为「无标题便签」。")
                    .child(TextInput::new(&self.title_input))
            } else {
                Field::new("标题")
                    .hint("标题为空时，列表显示为「无标题便签」；保存不会自动改写输入内容。")
                    .child(TextInput::new(&self.title_input))
            })
            .child(toolbar);

        panel = panel
            .child(
                div()
                    .flex_1()
                    .min_h(px(160.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(Label::new("正文"))
                    .child(body_editor),
            )
            .child(self.render_status(counts, cx, narrow));

        panel.into_any_element()
    }

    /// The pending decision as real modal dialogs.
    ///
    /// `pending` is the controlled open state: a switch/new-note decision
    /// drives a `Dialog`, a delete decision drives an `AlertDialog`. Both stay
    /// mounted — a dialog element that unmounts while open can never run its
    /// closing frame, which is where focus is handed back. Their content is
    /// only laid out while open, so a closed dialog is a no-op element.
    fn render_pending_dialogs(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let pending = self.pending;

        let switch_open = matches!(
            pending,
            Some(PendingAction::SwitchTo(_)) | Some(PendingAction::NewNote)
        );
        let delete_open = matches!(pending, Some(PendingAction::Delete(_)));

        let (title, message, save_label, discard_label) = match pending {
            Some(PendingAction::SwitchTo(target)) => {
                let label = self
                    .model
                    .get(target)
                    .map(|note| note.display_title().to_string())
                    .unwrap_or_else(|| "目标便签".to_string());
                (
                    "切换便签",
                    format!("当前便签有未保存的修改。切换到「{label}」前要保存吗？"),
                    "保存并切换",
                    "放弃修改并切换",
                )
            }
            Some(PendingAction::NewNote) => (
                "新建便签",
                "当前便签有未保存的修改。新建便签前要保存吗？".to_string(),
                "保存并新建",
                "放弃修改并新建",
            ),
            _ => ("", String::new(), "", ""),
        };

        let weak = cx.entity().downgrade();
        let weak_ok = weak.clone();
        let weak_cancel = weak.clone();
        let weak_change = weak.clone();
        let switch_dialog = Dialog::new("notes-pending-switch")
            .title(title)
            .open(switch_open)
            // Escape and a scrim press are "stay"; Enter takes the primary
            // save path, matching the old bar's primary button. Both still go
            // through the app handlers.
            .on_ok(move |_, window, cx| {
                weak_ok
                    .update(cx, |this, cx| this.save_and_continue(window, cx))
                    .is_ok()
            })
            .on_cancel(move |_, _, cx| {
                weak_cancel
                    .update(cx, |this, cx| this.cancel_pending(cx))
                    .is_ok()
            })
            .on_open_change(move |open, _, _, cx| {
                if !open {
                    weak_change
                        .update(cx, |this, cx| {
                            this.pending = None;
                            cx.notify();
                        })
                        .ok();
                }
            })
            .child(message)
            .footer_child(
                Button::new("notes-pending-stay")
                    .appearance(ButtonAppearance::Secondary)
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_pending(cx)))
                    .child("留在当前"),
            )
            .footer_child(
                Button::new("notes-pending-discard")
                    .appearance(ButtonAppearance::Secondary)
                    .on_click(cx.listener(|this, _, window, cx| this.confirm_pending(window, cx)))
                    .child(discard_label),
            )
            .footer_child(
                Button::new("notes-pending-save")
                    .appearance(ButtonAppearance::Primary)
                    .on_click(cx.listener(|this, _, window, cx| this.save_and_continue(window, cx)))
                    .child(save_label),
            )
            .into_any_element();

        let delete_label = match pending {
            Some(PendingAction::Delete(id)) => self
                .model
                .get(id)
                .map(|note| note.display_title().to_string())
                .unwrap_or_else(|| "该便签".to_string()),
            _ => String::new(),
        };
        let weak_ok = weak.clone();
        let weak_change = weak.clone();
        let delete_dialog = AlertDialog::new("notes-pending-delete")
            .title("删除便签")
            .open(delete_open)
            // Enter counts as the destructive confirm; Escape and scrim
            // presses cannot dismiss an alert dialog.
            .on_ok(move |_, window, cx| {
                weak_ok
                    .update(cx, |this, cx| this.confirm_pending(window, cx))
                    .is_ok()
            })
            .on_open_change(move |open, _, _, cx| {
                if !open {
                    weak_change
                        .update(cx, |this, cx| {
                            this.pending = None;
                            cx.notify();
                        })
                        .ok();
                }
            })
            .child(format!(
                "删除「{delete_label}」？删除后无法撤销；未保存的修改也会一起丢弃。"
            ))
            .footer_child(
                Button::new("notes-pending-cancel")
                    .appearance(ButtonAppearance::Secondary)
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_pending(cx)))
                    .child("取消"),
            )
            .footer_child(
                Button::new("notes-pending-delete")
                    .appearance(ButtonAppearance::Secondary)
                    .text_color(t.color.status_danger_foreground_1)
                    .on_click(cx.listener(|this, _, window, cx| this.confirm_pending(window, cx)))
                    .child("确认删除"),
            )
            .into_any_element();

        vec![switch_dialog, delete_dialog]
    }

    fn render_status(&self, counts: String, cx: &mut Context<Self>, narrow: bool) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let (icon, color) = match self.status.tone {
            StatusTone::Info => (FluentIcon::Info, t.color.neutral_foreground_3),
            StatusTone::Success => (
                FluentIcon::CheckmarkCircle,
                t.color.status_success_foreground_1,
            ),
            StatusTone::Warning => (FluentIcon::Warning, t.color.status_warning_foreground_1),
        };
        let running = format!("{} 条便签 · 内存模式", self.model.len());

        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .pt(px(8.))
            .border_t_1()
            .border_color(t.color.neutral_stroke_1)
            .child(Icon::new(icon).size(px(16.)).color(color))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_2)
                    .child(self.status.text.clone()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(t.color.neutral_foreground_3)
                    .child(counts),
            )
            .when(!narrow, |el| {
                el.child(
                    div()
                        .text_size(px(11.))
                        .text_color(t.color.neutral_foreground_4)
                        .child(running),
                )
            })
            .into_any_element()
    }

    fn render_empty_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let empty = self.model.is_empty();
        let (heading, detail) = if empty {
            (
                "还没有便签",
                "新建一条开始记录。所有内容只保存在内存中，关闭窗口后不会保留。",
            )
        } else {
            ("未选择的便签", "从左侧选择一条便签，或新建一条。")
        };

        div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .p(px(32.))
            .child(
                div()
                    .w(px(380.))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        Icon::new(FluentIcon::Info)
                            .size(px(32.))
                            .color(t.color.brand_foreground_1),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(heading),
                    )
                    .child(
                        div()
                            .whitespace_normal()
                            .text_center()
                            .text_size(px(13.))
                            .text_color(t.color.neutral_foreground_3)
                            .child(detail),
                    )
                    .child(
                        Button::new("notes-empty-editor-new")
                            .appearance(ButtonAppearance::Primary)
                            .icon(FluentIcon::Add)
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.request_new_note(window, cx)
                                }),
                            )
                            .child("新建便签"),
                    ),
            )
            .into_any_element()
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let hint = t.color.neutral_foreground_3;
        let label = |text: &'static str| {
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(hint)
                .child(text)
        };

        div()
            .w(px(320.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("设置"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(hint)
                    .whitespace_normal()
                    .child("外观与列表显示只影响当前会话；应用不保存任何偏好。"),
            )
            .child(Divider::new())
            .child(label("外观主题（受控）"))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        Radio::new("notes-theme-light")
                            .checked(self.appearance == Appearance::Light)
                            .label("浅色")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.apply_appearance(Appearance::Light, cx);
                                }
                            })),
                    )
                    .child(
                        Radio::new("notes-theme-dark")
                            .checked(self.appearance == Appearance::Dark)
                            .label("深色")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.apply_appearance(Appearance::Dark, cx);
                                }
                            })),
                    )
                    .child(
                        Radio::new("notes-theme-hc")
                            .checked(self.appearance == Appearance::HighContrast)
                            .label("高对比")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.apply_appearance(Appearance::HighContrast, cx);
                                }
                            })),
                    ),
            )
            .child(Divider::new())
            .child(label("列表显示（受控）"))
            .child(
                Checkbox::new("notes-settings-counts")
                    .checked(self.show_counts)
                    .label("在列表中显示字数")
                    .on_change(cx.listener(|this, checked, _, cx| {
                        this.set_show_counts(*checked, cx)
                    })),
            )
            .child(
                Switch::new("notes-settings-compact")
                    .checked(self.compact_list)
                    .label("紧凑列表（隐藏正文预览）")
                    .on_change(cx.listener(|this, checked, _, cx| {
                        this.set_compact_list(*checked, cx)
                    })),
            )
            .child(Divider::new())
            .child(label("非受控示例（组件自持状态）"))
            .child(
                Checkbox::new("notes-settings-uncontrolled-checkbox")
                    .default_checked(true)
                    .label("启动时显示操作提示")
                    .on_change(cx.listener(|_, _, _, _| {})),
            )
            .child(
                Switch::new("notes-settings-uncontrolled-switch")
                    .default_checked(false)
                    .label("自动保存草稿")
                    .on_change(cx.listener(|_, _, _, _| {})),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(hint)
                    .whitespace_normal()
                    .child(
                        "以上两个控件由组件自己保存状态，应用不读取；切换主题、开关浮层或重绘后应保持用户选择。",
                    ),
            )
            .child(Divider::new())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(
                        Button::new("notes-settings-done")
                            .appearance(ButtonAppearance::Primary)
                            .on_click(cx.listener(|this, _, _, cx| this.close_settings(cx)))
                            .child("完成"),
                    ),
            )
            .into_any_element()
    }

    fn render_props(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = FluentTheme::global(cx);
        let t = theme.tokens();
        let hint = t.color.neutral_foreground_3;
        let Some(note) = self.current() else {
            return div()
                .w(px(260.))
                .text_size(px(12.))
                .child("没有选中的便签。")
                .into_any_element();
        };
        let pinned = note.pinned;
        let tone = note.tone;
        let title = note.display_title().to_string();

        div()
            .w(px(280.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("便签属性"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(hint)
                    .truncate()
                    .child(format!("当前：{title}")),
            )
            .child(Divider::new())
            .child(
                Switch::new("notes-props-pin")
                    .checked(pinned)
                    .label("置顶便签")
                    .on_change(cx.listener(|this, checked, _, cx| this.set_pinned(*checked, cx))),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(hint)
                    .child("颜色标记（受控）"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        Radio::new("notes-props-tone-none")
                            .checked(tone == NoteTone::None)
                            .label("无")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.set_tone(NoteTone::None, cx);
                                }
                            })),
                    )
                    .child(
                        Radio::new("notes-props-tone-brand")
                            .checked(tone == NoteTone::Brand)
                            .label("品牌")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.set_tone(NoteTone::Brand, cx);
                                }
                            })),
                    )
                    .child(
                        Radio::new("notes-props-tone-warning")
                            .checked(tone == NoteTone::Warning)
                            .label("警告")
                            .on_change(cx.listener(|this, checked, _, cx| {
                                if *checked {
                                    this.set_tone(NoteTone::Warning, cx);
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(hint)
                    .whitespace_normal()
                    .child("属性立即写入当前内存便签；关闭窗口后不会保留。"),
            )
            .child(Divider::new())
            .child(
                div().flex().justify_end().child(
                    Button::new("notes-props-done")
                        .appearance(ButtonAppearance::Secondary)
                        .on_click(cx.listener(|this, _, _, cx| this.close_props(cx)))
                        .child("完成"),
                ),
            )
            .into_any_element()
    }
}

impl Render for NotesApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_root(window, cx)
    }
}

fn appearance_label(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Light => "浅色 light",
        Appearance::Dark => "深色 dark",
        Appearance::HighContrast => "高对比 high-contrast",
    }
}

#[cfg(test)]
mod tests;
