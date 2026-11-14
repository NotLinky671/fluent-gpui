//! Paint circles without snapping their edges independently of the outer control.

use fluent_core::gpui_compat::{Hsla, Pixels, Window, point};
use gpui::{PathBuilder, Point};

pub(crate) fn paint_circle(
    center: Point<Pixels>,
    diameter: Pixels,
    color: Hsla,
    window: &mut Window,
) {
    // Rounded quads snap each circle's edges independently in this GPUI snapshot.
    // A path preserves the fractional device-pixel edges around this centre.
    let radius = diameter / 2.;
    let control = radius * 0.552_284_8;
    let mut path = PathBuilder::fill();
    let (x, y) = (center.x, center.y);
    path.move_to(point(x + radius, y));
    path.cubic_bezier_to(
        point(x, y + radius),
        point(x + radius, y + control),
        point(x + control, y + radius),
    );
    path.cubic_bezier_to(
        point(x - radius, y),
        point(x - control, y + radius),
        point(x - radius, y + control),
    );
    path.cubic_bezier_to(
        point(x, y - radius),
        point(x - radius, y - control),
        point(x - control, y - radius),
    );
    path.cubic_bezier_to(
        point(x + radius, y),
        point(x + control, y - radius),
        point(x + radius, y - control),
    );
    path.close();
    window.paint_path(path.build().expect("four finite circle segments"), color);
}
