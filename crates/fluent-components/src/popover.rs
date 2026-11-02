//! Fluent 2 `Popover`.
//!
//! gpui-base owns the behaviour: anchoring, outside-click and Escape dismissal,
//! focus tracking and the open/close lifecycle. This file only supplies the
//! Fluent surface (background, radius, shadow, padding) around the content.

use fluent_core::gpui_compat::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled,
    Window, div, px,
};
use fluent_core::spec::feedback;
use fluent_theme::FluentTheme;
use gpui_base::Popover as BasePopover;

/// Called with the new open state whenever the popover shows or hides.
pub type OpenChangeHandler = Box<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A Fluent 2 popover: a floating panel anchored to a trigger.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    open: Option<bool>,
    default_open: bool,
    on_open_change: Option<OpenChangeHandler>,
    style: StyleRefinement,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            content: None,
            open: None,
            default_open: false,
            on_open_change: None,
            style: StyleRefinement::default(),
        }
    }

    /// The element that opens the popover on click.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// The floating panel's body.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Controlled open state: the caller owns it.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Uncontrolled initial state.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Box::new(handler));
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = FluentTheme::global(cx);
        let surface = feedback::tooltip(&theme);

        // gpui-base measures the trigger during prepaint but only asks for an
        // extra frame on the first capture, so while a scroll moves the trigger
        // the panel paints against the previous frame's anchor and jitters.
        // Repainting while open keeps the anchor one frame fresh instead.
        if self.open.unwrap_or(self.default_open) {
            window.request_animation_frame();
        }

        let mut base = BasePopover::new(self.id).offset(px(4.));

        if let Some(open) = self.open {
            base = base.open(open);
        } else {
            base = base.default_open(self.default_open);
        }

        if let Some(trigger) = self.trigger {
            base = base.trigger_with(move |_is_open, _, _| trigger.into_any_element());
        }

        if let Some(handler) = self.on_open_change {
            base = base.on_open_change(move |open, window, cx| handler(*open, window, cx));
        }

        if let Some(content) = self.content {
            base = base.content(move |_, _, _| {
                div()
                    .rounded(px(4.))
                    .bg(surface.background)
                    .text_color(surface.foreground)
                    .p(px(12.))
                    .shadow(surface.shadow.to_vec())
                    .child(content)
            });
        }

        base
    }
}
