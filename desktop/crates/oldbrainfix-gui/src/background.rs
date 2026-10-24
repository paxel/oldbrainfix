//! The ground of the window: the phone's hand-drawn question marks, from
//! the same shapes and the same seeded scatter, in slow parallax with the
//! Notes list.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};
use oldbrainfix_core::question_marks::{self, BOX, MAX_SIZE, SEED, SHAPES};

/// How much slower than the Notes list the question marks move.
pub const PARALLAX: f32 = 0.3;

/// Points per quadratic curve when a stroke is flattened.
const STEPS: usize = 8;

/// Paints the ground in `background` over `rect` and the marks in
/// `mark`, the plane moved up by `shift` pixels.
pub fn paint(painter: &Painter, rect: Rect, background: Color32, mark: Color32, shift: f32) {
    painter.rect_filled(rect, 0.0, background);
    let pad = MAX_SIZE;
    let top = f64::from(shift);
    let marks = question_marks::question_marks_in(
        -pad,
        top - pad,
        f64::from(rect.width()) + pad,
        top + f64::from(rect.height()) + pad,
        SEED,
    );
    let origin = rect.min.to_vec2();
    let mut shapes = Vec::new();
    for m in marks {
        let Some(shape) = SHAPES.get(m.shape) else {
            continue;
        };
        let scale = m.size / BOX;
        let (sin, cos) = m.rotation.sin_cos();
        // The shape's box, centred on the mark, scaled and turned.
        let place = |x: f64, y: f64| -> Pos2 {
            let (dx, dy) = ((x - BOX / 2.0) * scale, (y - BOX / 2.0) * scale);
            Pos2::new(
                (m.x + dx * cos - dy * sin) as f32,
                (m.y - top + dx * sin + dy * cos) as f32,
            ) + origin
        };
        let width = (shape.weight * scale) as f32;
        for stroke in shape.strokes {
            let points = flatten(stroke, &place);
            // Round caps: a dot at each end.
            for end in [points.first(), points.last()].into_iter().flatten() {
                shapes.push(Shape::circle_filled(*end, width / 2.0, mark));
            }
            shapes.push(Shape::line(points, Stroke::new(width, mark)));
        }
        if let Some([x, y, r]) = shape.dot {
            shapes.push(Shape::circle_filled(place(x, y), (r * scale) as f32, mark));
        }
    }
    painter.with_clip_rect(rect).extend(shapes);
}

/// A stroke's move and quadratic curves as a line of points.
fn flatten(stroke: &[f64], place: &impl Fn(f64, f64) -> Pos2) -> Vec<Pos2> {
    let mut points = Vec::new();
    let (Some(&x0), Some(&y0)) = (stroke.first(), stroke.get(1)) else {
        return points;
    };
    let (mut x, mut y) = (x0, y0);
    points.push(place(x, y));
    for &[cx, cy, ex, ey] in stroke[2..].as_chunks::<4>().0 {
        for i in 1..=STEPS {
            let t = i as f64 / STEPS as f64;
            let u = 1.0 - t;
            points.push(place(
                u * u * x + 2.0 * u * t * cx + t * t * ex,
                u * u * y + 2.0 * u * t * cy + t * t * ey,
            ));
        }
        (x, y) = (ex, ey);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_flattens_through_its_end_points() {
        let id = |x: f64, y: f64| Pos2::new(x as f32, y as f32);
        let points = flatten(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0], &id);
        assert_eq!(points.len(), 1 + STEPS);
        assert_eq!(points.first(), Some(&Pos2::new(0.0, 0.0)));
        assert_eq!(points.last(), Some(&Pos2::new(10.0, 10.0)));
        assert!(flatten(&[], &id).is_empty());
    }
}
