import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('there are 10 to 20 distinct hand-drawn shapes', () {
    expect(questionMarkShapes.length, inInclusiveRange(10, 20));
    final drawn = {
      for (final s in questionMarkShapes) '${s.strokes}|${s.dot}|${s.weight}',
    };
    expect(drawn, hasLength(questionMarkShapes.length));
  });

  test('every stroke is a start point plus whole quadratic segments', () {
    for (final shape in questionMarkShapes) {
      expect(shape.strokes, isNotEmpty);
      for (final stroke in shape.strokes) {
        expect(stroke.length, greaterThanOrEqualTo(6));
        expect((stroke.length - 2) % 4, 0);
        for (final v in stroke) {
          expect(v, inInclusiveRange(0, questionMarkBox));
        }
      }
    }
  });

  test('the same seed scatters the same marks', () {
    final a = questionMarksIn(0, 0, 1000, 2000);
    final b = questionMarksIn(0, 0, 1000, 2000);
    expect(a.map(_describe), b.map(_describe));
    expect(a, isNotEmpty);
  });

  test('a different seed scatters differently', () {
    final a = questionMarksIn(0, 0, 1000, 2000);
    final b = questionMarksIn(0, 0, 1000, 2000, seed: questionMarkSeed + 1);
    expect(a.map(_describe), isNot(b.map(_describe)));
  });

  test('a mark stays where it is whatever region is asked for', () {
    final whole = questionMarksIn(0, 0, 1000, 2000).map(_describe).toSet();
    final part = questionMarksIn(300, 500, 600, 900).map(_describe);
    expect(whole.containsAll(part), isTrue);
    expect(part, isNotEmpty);
  });

  test('size and rotation vary within their bounds', () {
    final marks = questionMarksIn(0, 0, 1000, 3000);
    final sizes = marks.map((m) => m.size).toSet();
    final turns = marks.map((m) => m.rotation).toSet();
    expect(sizes.length, greaterThan(marks.length ~/ 2));
    expect(turns.length, greaterThan(marks.length ~/ 2));
    for (final m in marks) {
      expect(
        m.size,
        inInclusiveRange(questionMarkMinSize, questionMarkMaxSize),
      );
      expect(m.rotation.abs(), lessThanOrEqualTo(questionMarkMaxTurn));
    }
    expect(sizes.reduce((a, b) => a < b ? a : b), lessThan(45));
    expect(sizes.reduce((a, b) => a > b ? a : b), greaterThan(60));
    expect(turns.any((r) => r < -0.3) && turns.any((r) => r > 0.3), isTrue);
  });

  test('a phone screen worth of background shows many different shapes', () {
    final marks = questionMarksIn(0, 0, 400, 1600);
    expect(marks.map((m) => m.shape).toSet().length, greaterThanOrEqualTo(10));
  });

  test('marks lie in the cells touching the region asked for', () {
    for (final m in questionMarksIn(-200, 400, 300, 900)) {
      expect(
        m.x,
        inInclusiveRange(-200 - questionMarkCell, 300 + questionMarkCell),
      );
      expect(
        m.y,
        inInclusiveRange(400 - questionMarkCell, 900 + questionMarkCell),
      );
    }
  });
}

String _describe(QuestionMark m) =>
    '${m.shape}@${m.x},${m.y} ${m.size} ${m.rotation}';
