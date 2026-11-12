import 'area_keys.dart';
import 'checklists.dart';
import 'entry.dart';
import 'frogs.dart';
import 'notes.dart';
import 'places.dart';
import 'profiles.dart';
import 'routines.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tallies.dart';

/// Where the marks "Hidden from AI" lie in the log (#267, ADR 0049).
abstract final class HiddenFromAiKeys {
  /// The id prefix, before the id of the marked Note or Area:
  /// `aiHidden:<id>`, outside every Area like an Override, so it is
  /// sealed under the personal key, goes only to the user's own Devices,
  /// and only what they wrote counts.
  static const prefix = 'aiHidden:';

  /// The entity holding the mark of the Note or Area [id].
  static String of(String id) => '$prefix$id';

  /// On that entity: [on] while the mark is set; absent, null or any
  /// other value for none.
  static const hidden = 'hidden';

  static const on = 'on';
}

/// Sharing something Hidden from AI (#267): filing a marked Note into a
/// Shared Area, sharing a hidden Area or one holding hidden Notes,
/// sending a hidden Note as a Sent Item, or putting one into a Pack.
/// Nothing changed; it goes once the user takes the marks off.
class HiddenFromAiRefused implements Exception {
  /// The Notes and Areas whose marks stand in the way, sorted.
  final List<String> marks;

  const HiddenFromAiRefused(this.marks);

  @override
  String toString() => 'HiddenFromAiRefused(${marks.join(', ')})';
}

/// The mark "Hidden from AI" on a Note or an Area (#267, ADR 0049): for
/// the AI such a Note, a Note in such an Area, and everything on them do
/// not exist. Hidden and shared exclude each other. Decided from the log
/// alone, by the marks the user's own Devices wrote, so a pending change
/// hides or shows nothing.
extension HiddenFromAi on Store {
  /// Whether the Note or Area [id] carries the mark itself.
  bool isMarkedHiddenFromAi(String id) =>
      currentByOwn(HiddenFromAiKeys.of(id), HiddenFromAiKeys.hidden) ==
      HiddenFromAiKeys.on;

  /// The ids of every Note and Area carrying the mark.
  Set<String> hiddenFromAiMarks() {
    final own = <String, bool>{};
    return {
      for (final entity in entitiesWithPrefix(HiddenFromAiKeys.prefix))
        if (currentByOwn(entity, HiddenFromAiKeys.hidden, own) ==
            HiddenFromAiKeys.on)
          entity.substring(HiddenFromAiKeys.prefix.length),
    };
  }

  /// Whether the mark may be set on [id], as the apps offer it: an Area
  /// that is not shared, or a Note in no Shared Area.
  bool mayHideFromAi(String id) {
    if (Places(this).areaById(id) != null) return !isShared(id);
    if (!id.startsWith('note:') || Notes(this).byId(id) == null) return false;
    final area = areaOf(id);
    return area == null || !isShared(area);
  }

  /// Sets the mark on the Note or Area [id], or takes it off, for all of
  /// the user's own Devices; the same again writes nothing. Taking it off
  /// is never refused. Setting it throws an [ArgumentError] for an id
  /// that is no Note or Area, and a [StateError] in a Shared Area.
  void setHiddenFromAi(String id, bool hidden) {
    if (isMarkedHiddenFromAi(id) == hidden) return;
    if (hidden && !mayHideFromAi(id)) {
      final known =
          Places(this).areaById(id) != null ||
          (id.startsWith('note:') && Notes(this).byId(id) != null);
      if (!known) throw ArgumentError.value(id, 'id', 'not a Note or Area');
      throw StateError('$id is in a Shared Area');
    }
    append(
      HiddenFromAiKeys.of(id),
      HiddenFromAiKeys.hidden,
      hidden ? HiddenFromAiKeys.on : null,
    );
  }

  /// The marked Notes and Areas that hide [entity] from the AI, sorted:
  /// its own mark, its Area's through its Topic, those of the Note it is
  /// merged into, and for a Section, Routine, Frog, Tally entry, Line or
  /// an own choice on a Note, those of the Note. Each link is followed as
  /// the log has it and as the user's own Devices last wrote it, so an
  /// entry another person's Device slipped in shows nothing. Empty when
  /// it is not hidden. [marks] are those of [hiddenFromAiMarks], when
  /// read already.
  List<String> hiddenFromAiBy(String entity, [Set<String>? marks]) {
    marks ??= hiddenFromAiMarks();
    if (marks.isEmpty) return const [];
    final found = <String>{};
    final seen = <String>{};
    final own = <String, bool>{};
    final todo = [entity];
    while (todo.isNotEmpty) {
      final id = todo.removeLast();
      if (!seen.add(id)) continue;
      if (marks.contains(id) &&
          (id.startsWith('note:') || id.startsWith('area:'))) {
        found.add(id);
      }
      todo.addAll(_aboveForAi(id, own));
    }
    return found.toList()..sort();
  }

  /// Whether [entity] does not exist for the AI ([hiddenFromAiBy]).
  bool isHiddenFromAi(String entity) => hiddenFromAiBy(entity).isNotEmpty;

  /// The marks that keep the Area [areaId] from being shared, sorted: its
  /// own, and those hiding a Note filed in it or one of its Topics,
  /// trashed and merged ones too, as their history goes along.
  List<String> hiddenFromAiIn(String areaId) {
    final marks = hiddenFromAiMarks();
    if (marks.isEmpty) return const [];
    final found = <String>{if (marks.contains(areaId)) areaId};
    for (final note in entitiesOfType(NoteKeys.typeNote)) {
      if (scopeOf(note) == areaId) found.addAll(hiddenFromAiBy(note, marks));
    }
    return found.toList()..sort();
  }

  /// Throws [HiddenFromAiRefused] for [marks], when there are any.
  void refuseHiddenFromAi(List<String> marks) {
    if (marks.isNotEmpty) throw HiddenFromAiRefused(marks);
  }

  /// What [id] lies in or under, as the log alone has it and as the
  /// user's own Devices last wrote it ([own] remembers which Devices are
  /// theirs): an Area for a Topic; the Area or Topic and the Note it is
  /// merged into for a Note; the Note for what is on one; the Note or
  /// Area a mark or an own choice is about. Only ids of the kind expected
  /// count.
  List<String> _aboveForAi(String id, Map<String, bool> own) {
    List<String> held(String field, List<String> kinds) {
      Entry? latest;
      Entry? mine;
      for (final e in loggedHistory(id)) {
        if (e.field != field) continue;
        if (latest == null || Store.latestWins(e, latest) > 0) latest = e;
        if (!own.putIfAbsent(e.device, () => writtenByOwn(e.device))) continue;
        if (mine == null || Store.latestWins(e, mine) > 0) mine = e;
      }
      return {
        for (final e in [?latest, ?mine])
          if (e.value case final value? when kinds.any(value.startsWith)) value,
      }.toList();
    }

    String? after(String prefix, List<String> kinds) {
      final rest = id.substring(prefix.length);
      return kinds.any(rest.startsWith) ? rest : null;
    }

    if (id.startsWith('topic:')) return held(PlaceKeys.area, const ['area:']);
    if (id.startsWith('note:')) {
      return [
        ...held(NoteKeys.place, const ['area:', 'topic:']),
        ...held(NoteKeys.merged, const ['note:']),
      ];
    }
    const note = ['note:'];
    if (id.startsWith('section:')) return held(SectionKeys.note, note);
    if (id.startsWith('routine:')) return held(RoutineKeys.note, note);
    if (id.startsWith(FrogKeys.prefix)) return held(FrogKeys.note, note);
    if (id.startsWith(TallyKeys.entryPrefix)) {
      return held(TallyKeys.note, note);
    }
    if (id.startsWith(ChecklistKeys.linePrefix)) {
      return held(ChecklistKeys.note, note);
    }
    final String? one;
    if (id.startsWith(ChecklistKeys.agendaPrefix)) {
      one = after(ChecklistKeys.agendaPrefix, note);
    } else if (id.startsWith(HiddenFromAiKeys.prefix)) {
      one = after(HiddenFromAiKeys.prefix, const ['note:', 'area:']);
    } else {
      one = null;
    }
    return [?one];
  }
}
