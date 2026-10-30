import 'entry.dart';
import 'notes.dart';
import 'profiles.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'text_boxes.dart';

/// Field keys of the ticks of boxes (ADR 0035).
abstract final class BoxKeys {
  /// One field per box, `box@<id>` = the [BoxState] it was ticked to, on
  /// the entity whose text holds the box: the Note, the Section, or the
  /// Note merged into another. The moment of a tick is its entry's date.
  static const prefix = 'box@';

  static String tick(String id) => '$prefix$id';
}

/// Ticking the boxes in the texts of Notes and Sections (ADR 0035): each
/// tick is an entry of its own on the entity whose text holds the box,
/// the later one by its date wins, and every one stays in the log. In a
/// Shared Area anyone with a role there ticks any box; anywhere else
/// only ticks the user's own Devices wrote count. A tick for an id the
/// text no longer holds counts for nothing.
class Boxes {
  final Store store;

  Boxes(this.store);

  /// The latest tick of each box id of [entity] that counts, from its
  /// [history] when given; ids the text does not hold among them.
  Map<String, BoxTick> ticks(String entity, [List<Entry>? history]) {
    final latest = <String, (Entry, BoxTick)>{};
    for (final (e, id, tick) in _counted(entity, history)) {
      final held = latest[id];
      if (held == null || Store.latestWins(e, held.$1) > 0) {
        latest[id] = (e, tick);
      }
    }
    return {for (final MapEntry(:key, :value) in latest.entries) key: value.$2};
  }

  /// Every tick of the box [box] in the text of [entity] that counts,
  /// the oldest first.
  List<BoxTick> history(String entity, String box) {
    final held = [
      for (final (e, id, tick) in _counted(entity))
        if (id == box) (e, tick),
    ]..sort((a, b) => Store.latestWins(a.$1, b.$1));
    return [for (final (_, tick) in held) tick];
  }

  /// The boxes of the text of [entity], a Note's or a Section's id, as
  /// they show now; empty when there is no such text.
  List<ShownBox> of(String entity) => _boxes(entity) ?? const [];

  /// Whether this Device may tick the boxes of the text of [entity]:
  /// wherever it holds a role, as for checking a Line.
  bool mayTick(String entity) =>
      _boxes(entity) != null && store.mayWriteIn(entity);

  /// Ticks the box [box] in the text of [entity], a Note's or a
  /// Section's id, to [state]. Nothing is written when it shows [state]
  /// already. Throws an [ArgumentError] when the text holds no such box,
  /// a [StateError] when this Device may not tick it.
  void tick(String entity, String box, BoxState state) {
    final shown = of(entity).where((b) => b.id == box).firstOrNull;
    if (shown == null) {
      throw ArgumentError.value(box, 'box', 'not a box of $entity');
    }
    if (!store.mayWriteIn(entity)) {
      throw StateError('no role in the Shared Area of $entity');
    }
    if (shown.state == state) return;
    store.append(entity, BoxKeys.tick(box), state.name);
  }

  /// The boxes of the text of [entity], or null when there is no such
  /// text.
  List<ShownBox>? _boxes(String entity) {
    final notes = Notes(store);
    return entity.startsWith('note:')
        ? notes.byId(entity)?.boxes
        : notes.section(entity)?.boxes;
  }

  /// Every tick of [entity] in the shape the apps write that counts: in
  /// a Shared Area what the roles allow, anywhere else what the user's
  /// own Devices wrote, so a tick another person's modified app slips
  /// into the log changes nothing.
  Iterable<(Entry, String, BoxTick)> _counted(
    String entity, [
    List<Entry>? history,
  ]) sync* {
    final area = store.areaOf(entity);
    final shared = area != null && store.isShared(area);
    final own = <String, bool>{};
    for (final e in history ?? store.history(entity)) {
      if (!e.field.startsWith(BoxKeys.prefix)) continue;
      final id = e.field.substring(BoxKeys.prefix.length);
      final state = BoxState.named(e.value);
      if (!TextBoxes.isId(id) || state == null) continue;
      final mine = own.putIfAbsent(
        e.device,
        () => store.writtenByOwn(e.device),
      );
      if (!shared && !mine) continue;
      yield (
        e,
        id,
        BoxTick(state: state, at: e.date, device: e.device, own: mine),
      );
    }
  }
}
