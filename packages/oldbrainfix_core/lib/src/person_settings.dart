import 'dart:convert';

import 'places.dart';
import 'profiles.dart';
import 'store.dart';

/// Where the person's own settings lie in the log (#211).
abstract final class PersonSettingKeys {
  /// The entity holding them, outside every Area like an Override: sealed
  /// under the personal key, it goes to the user's own Devices only, and
  /// only what they wrote counts.
  static const entity = 'settings:person';

  /// "Move a Note to the Trash when its one-time Reminder is done":
  /// [on] while it is on; absent, null or any other value for off.
  static const trashDoneReminder = 'trashDoneReminder';

  static const on = 'on';

  /// Simple or Advanced (#197): [simple] or [advanced] once chosen;
  /// absent, null or any other value reads as Advanced, so nothing is
  /// hidden from someone who never chose.
  static const features = 'features';

  static const simple = 'simple';
  static const advanced = 'advanced';

  /// The person's own order of the Areas (#280): a JSON list of Area ids,
  /// first to last. Ids that are no Area here, repeats and anything not a
  /// string are skipped; absent, null or anything not a list reads as no
  /// order, so the Areas show by name.
  static const areaOrder = 'areaOrder';
}

/// The settings of the person rather than of this Device (#211): kept in
/// the log like an Override, so they are the same on all of the user's
/// own Devices and never reach another person. What another person's
/// Device wrote counts for nothing. The per-Device ones are
/// [DeviceSettings].
extension PersonSettings on Store {
  /// Whether Done on a one-time Reminder that is the only Routine or
  /// Reminder of a plain Note moves the Note to the trash with it
  /// ([Routines.checkOff]). Off until switched on; a value no app writes
  /// reads as off, so nothing is trashed by it.
  bool get trashesDoneReminder =>
      currentByOwn(
        PersonSettingKeys.entity,
        PersonSettingKeys.trashDoneReminder,
      ) ==
      PersonSettingKeys.on;

  /// Switches it on or off for all of the user's own Devices; the same
  /// again writes nothing.
  set trashesDoneReminder(bool on) {
    if (trashesDoneReminder == on) return;
    append(
      PersonSettingKeys.entity,
      PersonSettingKeys.trashDoneReminder,
      on ? PersonSettingKeys.on : null,
    );
  }

  /// Whether making Tallies, Sessions, Frogs and Topics, the format bar
  /// and Shared Area roles are offered (#197). Only [PersonSettingKeys.simple]
  /// hides them; never chosen, or a value no app writes, reads as
  /// Advanced. What exists stays visible and usable either way.
  bool get advanced =>
      currentByOwn(PersonSettingKeys.entity, PersonSettingKeys.features) !=
      PersonSettingKeys.simple;

  /// Chooses Simple or Advanced for all of the user's own Devices; the
  /// same value stored again writes nothing, while Advanced chosen where
  /// none was stored is written, so the choice is recorded.
  set advanced(bool on) {
    final value = on ? PersonSettingKeys.advanced : PersonSettingKeys.simple;
    if (currentByOwn(PersonSettingKeys.entity, PersonSettingKeys.features) ==
        value) {
      return;
    }
    append(PersonSettingKeys.entity, PersonSettingKeys.features, value);
  }

  /// Every Area in the person's own order (#280), as the Areas list and
  /// the sidebar show them: those in the stored order first, as it has
  /// them, then the Areas missing from it (a new one, a Shared Area joined
  /// later) by the name [shown] gives each, the one the app shows, in
  /// lower case; equal names go by id. Without [shown], by stored name.
  /// With no order stored, all of them by that name.
  List<Area> areasInOrder({String Function(Area area)? shown}) {
    final byId = {for (final area in Places(this).areas()) area.id: area};
    final ordered = <Area>[];
    for (final id in _heldAreaOrder()) {
      if (byId.remove(id) case final area?) ordered.add(area);
    }
    return [...ordered, ..._byShownName(byId.values, shown)];
  }

  /// Puts the Area [areaId] at [index] of [areasInOrder] by [shown],
  /// counted after it was taken out, for all of the user's own Devices.
  /// An index past either end puts it first or last. The whole order is
  /// stored, so a new Area goes to the end. Throws when [areaId] is no
  /// Area.
  void moveArea(String areaId, int index, {String Function(Area area)? shown}) {
    final ids = _shownAreaIds(shown);
    if (!ids.remove(areaId)) {
      throw ArgumentError.value(areaId, 'areaId', 'not an Area');
    }
    ids.insert(index.clamp(0, ids.length), areaId);
    _storeAreaOrder(ids, shown);
  }

  /// Puts the Areas in the order of [areaIds] for all of the user's own
  /// Devices; the Areas it leaves out follow in the order
  /// [areasInOrder] by [shown] had them. Ids that are no Area are
  /// skipped.
  void setAreaOrder(List<String> areaIds, {String Function(Area area)? shown}) {
    final had = _shownAreaIds(shown);
    final named = {...areaIds.where(had.contains)};
    _storeAreaOrder([
      ...named,
      ...had.where((id) => !named.contains(id)),
    ], shown);
  }

  /// Puts the Area [to] where [from] stands in the stored order, as a
  /// starter Area shared under a new id (#280). Nothing when [from] is
  /// not in it: [to] is then missing from it too, in the same place by
  /// name.
  void putAreaInPlaceOf(String from, String to) {
    final held = _heldAreaOrder();
    if (!held.contains(from)) return;
    final areas = {for (final area in Places(this).areas()) area.id};
    final ids = {
      for (final id in held)
        if (id == from) to else id,
    }.where(areas.contains);
    append(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
      jsonEncode([...ids]),
    );
  }

  List<String> _shownAreaIds(String Function(Area area)? shown) => [
    for (final area in areasInOrder(shown: shown)) area.id,
  ];

  /// Stores [ids] as the order; the order already shown by [shown]
  /// writes nothing.
  void _storeAreaOrder(List<String> ids, String Function(Area area)? shown) {
    if (jsonEncode(ids) == jsonEncode(_shownAreaIds(shown))) return;
    append(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
      jsonEncode(ids),
    );
  }

  /// The ids of the stored order, as written by the user's own Devices,
  /// the newest such entry winning; empty when there is none or it is
  /// not a list.
  List<String> _heldAreaOrder() {
    final value = currentByOwn(
      PersonSettingKeys.entity,
      PersonSettingKeys.areaOrder,
    );
    if (value == null) return const [];
    final Object? list;
    try {
      list = jsonDecode(value);
    } on FormatException {
      return const [];
    }
    if (list is! List) return const [];
    return [...list.whereType<String>()];
  }
}

/// [areas] by the name [shown] gives each, stored names without it, in
/// lower case as `toLowerCase` has it and compared by UTF-16 code units,
/// as the apps listed the Areas before #280; equal names go by id, so the
/// order is the same every time and in both cores.
List<Area> _byShownName(
  Iterable<Area> areas,
  String Function(Area area)? shown,
) {
  final keyed = [
    for (final area in areas)
      (area, (shown?.call(area) ?? area.name).toLowerCase()),
  ];
  keyed.sort((a, b) {
    final byName = a.$2.compareTo(b.$2);
    return byName != 0 ? byName : a.$1.id.compareTo(b.$1.id);
  });
  return [for (final (area, _) in keyed) area];
}
