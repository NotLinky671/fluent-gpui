//! Assert actual painted geometry, including fractional display scales.
use super::*;
use crate::{FluentRoot, Switch};
use gpui::{AppContext, Context, Render, TestAppContext, px, size};

struct View {
    radio: bool,
    control_size: ControlSize,
    offset: f32,
    checked: bool,
    disabled: bool,
}
impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let control = if self.radio {
            Radio::new("probe")
                .size(self.control_size)
                .checked(true)
                .disabled(self.disabled)
                .into_any_element()
        } else {
            Switch::new("probe")
                .size(self.control_size)
                .checked(self.checked)
                .disabled(self.disabled)
                .into_any_element()
        };
        FluentRoot::new().child(div().p(px(self.offset)).child(control))
    }
}

impl View {
    fn assert_geometry(&self, window: &Window, theme: &FluentTheme, scale: f32) {
        let radio = selection::radio(
            theme,
            &RadioProps {
                size: self.control_size,
                checked: true,
                disabled: self.disabled,
            },
        );
        let switch = fluent_core::spec::switch::resolve(
            theme,
            &fluent_core::spec::switch::SwitchProps {
                size: self.control_size,
                checked: self.checked,
                disabled: self.disabled,
            },
        );
        let (width, height, diameter, color) = if self.radio {
            (radio.circle, radio.circle, radio.dot, radio.dot_color)
        } else {
            (
                switch.track_width,
                switch.track_height,
                switch.thumb_size,
                switch.thumb,
            )
        };
        let quads = window.painted_quads();
        let outer = quads
            .iter()
            .find(|q| {
                (q.bounds.size.width.0 - f32::from(width) * scale).abs() < 1.1
                    && (q.bounds.size.height.0 - f32::from(height) * scale).abs() < 1.1
            })
            .expect("outer indicator painted")
            .bounds;
        // Accept either renderer, so the same regression fails against the old
        // rounded-quad implementation as well as checking the new path output.
        let inner = quads
            .iter()
            .find(|q| {
                q.background.as_solid() == Some(color) && q.bounds.size.width < outer.size.width
            })
            .map(|q| q.bounds)
            .or_else(|| {
                window
                    .painted_paths()
                    .iter()
                    .find(|p| p.color.as_solid() == Some(color))
                    .map(|p| p.bounds)
            })
            .expect("inner circle painted");
        let context = format!(
            "radio={} scale={scale} size={:?} offset={} checked={} disabled={} outer={outer:?} inner={inner:?}",
            self.radio, self.control_size, self.offset, self.checked, self.disabled
        );
        assert!(
            (inner.size.width.0 - f32::from(diameter) * scale).abs() < 0.01,
            "circle diameter changed: {context}"
        );
        assert!(
            (inner.size.width.0 - inner.size.height.0).abs() < 0.01,
            "circle became an ellipse: {context}"
        );
        assert!(
            (outer.center().y.0 - inner.center().y.0).abs() < 0.01,
            "vertical center drift: {context}"
        );
        if self.radio {
            assert!(
                (outer.center().x.0 - inner.center().x.0).abs() < 0.01,
                "horizontal center drift: {context}"
            );
        } else {
            let gap_y = inner.top().0 - outer.top().0;
            let gap_x = if self.checked {
                outer.right().0 - inner.right().0
            } else {
                inner.left().0 - outer.left().0
            };
            assert!(
                (gap_x - gap_y).abs() < 0.01,
                "asymmetric switch endpoint: {context}"
            );
        }
    }
}

fn check(cx: &mut TestAppContext, radio: bool) {
    cx.update(fluent_core::gpui_compat::init_base);
    cx.update(|cx| cx.set_reduce_motion(true));
    let handle = cx.open_window(size(px(240.), px(100.)), |_, _| View {
        radio,
        control_size: ControlSize::Medium,
        offset: 8.,
        checked: true,
        disabled: false,
    });
    let view = cx
        .update_window(handle.into(), |root, _, _| root.downcast::<View>().unwrap())
        .unwrap();
    for theme in [
        FluentTheme::web_light(),
        FluentTheme::web_dark(),
        FluentTheme::high_contrast(),
    ] {
        cx.update(|cx| theme.clone().install(cx));
        for scale in [1.0, 1.25, 1.5, 2.0] {
            cx.simulate_window_scale_factor_change(handle.into(), scale);
            cx.run_until_parked();
            for control_size in [ControlSize::Small, ControlSize::Medium, ControlSize::Large] {
                for offset in [8., 9.] {
                    for disabled in [false, true] {
                        for checked in [false, true]
                            .into_iter()
                            .filter(|checked| !radio || *checked)
                        {
                            view.update(cx, |v, cx| {
                                v.control_size = control_size;
                                v.offset = offset;
                                v.checked = checked;
                                v.disabled = disabled;
                                cx.notify();
                            });
                            cx.update_window(handle.into(), |_, window, cx| {
                                window.refresh();
                                window.draw(cx).clear(cx);
                                view.read(cx).assert_geometry(window, &theme, scale);
                            })
                            .unwrap();
                        }
                    }
                }
            }
        }
    }
}
#[gpui::test]
fn radio_dot_stays_concentric_at_display_scales(cx: &mut TestAppContext) {
    check(cx, true);
}
#[gpui::test]
fn switch_thumb_stays_centered_at_display_scales(cx: &mut TestAppContext) {
    check(cx, false);
}
