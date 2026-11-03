//! Pure note data and ordering rules.
//!
//! Everything here is independent of GPUI so it can be unit-tested: which note
//! is selected after a delete, how the list is ordered, and how an empty title or
//! a long preview is presented. The view owns the input entities and transient
//! UI state; this module owns the notes themselves.

/// Stable identity of a note within one session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(pub usize);

/// The optional colour marker edited from the properties popover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteTone {
    #[default]
    None,
    Brand,
    Warning,
}

impl NoteTone {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Brand => "品牌色",
            Self::Warning => "警告色",
        }
    }
}

/// One in-memory note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub tone: NoteTone,
    /// Monotonic edit counter used for ordering; larger means more recent.
    pub updated_seq: u64,
}

impl Note {
    /// What the list calls a note whose title is empty. The editor itself keeps
    /// the empty input untouched.
    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            "无标题便签"
        } else {
            &self.title
        }
    }

    /// Single-line preview for the list: whitespace (including newlines) is
    /// collapsed, empty bodies get a placeholder, and long text is cut at a
    /// character boundary rather than a byte boundary.
    pub fn preview(&self, max_chars: usize) -> String {
        let collapsed = self.body.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            return "（无正文）".to_string();
        }
        if collapsed.chars().count() <= max_chars {
            return collapsed;
        }
        let kept: String = collapsed
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect();
        format!("{kept}…")
    }

    pub fn title_chars(&self) -> usize {
        self.title.chars().count()
    }

    pub fn body_chars(&self) -> usize {
        self.body.chars().count()
    }
}

/// The whole in-memory store.
pub struct NotesModel {
    notes: Vec<Note>,
    next_id: usize,
    next_seq: u64,
}

impl NotesModel {
    /// A small set of notes covering the states the acceptance checklist needs:
    /// pinned, mixed Chinese/English, a long body, a long title, and an empty
    /// title.
    pub fn seeded() -> Self {
        let notes = vec![
            Note {
                id: NoteId(1),
                title: "欢迎使用便签".into(),
                body: "这是一个用于验证 fluent-ui 跨组件流程的小应用。\n\n\
                       所有内容只保存在内存中：关闭窗口或进程后不会保留，也不会写入磁盘。\n\n\
                       可以试试：新建便签、编辑后取消、在“更多”菜单里置顶、打开“设置”切换主题。"
                    .into(),
                pinned: true,
                tone: NoteTone::Brand,
                updated_seq: 4,
            },
            Note {
                id: NoteId(2),
                title: "会议记录 Meeting notes".into(),
                body: "周五同步 Weekly sync\n\n\
                       - 菜单的键盘导航已覆盖方向键、Home/End、Enter/Space，跳过禁用行。\n\
                       - 浮层方面：外部点击关闭、Esc 关闭、焦点交接与 Tab 陷阱都有行为测试。\n\
                       - 仍未完成：子菜单，以及 Dialog / Tabs / Sheet。\n\n\
                       长正文用来检查 Textarea 的内部滚动：切换主题、打开浮层或改变列表密度时，\
                       草稿、光标和选择都不应该被重置。"
                    .into(),
                pinned: false,
                tone: NoteTone::None,
                updated_seq: 3,
            },
            Note {
                id: NoteId(3),
                title: "买点东西".into(),
                body: "牛奶、鸡蛋、面包。".into(),
                pinned: false,
                tone: NoteTone::Warning,
                updated_seq: 2,
            },
            Note {
                id: NoteId(4),
                title: String::new(),
                body: "这条便签没有标题，用来检查列表里的“无标题便签”占位和保存后的反馈。".into(),
                pinned: false,
                tone: NoteTone::None,
                updated_seq: 1,
            },
        ];
        Self {
            next_id: 5,
            next_seq: 5,
            notes,
        }
    }

    /// An empty store; kept for symmetry with [`Self::seeded`] in tests that
    /// need a clean starting point.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            notes: Vec::new(),
            next_id: 1,
            next_seq: 1,
        }
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn get(&self, id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn get_mut(&mut self, id: NoteId) -> Option<&mut Note> {
        self.notes.iter_mut().find(|note| note.id == id)
    }

    /// Ids in display order: pinned first, then most recently edited.
    ///
    /// Ties fall back to id so the order is deterministic; seeded notes and
    /// every save use distinct edit counters, but a restored store might not.
    pub fn order_ids(&self) -> Vec<NoteId> {
        let mut notes: Vec<&Note> = self.notes.iter().collect();
        notes.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.updated_seq.cmp(&a.updated_seq))
                .then(a.id.cmp(&b.id))
        });
        notes.into_iter().map(|note| note.id).collect()
    }

    pub fn first(&self) -> Option<NoteId> {
        self.order_ids().first().copied()
    }

    /// Adds an empty note, makes it the most recent, and returns its id.
    pub fn create(&mut self) -> NoteId {
        let id = NoteId(self.next_id);
        self.next_id += 1;
        let seq = self.next_seq;
        self.next_seq += 1;
        self.notes.push(Note {
            id,
            title: String::new(),
            body: String::new(),
            pinned: false,
            tone: NoteTone::None,
            updated_seq: seq,
        });
        id
    }

    /// Writes editor contents into an existing note and marks it most recent.
    ///
    /// Returns `false` when the note no longer exists; callers should not report
    /// a successful save in that case.
    pub fn save(&mut self, id: NoteId, title: String, body: String) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        let note = self.get_mut(id).expect("checked above");
        note.title = title;
        note.body = body;
        note.updated_seq = seq;
        true
    }

    pub fn set_pinned(&mut self, id: NoteId, pinned: bool) -> bool {
        let Some(note) = self.get_mut(id) else {
            return false;
        };
        note.pinned = pinned;
        true
    }

    pub fn set_tone(&mut self, id: NoteId, tone: NoteTone) -> bool {
        let Some(note) = self.get_mut(id) else {
            return false;
        };
        note.tone = tone;
        true
    }

    /// Removes a note and returns the id that should be selected next: the
    /// following note in display order, otherwise the preceding one, otherwise
    /// `None` when the store is now empty.
    pub fn delete(&mut self, id: NoteId) -> Option<NoteId> {
        let order = self.order_ids();
        let position = order.iter().position(|candidate| *candidate == id)?;
        let fallback = order
            .get(position + 1)
            .or_else(|| position.checked_sub(1).and_then(|index| order.get(index)))
            .copied();
        self.notes.retain(|note| note.id != id);
        fallback
    }

    /// Whether the editor text still matches the saved note.
    pub fn matches(&self, id: NoteId, title: &str, body: &str) -> bool {
        self.get(id)
            .is_some_and(|note| note.title == title && note.body == body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles_in_order(model: &NotesModel) -> Vec<String> {
        model
            .order_ids()
            .into_iter()
            .map(|id| {
                model
                    .get(id)
                    .expect("ordered id exists")
                    .display_title()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn pinned_notes_come_first_then_recent_edits() {
        let model = NotesModel::seeded();
        assert_eq!(
            titles_in_order(&model),
            vec![
                "欢迎使用便签",
                "会议记录 Meeting notes",
                "买点东西",
                "无标题便签",
            ]
        );
    }

    #[test]
    fn display_title_only_replaces_blank_titles() {
        let model = NotesModel::seeded();
        let untitled = model.get(NoteId(4)).unwrap();
        assert_eq!(untitled.title, "");
        assert_eq!(untitled.display_title(), "无标题便签");

        let spaced = Note {
            title: "   \n ".into(),
            ..untitled.clone()
        };
        assert_eq!(spaced.display_title(), "无标题便签");
    }

    #[test]
    fn preview_collapses_whitespace_and_marks_an_empty_body() {
        let note = Note {
            id: NoteId(1),
            title: String::new(),
            body: "  第一行\n\n第二行\t第三行  ".into(),
            pinned: false,
            tone: NoteTone::None,
            updated_seq: 1,
        };
        assert_eq!(note.preview(80), "第一行 第二行 第三行");

        let empty = Note {
            body: " \n\t ".into(),
            ..note.clone()
        };
        assert_eq!(empty.preview(80), "（无正文）");
    }

    #[test]
    fn preview_cuts_at_a_character_boundary_and_counts_chars() {
        let note = Note {
            id: NoteId(1),
            title: String::new(),
            body: "中文字符abc".into(),
            pinned: false,
            tone: NoteTone::None,
            updated_seq: 1,
        };
        // Seven characters fit exactly; a five-character budget keeps four and
        // adds an ellipsis, without splitting a multi-byte character.
        assert_eq!(note.preview(7), "中文字符abc");
        assert_eq!(note.preview(5), "中文字符…");
        assert_eq!(note.body_chars(), 7);
    }

    #[test]
    fn creating_a_note_returns_a_fresh_id_and_puts_it_first() {
        let mut model = NotesModel::seeded();
        let id = model.create();
        assert!(model.get(id).is_some());
        assert_eq!(
            model.order_ids(),
            vec![NoteId(1), id, NoteId(2), NoteId(3), NoteId(4)],
            "the pinned note stays first; the new unpublished note is the most recent"
        );
        assert_eq!(model.len(), 5);
        assert_eq!(model.get(id).unwrap().title, "");
        assert_eq!(model.get(id).unwrap().body, "");
    }

    #[test]
    fn saving_writes_both_fields_and_moves_the_note_up() {
        let mut model = NotesModel::seeded();
        assert!(model.save(
            NoteId(3),
            "买点东西".into(),
            "牛奶、鸡蛋、面包、咖啡。".into()
        ));
        let order = model.order_ids();
        assert_eq!(order[0], NoteId(1), "the pinned note stays first");
        assert_eq!(order[1], NoteId(3), "the edited note is next");
        assert_eq!(
            model.get(NoteId(3)).unwrap().body,
            "牛奶、鸡蛋、面包、咖啡。"
        );
        assert!(!model.save(NoteId(99), String::new(), String::new()));
    }

    #[test]
    fn pinning_changes_only_placement_and_the_flag() {
        let mut model = NotesModel::seeded();
        let before = model.get(NoteId(3)).unwrap().clone();
        assert!(model.set_pinned(NoteId(3), true));
        let after = model.get(NoteId(3)).unwrap();
        assert_eq!(after.title, before.title);
        assert_eq!(after.body, before.body);
        assert!(after.pinned);
        assert_eq!(model.order_ids()[..2], [NoteId(1), NoteId(3)]);

        assert!(model.set_pinned(NoteId(3), false));
        assert_eq!(model.order_ids()[0], NoteId(1));
        assert_eq!(model.order_ids()[1], NoteId(2));
    }

    #[test]
    fn deleting_prefers_the_next_note_then_the_previous_one() {
        let mut model = NotesModel::seeded();
        // Order: 1 (pinned), 2, 3, 4. Deleting 2 selects 3.
        assert_eq!(model.delete(NoteId(2)), Some(NoteId(3)));
        assert!(model.get(NoteId(2)).is_none());
        // Deleting the last note (4) selects the preceding one (3).
        assert_eq!(model.delete(NoteId(4)), Some(NoteId(3)));
        // Deleting the pinned first note selects the next remaining note.
        assert_eq!(model.delete(NoteId(1)), Some(NoteId(3)));
        // Deleting the final note leaves nothing to select.
        assert_eq!(model.delete(NoteId(3)), None);
        assert!(model.is_empty());
    }

    #[test]
    fn deleting_an_unknown_id_does_not_change_the_store() {
        let mut model = NotesModel::seeded();
        assert_eq!(model.delete(NoteId(99)), None);
        assert_eq!(model.len(), 4);
    }

    #[test]
    fn dirty_comparison_is_exact_and_requires_the_note() {
        let model = NotesModel::seeded();
        let note = model.get(NoteId(2)).unwrap();
        assert!(model.matches(NoteId(2), &note.title, &note.body));
        assert!(!model.matches(NoteId(2), &format!("{} ", note.title), &note.body));
        assert!(!model.matches(NoteId(2), &note.title, ""));
        assert!(!model.matches(NoteId(99), "", ""));
    }

    #[test]
    fn an_empty_store_has_nothing_to_select_until_a_note_is_created() {
        let mut model = NotesModel::empty();
        assert!(model.is_empty());
        assert_eq!(model.first(), None);
        let id = model.create();
        assert_eq!(model.first(), Some(id));
    }

    #[test]
    fn tones_are_stored_per_note() {
        let mut model = NotesModel::seeded();
        assert!(model.set_tone(NoteId(4), NoteTone::Warning));
        assert_eq!(model.get(NoteId(4)).unwrap().tone, NoteTone::Warning);
        assert_eq!(model.get(NoteId(1)).unwrap().tone, NoteTone::Brand);
        assert!(!model.set_tone(NoteId(99), NoteTone::None));
    }
}
