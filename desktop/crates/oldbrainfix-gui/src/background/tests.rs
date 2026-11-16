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
