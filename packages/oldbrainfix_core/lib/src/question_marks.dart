/// The hand-drawn question marks behind every page, and where they lie.
///
/// Everything here is plain data and integer arithmetic so the desktop
/// can copy it and draw the very same background (ADR 0001): the shapes
/// are path commands in a [questionMarkBox] square, the scatter uses its
/// own 32-bit generator instead of `dart:math`'s `Random`.
library;

/// Side of the square every shape is drawn in; the centre is (50, 50).
const questionMarkBox = 100.0;

/// One hand-drawn question mark.
///
/// Each stroke is `[x0, y0, cx1, cy1, x1, y1, cx2, cy2, x2, y2, ...]`:
/// a move to (x0, y0), then quadratic curves through control point
/// (cx, cy) to (x, y). Strokes are drawn with round caps and joins at
/// [weight]. [dot] is a filled circle `[x, y, radius]`, or null when the
/// dot is drawn as a stroke.
class QuestionMarkShape {
  final List<List<double>> strokes;
  final List<double>? dot;
  final double weight;

  const QuestionMarkShape(this.strokes, {this.dot, this.weight = 9});
}

/// The set the background is drawn from.
const questionMarkShapes = <QuestionMarkShape>[
  // Classic.
  QuestionMarkShape(
    [
      [28, 32, 26, 8, 50, 8, 74, 8, 72, 30, 70, 46, 54, 54, 48, 58, 49, 72],
    ],
    dot: [50, 88, 5.5],
  ),
  // Wide and round.
  QuestionMarkShape(
    [
      [20, 30, 20, 4, 50, 5, 82, 6, 78, 32, 74, 50, 55, 56, 46, 60, 48, 74],
    ],
    dot: [48, 90, 6],
  ),
  // Narrow, leaning right.
  QuestionMarkShape(
    [
      [34, 26, 36, 4, 56, 6, 76, 10, 70, 30, 64, 46, 56, 58, 52, 66, 54, 76],
    ],
    dot: [55, 91, 5],
    weight: 8,
  ),
  // Starts with a little inward curl.
  QuestionMarkShape(
    [
      [
        36, 34, 22, 30, 28, 18, 36, 4, 54, 6, 76, 10, 72, 30, //
        68, 46, 52, 54, 46, 60, 50, 72,
      ],
    ],
    dot: [50, 88, 5],
  ),
  // Long swooping tail.
  QuestionMarkShape(
    [
      [24, 28, 30, 6, 52, 8, 76, 12, 70, 32, 62, 48, 50, 56, 40, 64, 46, 78],
    ],
    dot: [47, 92, 4.5],
    weight: 8,
  ),
  // Kinked, a bit angular.
  QuestionMarkShape(
    [
      [26, 30, 34, 10, 50, 10, 70, 10, 70, 28, 70, 40, 52, 48, 50, 49, 50, 68],
    ],
    dot: [50, 86, 6],
    weight: 10,
  ),
  // Sketched twice, slightly off.
  QuestionMarkShape(
    [
      [28, 32, 26, 8, 50, 8, 74, 8, 72, 30, 70, 46, 54, 54, 48, 58, 49, 72],
      [31, 29, 31, 10, 52, 10, 72, 12, 70, 30, 66, 46, 52, 56, 50, 60, 51, 70],
    ],
    dot: [51, 87, 5],
    weight: 6,
  ),
  // Leaning left.
  QuestionMarkShape(
    [
      [30, 34, 24, 10, 46, 7, 70, 6, 66, 28, 62, 44, 46, 54, 40, 60, 42, 74],
    ],
    dot: [42, 89, 5.5],
  ),
  // Small tight hook on a long stem.
  QuestionMarkShape(
    [
      [34, 24, 36, 8, 50, 8, 64, 8, 64, 22, 64, 34, 52, 40, 48, 44, 49, 72],
    ],
    dot: [50, 88, 5],
  ),
  // Tail ending in a little foot.
  QuestionMarkShape(
    [
      [
        24, 30, 26, 8, 50, 8, 74, 8, 72, 28, 70, 44, 54, 52, //
        46, 56, 48, 66, 50, 72, 56, 70,
      ],
    ],
    dot: [50, 86, 5],
  ),
  // The dot is a quick dash.
  QuestionMarkShape([
    [28, 32, 26, 8, 50, 8, 74, 8, 72, 30, 70, 46, 54, 54, 48, 58, 49, 70],
    [48, 84, 50, 87, 51, 92],
  ]),
  // Heavy and round.
  QuestionMarkShape(
    [
      [22, 34, 18, 6, 50, 6, 80, 6, 78, 30, 76, 48, 56, 54, 50, 56, 50, 70],
    ],
    dot: [50, 87, 7],
    weight: 12,
  ),
  // Wobbly hand.
  QuestionMarkShape(
    [
      [
        26, 30, 28, 14, 40, 10, 52, 4, 62, 10, 76, 18, 70, 32, //
        66, 42, 56, 48, 48, 54, 52, 62, 54, 68, 50, 74,
      ],
    ],
    dot: [51, 89, 5],
  ),
  // Open hook starting low.
  QuestionMarkShape(
    [
      [22, 40, 22, 6, 52, 6, 80, 8, 74, 32, 66, 52, 50, 56, 48, 58, 48, 74],
    ],
    dot: [49, 88, 5],
    weight: 10,
  ),
];

/// The background's fixed seed: every Device shows the same scatter.
const questionMarkSeed = 20261024;

/// The background is an endless plane of square cells with one mark
/// each, so a mark sits at the same spot however far a page scrolls.
const questionMarkCell = 150.0;

/// Drawn height of the smallest and the largest mark.
const questionMarkMinSize = 34.0;
const questionMarkMaxSize = 74.0;

/// Largest tilt either way, in radians.
const questionMarkMaxTurn = 0.6;

/// One mark on the plane: which shape, its centre, its drawn size (the
/// side of the shape's box) and its tilt in radians.
class QuestionMark {
  final int shape;
  final double x;
  final double y;
  final double size;
  final double rotation;

  const QuestionMark({
    required this.shape,
    required this.x,
    required this.y,
    required this.size,
    required this.rotation,
  });
}

/// The marks of every cell touching the region from ([left], [top]) to
/// ([right], [bottom]), in plane coordinates. Callers that draw pad the
/// region by [questionMarkMaxSize] so marks do not pop at the edges.
List<QuestionMark> questionMarksIn(
  double left,
  double top,
  double right,
  double bottom, {
  int seed = questionMarkSeed,
}) {
  final marks = <QuestionMark>[];
  final firstCol = (left / questionMarkCell).floor();
  final lastCol = (right / questionMarkCell).floor();
  final firstRow = (top / questionMarkCell).floor();
  final lastRow = (bottom / questionMarkCell).floor();
  for (var row = firstRow; row <= lastRow; row++) {
    for (var col = firstCol; col <= lastCol; col++) {
      final next = _Mulberry32(_cellSeed(seed, col, row)).next;
      final size =
          questionMarkMinSize +
          next() * (questionMarkMaxSize - questionMarkMinSize);
      // Keep the centre far enough inside the cell that marks of
      // neighbouring cells rarely touch.
      final room = questionMarkCell - size;
      marks.add(
        QuestionMark(
          shape: (next() * questionMarkShapes.length).floor(),
          x: col * questionMarkCell + size / 2 + next() * room,
          y: row * questionMarkCell + size / 2 + next() * room,
          size: size,
          rotation: (next() * 2 - 1) * questionMarkMaxTurn,
        ),
      );
    }
  }
  return marks;
}

const _mask = 0xFFFFFFFF;

/// 32-bit multiplication with wrap-around, like JavaScript's Math.imul
/// and Rust's u32::wrapping_mul.
int _imul(int a, int b) => (a * b) & _mask;

/// Mixes the seed with a cell's column and row into one 32-bit state.
int _cellSeed(int seed, int col, int row) {
  var h = seed & _mask;
  h = _imul(h ^ (col & _mask), 0x85EBCA6B);
  h ^= h >> 13;
  h = _imul(h ^ (row & _mask), 0xC2B2AE35);
  h ^= h >> 16;
  return h;
}

/// Mulberry32: a tiny generator any language can reproduce bit for bit.
class _Mulberry32 {
  int _state;

  _Mulberry32(int seed) : _state = seed & _mask;

  /// The next number in [0, 1).
  double next() {
    _state = (_state + 0x6D2B79F5) & _mask;
    var t = _state;
    t = _imul(t ^ (t >> 15), t | 1);
    t ^= (t + _imul(t ^ (t >> 7), t | 61)) & _mask;
    return ((t ^ (t >> 14)) & _mask) / 4294967296;
  }
}
