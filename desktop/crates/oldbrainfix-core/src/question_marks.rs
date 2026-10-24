//! The hand-drawn question marks behind every page, and where they lie: a
//! copy of the Dart core's `question_marks.dart`, so the desktop draws the
//! very same background. The shapes are path commands in a
//! [`BOX`]-sized square; the scatter uses Mulberry32 seeded per cell.

/// Side of the square every shape is drawn in; the centre is (50, 50).
pub const BOX: f64 = 100.0;

/// One hand-drawn question mark.
///
/// Each stroke is `[x0, y0, cx1, cy1, x1, y1, cx2, cy2, x2, y2, ...]`: a
/// move to (x0, y0), then quadratic curves through control point
/// (cx, cy) to (x, y), drawn with round caps and joins at `weight`.
/// `dot` is a filled circle `[x, y, radius]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub strokes: &'static [&'static [f64]],
    pub dot: Option<[f64; 3]>,
    pub weight: f64,
}

/// The set the background is drawn from.
pub const SHAPES: [Shape; 14] = [
    // Classic.
    Shape {
        strokes: &[&[
            28.0, 32.0, 26.0, 8.0, 50.0, 8.0, 74.0, 8.0, 72.0, 30.0, 70.0, 46.0, 54.0, 54.0, 48.0,
            58.0, 49.0, 72.0,
        ]],
        dot: Some([50.0, 88.0, 5.5]),
        weight: 9.0,
    },
    // Wide and round.
    Shape {
        strokes: &[&[
            20.0, 30.0, 20.0, 4.0, 50.0, 5.0, 82.0, 6.0, 78.0, 32.0, 74.0, 50.0, 55.0, 56.0, 46.0,
            60.0, 48.0, 74.0,
        ]],
        dot: Some([48.0, 90.0, 6.0]),
        weight: 9.0,
    },
    // Narrow, leaning right.
    Shape {
        strokes: &[&[
            34.0, 26.0, 36.0, 4.0, 56.0, 6.0, 76.0, 10.0, 70.0, 30.0, 64.0, 46.0, 56.0, 58.0, 52.0,
            66.0, 54.0, 76.0,
        ]],
        dot: Some([55.0, 91.0, 5.0]),
        weight: 8.0,
    },
    // Starts with a little inward curl.
    Shape {
        strokes: &[&[
            36.0, 34.0, 22.0, 30.0, 28.0, 18.0, 36.0, 4.0, 54.0, 6.0, 76.0, 10.0, 72.0, 30.0, 68.0,
            46.0, 52.0, 54.0, 46.0, 60.0, 50.0, 72.0,
        ]],
        dot: Some([50.0, 88.0, 5.0]),
        weight: 9.0,
    },
    // Long swooping tail.
    Shape {
        strokes: &[&[
            24.0, 28.0, 30.0, 6.0, 52.0, 8.0, 76.0, 12.0, 70.0, 32.0, 62.0, 48.0, 50.0, 56.0, 40.0,
            64.0, 46.0, 78.0,
        ]],
        dot: Some([47.0, 92.0, 4.5]),
        weight: 8.0,
    },
    // Kinked, a bit angular.
    Shape {
        strokes: &[&[
            26.0, 30.0, 34.0, 10.0, 50.0, 10.0, 70.0, 10.0, 70.0, 28.0, 70.0, 40.0, 52.0, 48.0,
            50.0, 49.0, 50.0, 68.0,
        ]],
        dot: Some([50.0, 86.0, 6.0]),
        weight: 10.0,
    },
    // Sketched twice, slightly off.
    Shape {
        strokes: &[
            &[
                28.0, 32.0, 26.0, 8.0, 50.0, 8.0, 74.0, 8.0, 72.0, 30.0, 70.0, 46.0, 54.0, 54.0,
                48.0, 58.0, 49.0, 72.0,
            ],
            &[
                31.0, 29.0, 31.0, 10.0, 52.0, 10.0, 72.0, 12.0, 70.0, 30.0, 66.0, 46.0, 52.0, 56.0,
                50.0, 60.0, 51.0, 70.0,
            ],
        ],
        dot: Some([51.0, 87.0, 5.0]),
        weight: 6.0,
    },
    // Leaning left.
    Shape {
        strokes: &[&[
            30.0, 34.0, 24.0, 10.0, 46.0, 7.0, 70.0, 6.0, 66.0, 28.0, 62.0, 44.0, 46.0, 54.0, 40.0,
            60.0, 42.0, 74.0,
        ]],
        dot: Some([42.0, 89.0, 5.5]),
        weight: 9.0,
    },
    // Small tight hook on a long stem.
    Shape {
        strokes: &[&[
            34.0, 24.0, 36.0, 8.0, 50.0, 8.0, 64.0, 8.0, 64.0, 22.0, 64.0, 34.0, 52.0, 40.0, 48.0,
            44.0, 49.0, 72.0,
        ]],
        dot: Some([50.0, 88.0, 5.0]),
        weight: 9.0,
    },
    // Tail ending in a little foot.
    Shape {
        strokes: &[&[
            24.0, 30.0, 26.0, 8.0, 50.0, 8.0, 74.0, 8.0, 72.0, 28.0, 70.0, 44.0, 54.0, 52.0, 46.0,
            56.0, 48.0, 66.0, 50.0, 72.0, 56.0, 70.0,
        ]],
        dot: Some([50.0, 86.0, 5.0]),
        weight: 9.0,
    },
    // The dot is a quick dash.
    Shape {
        strokes: &[
            &[
                28.0, 32.0, 26.0, 8.0, 50.0, 8.0, 74.0, 8.0, 72.0, 30.0, 70.0, 46.0, 54.0, 54.0,
                48.0, 58.0, 49.0, 70.0,
            ],
            &[48.0, 84.0, 50.0, 87.0, 51.0, 92.0],
        ],
        dot: None,
        weight: 9.0,
    },
    // Heavy and round.
    Shape {
        strokes: &[&[
            22.0, 34.0, 18.0, 6.0, 50.0, 6.0, 80.0, 6.0, 78.0, 30.0, 76.0, 48.0, 56.0, 54.0, 50.0,
            56.0, 50.0, 70.0,
        ]],
        dot: Some([50.0, 87.0, 7.0]),
        weight: 12.0,
    },
    // Wobbly hand.
    Shape {
        strokes: &[&[
            26.0, 30.0, 28.0, 14.0, 40.0, 10.0, 52.0, 4.0, 62.0, 10.0, 76.0, 18.0, 70.0, 32.0,
            66.0, 42.0, 56.0, 48.0, 48.0, 54.0, 52.0, 62.0, 54.0, 68.0, 50.0, 74.0,
        ]],
        dot: Some([51.0, 89.0, 5.0]),
        weight: 9.0,
    },
    // Open hook starting low.
    Shape {
        strokes: &[&[
            22.0, 40.0, 22.0, 6.0, 52.0, 6.0, 80.0, 8.0, 74.0, 32.0, 66.0, 52.0, 50.0, 56.0, 48.0,
            58.0, 48.0, 74.0,
        ]],
        dot: Some([49.0, 88.0, 5.0]),
        weight: 10.0,
    },
];

/// The background's fixed seed: every Device shows the same scatter.
pub const SEED: u32 = 20261024;

/// The plane is made of square cells with one mark each.
pub const CELL: f64 = 150.0;

/// Drawn height of the smallest and the largest mark.
pub const MIN_SIZE: f64 = 34.0;
pub const MAX_SIZE: f64 = 74.0;

/// Largest tilt either way, in radians.
pub const MAX_TURN: f64 = 0.6;

/// One mark on the plane: which shape, its centre, its drawn size (the
/// side of the shape's box) and its tilt in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuestionMark {
    pub shape: usize,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub rotation: f64,
}

/// The marks of every cell touching the region from (`left`, `top`) to
/// (`right`, `bottom`), in plane coordinates. Callers that draw pad the
/// region by [`MAX_SIZE`] so marks do not pop at the edges.
pub fn question_marks_in(
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    seed: u32,
) -> Vec<QuestionMark> {
    let mut marks = Vec::new();
    let first_col = (left / CELL).floor() as i64;
    let last_col = (right / CELL).floor() as i64;
    let first_row = (top / CELL).floor() as i64;
    let last_row = (bottom / CELL).floor() as i64;
    for row in first_row..=last_row {
        for col in first_col..=last_col {
            let mut random = Mulberry32::new(cell_seed(seed, col, row));
            let size = MIN_SIZE + random.next() * (MAX_SIZE - MIN_SIZE);
            // Keep the centre far enough inside the cell that marks of
            // neighbouring cells rarely touch.
            let room = CELL - size;
            let shape = (random.next() * SHAPES.len() as f64).floor() as usize;
            let x = col as f64 * CELL + size / 2.0 + random.next() * room;
            let y = row as f64 * CELL + size / 2.0 + random.next() * room;
            let rotation = (random.next() * 2.0 - 1.0) * MAX_TURN;
            marks.push(QuestionMark {
                shape,
                x,
                y,
                size,
                rotation,
            });
        }
    }
    marks
}

/// Mixes the seed with a cell's column and row into one 32-bit state; a
/// negative column or row counts as its 32-bit two's complement.
fn cell_seed(seed: u32, col: i64, row: i64) -> u32 {
    let mut h = seed;
    h = (h ^ col as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = (h ^ row as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

/// Mulberry32: a tiny generator any language can reproduce bit for bit.
struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Mulberry32 {
        Mulberry32 { state: seed }
    }

    /// The next number in [0, 1).
    fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cell_touching_the_region_holds_one_mark_inside_it() {
        let marks = question_marks_in(-10.0, -10.0, 310.0, 160.0, SEED);
        // Columns -1..=2, rows -1..=1.
        assert_eq!(marks.len(), 12);
        for m in &marks {
            assert!(m.shape < SHAPES.len());
            assert!((MIN_SIZE..MAX_SIZE).contains(&m.size));
            assert!(m.rotation.abs() <= MAX_TURN);
            let col = (m.x / CELL).floor();
            assert!(m.x - m.size / 2.0 >= col * CELL);
            assert!(m.x + m.size / 2.0 <= (col + 1.0) * CELL);
        }
        assert_eq!(marks, question_marks_in(-10.0, -10.0, 310.0, 160.0, SEED));
        assert_ne!(marks, question_marks_in(-10.0, -10.0, 310.0, 160.0, 1));
    }
}
