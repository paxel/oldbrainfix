import 'device_settings.dart';
import 'notes.dart';
import 'routines.dart';
import 'shared_areas.dart';
import 'store.dart';

/// Field keys of Areas and Topics.
abstract final class PlaceKeys {
  static const type = r'$type';
  static const typeArea = 'area';
  static const typeTopic = 'topic';
  static const name = 'name';

  /// The Starter Set slug a place was planted from; absent on places the
  /// user made.
  static const starter = 'starter';

  /// An Area's colour as `RRGGBB` hex.
  static const colour = 'colour';

  /// An Area's or a Topic's icon, one of [areaIcons] (obf-1156). A
  /// Topic has no colour of its own: its icon shows in its Area's.
  static const icon = 'icon';

  /// The Area a Topic lives in.
  static const area = 'area';
}

/// The icons an Area or a Topic can carry, as keys every front end draws
/// its own way, in the groups the look page shows them in (obf-1156):
/// home and family, work and study, health and sport, hobbies, getting
/// around, money and things, and the rest. A key once offered stays, so
/// every stored look still reads; new ones join a group.
const areaIconGroups = [
  [
    'home',
    'kitchen',
    'garden',
    'kids',
    'pets',
    'cleaning',
    'laundry',
    'bed',
    'repairs',
    'plants',
    'family',
  ],
  [
    'work',
    'school',
    'computer',
    'meetings',
    'mail',
    'book',
    'science',
    'idea',
    'code',
  ],
  [
    'health',
    'sport',
    'doctor',
    'medication',
    'food',
    'running',
    'cycling',
    'relax',
  ],
  ['music', 'photo', 'movie', 'games', 'art', 'crafts', 'hiking', 'football'],
  ['car', 'travel', 'bus', 'train', 'boat', 'beach', 'map', 'hotel'],
  ['shopping', 'money', 'gift', 'phone', 'key', 'bank', 'receipt', 'document'],
  ['star', 'brain', 'flag', 'event', 'party', 'coffee', 'bolt', 'sun'],
];

/// Every key of [areaIconGroups], group by group.
final List<String> areaIcons = List.unmodifiable([
  for (final group in areaIconGroups) ...group,
]);

/// The top level Notes are filed under (CONTEXT.md).
class Area {
  /// Entity id, `area:<id>`.
  final String id;

  /// The name as stored: typed by the user, or a starter's English name.
  final String name;

  /// The Starter Set slug, or null when the user made it.
  final String? starter;
  final String? colour;
  final String? icon;

  const Area({
    required this.id,
    required this.name,
    this.starter,
    this.colour,
    this.icon,
  });
}

/// The second and last level, inside an Area (CONTEXT.md).
class Topic {
  /// Entity id, `topic:<id>`.
  final String id;
  final String areaId;

  /// The name as stored: typed by the user, or a starter's English name.
  final String name;

  /// The Starter Set slug, or null when the user made it.
  final String? starter;

  /// One of [areaIcons], drawn in its Area's colour (obf-1156).
  final String? icon;

  const Topic({
    required this.id,
    required this.areaId,
    required this.name,
    this.starter,
    this.icon,
  });
}

/// One Area or Topic of the Starter Set. Names are the canonical English
/// ones; the front ends translate them at display time while the user has
/// not renamed them (Catlog's ADR 0005).
typedef StarterPlace = ({String slug, String name});

/// The Areas and Topics offered on first start.
const starterSet = <({String slug, String name, List<StarterPlace> topics})>[
  (
    slug: 'household',
    name: 'Household',
    topics: [
      (slug: 'kitchen', name: 'Kitchen'),
      (slug: 'garden', name: 'Garden'),
      (slug: 'cleaning', name: 'Cleaning'),
    ],
  ),
  (
    slug: 'work',
    name: 'Work',
    topics: [
      (slug: 'meetings', name: 'Meetings'),
      (slug: 'to-sort-out', name: 'To sort out'),
    ],
  ),
  (
    slug: 'car',
    name: 'Car',
    topics: [
      (slug: 'service', name: 'Service'),
      (slug: 'repairs', name: 'Repairs'),
    ],
  ),
  (
    slug: 'health',
    name: 'Health',
    topics: [
      (slug: 'doctor', name: 'Doctor'),
      (slug: 'medication', name: 'Medication'),
    ],
  ),
];

/// How far deleting an Area is: [moved] of the [of] Notes inside it have
/// gone where they go (obf-1093), told before the first and after each.
typedef MoveProgress = void Function(int moved, int of);

/// Whether a long job beside the window is to stop (obf-1078), asked
/// before each of its steps: deleting an Area before each Note, an Import
/// before each Note it writes. What was done before stays done.
typedef Stopping = bool Function();

/// Where a deleted place's Notes go.
enum NotesGo {
  /// Back to Unsorted.
  unsorted,

  /// Into the deleted Topic's Area.
  area,

  /// Into the trash.
  trash,
}

/// Areas and Topics: exactly two levels.
class Places {
  final Store store;

  Places(this.store);

  /// Creates an Area named [name].
  Area addArea(String name, {String? starter}) {
    final trimmed = _name(name);
    final id = store.batch(() => _appendArea(trimmed, starter));
    return areaById(id) ??
        (throw StateError('$id does not read back after it was written'));
  }

  /// Creates a Topic named [name] inside the Area [areaId]. Only an Area
  /// holds Topics: there is no third level.
  Topic addTopic(String areaId, String name, {String? starter}) {
    if (areaById(areaId) == null) {
      throw ArgumentError.value(areaId, 'areaId', 'not an Area');
    }
    if (!store.mayManage(areaId)) {
      throw StateError('not a Manager of $areaId');
    }
    final trimmed = _name(name);
    final id = store.batch(() => _appendTopic(areaId, trimmed, starter));
    return topicById(id) ??
        (throw StateError('$id does not read back after it was written'));
  }

  /// Renames the Area or Topic [id].
  void rename(String id, String name) {
    final trimmed = _name(name);
    if (areaById(id) == null && topicById(id) == null) {
      throw ArgumentError.value(id, 'id', 'not an Area or Topic');
    }
    store.append(id, PlaceKeys.name, trimmed);
  }

  /// Sets or clears the colour of the Area [areaId].
  void setColour(String areaId, String? colour) {
    _requireArea(areaId);
    store.append(areaId, PlaceKeys.colour, colour);
  }

  /// Sets or clears the icon of the Area [areaId].
  void setIcon(String areaId, String? icon) {
    _requireArea(areaId);
    if (icon != null && !areaIcons.contains(icon)) {
      throw ArgumentError.value(icon, 'icon', 'not one of areaIcons');
    }
    store.append(areaId, PlaceKeys.icon, icon);
  }

  /// Sets or clears the icon of the Topic [topicId] (obf-1156), stored as
  /// an Area's: an older app keeps it, as it writes only the fields it
  /// changes.
  void setTopicIcon(String topicId, String? icon) {
    if (topicById(topicId) == null) {
      throw ArgumentError.value(topicId, 'topicId', 'not a Topic');
    }
    if (icon != null && !areaIcons.contains(icon)) {
      throw ArgumentError.value(icon, 'icon', 'not one of areaIcons');
    }
    store.append(topicId, PlaceKeys.icon, icon);
  }

  /// The Area [id], or null when there is none.
  Area? areaById(String id) {
    final fields = store.currentFields(id);
    if (fields[PlaceKeys.type] != PlaceKeys.typeArea) return null;
    return Area(
      id: id,
      name: fields[PlaceKeys.name] ?? '',
      starter: fields[PlaceKeys.starter],
      colour: fields[PlaceKeys.colour],
      icon: fields[PlaceKeys.icon],
    );
  }

  /// The Topic [id], or null when there is none or its Area is gone.
  Topic? topicById(String id) {
    final fields = store.currentFields(id);
    if (fields[PlaceKeys.type] != PlaceKeys.typeTopic) return null;
    final areaId = fields[PlaceKeys.area];
    if (areaId == null || areaById(areaId) == null) return null;
    return Topic(
      id: id,
      areaId: areaId,
      name: fields[PlaceKeys.name] ?? '',
      starter: fields[PlaceKeys.starter],
      icon: fields[PlaceKeys.icon],
    );
  }

  /// Every Area, by stored name.
  List<Area> areas() {
    final areas = [
      for (final id in store.entitiesOfType(PlaceKeys.typeArea)) ?areaById(id),
    ];
    areas.sort((a, b) => a.name.compareTo(b.name));
    return areas;
  }

  /// The Topics of the Area [areaId], by stored name.
  List<Topic> topics(String areaId) {
    // None while the Area is gone ([topicById]); every Topic's fields
    // read at once, not one by one (obf-1069), in the order
    // [Store.entitiesOfType] lists them.
    if (areaById(areaId) == null) return [];
    final ever = store.everOfType(PlaceKeys.typeTopic);
    final fields = store.currentFieldsOf(ever);
    final topics = [
      for (final id in ever)
        if (fields[id] case final held?
            when held[PlaceKeys.type] == PlaceKeys.typeTopic &&
                held[PlaceKeys.area] == areaId)
          Topic(
            id: id,
            areaId: areaId,
            name: held[PlaceKeys.name] ?? '',
            starter: held[PlaceKeys.starter],
            icon: held[PlaceKeys.icon],
          ),
    ];
    topics.sort((a, b) => a.name.compareTo(b.name));
    return topics;
  }

  /// How many Notes sit in [id]; for an Area, its Topics' Notes count too.
  /// Told from their fields, without making a Note (obf-1093).
  int notesInside(String id) => Notes(store).countInside(id);

  /// Deletes the Topic [topicId]; its Notes go where [notesGo] says.
  void deleteTopic(String topicId, NotesGo notesGo) {
    final topic = topicById(topicId);
    if (topic == null) {
      throw ArgumentError.value(topicId, 'topicId', 'not a Topic');
    }
    store.batch(() {
      _moveNotes(topicId, notesGo, topic.areaId);
      store.append(topicId, PlaceKeys.type, null);
    });
  }

  /// Deletes the Area [areaId] with its Topics; their Notes go where
  /// [notesGo] says: back to Unsorted or into the trash. Each Note goes
  /// in a transaction of its own, then the Area with its Topics in one
  /// more (obf-1093), so another connection to the database, such as the
  /// app's while this runs beside it, waits at most for one Note;
  /// [progress] hears how many have gone.
  void deleteArea(String areaId, NotesGo notesGo, {MoveProgress? progress}) {
    deleteAreaStopping(areaId, notesGo, progress: progress);
  }

  /// As [deleteArea], asking [stop] before each Note (obf-1078): once it
  /// says so, the Notes gone stay where they went, the others and the
  /// Area with its Topics stay, and it answers false; true once the Area
  /// is deleted.
  bool deleteAreaStopping(
    String areaId,
    NotesGo notesGo, {
    MoveProgress? progress,
    Stopping? stop,
  }) {
    _requireArea(areaId);
    if (notesGo == NotesGo.area) {
      throw ArgumentError.value(notesGo, 'notesGo', 'an Area has no Area');
    }
    if (!_moveNotes(areaId, notesGo, null, progress, stop)) return false;
    store.batch(() {
      for (final topic in topics(areaId)) {
        store.append(topic.id, PlaceKeys.type, null);
      }
      store.append(areaId, PlaceKeys.type, null);
    });
    return true;
  }

  /// Plants the Starter Set entries whose slugs are in [picked]. A Topic
  /// is planted only together with its Area.
  ///
  /// Every Device plants a starter Area or Topic under the same id
  /// ([starterId]) and dated [starterDate]: two Devices that both planted
  /// it hold one place after a sync, not two, and whatever the user did
  /// to it on any Device (a rename, a colour, deleting it) is dated later
  /// and wins over any planting.
  void plantStarterSet(Set<String> picked) {
    store.batch(() {
      for (final area in starterSet) {
        if (!picked.contains(area.slug)) continue;
        final areaId = starterId(PlaceKeys.typeArea, area.slug);
        _plant(areaId, PlaceKeys.typeArea, area.name, area.slug);
        for (final topic in area.topics) {
          if (!picked.contains(topic.slug)) continue;
          final topicId = starterId(PlaceKeys.typeTopic, topic.slug);
          if (_plant(topicId, PlaceKeys.typeTopic, topic.name, topic.slug)) {
            store.append(topicId, PlaceKeys.area, areaId, date: starterDate);
          }
        }
      }
    });
  }

  /// Whether the Starter Set is still to be offered: this Device never
  /// offered it, and no Areas arrived from another Device meanwhile.
  bool get starterSetDue => !store.starterSetOffered && areas().isEmpty;

  /// The id every Device plants the starter place [slug] of [type] under.
  static String starterId(String type, String slug) => '$type:starter-$slug';

  /// The date of every planting: before anything a user does.
  static final starterDate = DateTime.utc(2000);

  /// Writes the starter place [id] unless this Device holds it already,
  /// planted here or arrived by sync; returns whether it wrote it.
  bool _plant(String id, String type, String name, String slug) {
    if (store.history(id).isNotEmpty) return false;
    store.append(id, PlaceKeys.type, type, date: starterDate);
    store.append(id, PlaceKeys.name, name, date: starterDate);
    store.append(id, PlaceKeys.starter, slug, date: starterDate);
    return true;
  }

  /// Whether [id] is an Area or a Topic that exists.
  bool isPlace(String id) => areaById(id) != null || topicById(id) != null;

  /// Every Area and Topic that exists, by id: those [isPlace] holds for,
  /// read at once for a list of many Notes (obf-1069).
  Set<String> ids() {
    final areas = store.currentFieldsOfType(PlaceKeys.typeArea).keys.toSet();
    return {
      ...areas,
      for (final MapEntry(key: id, value: fields)
          in store.currentFieldsOfType(PlaceKeys.typeTopic).entries)
        if (areas.contains(fields[PlaceKeys.area])) id,
    };
  }

  List<Note> _notesInside(String id) => Notes(store).inside(id);

  /// Sends the Notes inside [id] where [notesGo] says, each Note in a
  /// transaction of its own unless a batch holds them all; [areaId] is
  /// where [NotesGo.area] leads. [progress] hears how many have gone;
  /// [stop] is asked before each, and false is answered once it said so.
  bool _moveNotes(
    String id,
    NotesGo notesGo,
    String? areaId, [
    MoveProgress? progress,
    Stopping? stop,
  ]) {
    final notes = _notesInside(id);
    if (notesGo == NotesGo.trash) {
      // With their Routines and Reminders, as every deletion (#262), those
      // read once for all of them (obf-1093).
      return Routines(store)
          .trashNotesStopping(notes, progress: progress, stop: stop);
    }
    progress?.call(0, notes.length);
    for (final (i, note) in notes.indexed) {
      if (stop?.call() ?? false) return false;
      store.batch(() {
        if (notesGo == NotesGo.area) {
          store.append(note.id, NoteKeys.place, areaId);
          return;
        }
        // Back to Unsorted, even when it spawned a Routine (CONTEXT.md).
        store.append(note.id, NoteKeys.place, null);
        if (note.spawned) store.append(note.id, NoteKeys.spawned, null);
      });
      progress?.call(i + 1, notes.length);
    }
    return true;
  }

  // The writes behind addArea and addTopic, for use inside one batch.
  String _appendArea(String name, String? starter) {
    final id = 'area:${Store.newId()}';
    store.append(id, PlaceKeys.type, PlaceKeys.typeArea);
    store.append(id, PlaceKeys.name, name);
    if (starter != null) store.append(id, PlaceKeys.starter, starter);
    return id;
  }

  String _appendTopic(String areaId, String name, String? starter) {
    final id = 'topic:${Store.newId()}';
    store.append(id, PlaceKeys.type, PlaceKeys.typeTopic);
    store.append(id, PlaceKeys.area, areaId);
    store.append(id, PlaceKeys.name, name);
    if (starter != null) store.append(id, PlaceKeys.starter, starter);
    return id;
  }

  void _requireArea(String id) {
    if (areaById(id) == null) {
      throw ArgumentError.value(id, 'id', 'not an Area');
    }
  }

  static String _name(String name) {
    final trimmed = name.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(name, 'name', 'must not be empty');
    }
    return trimmed;
  }
}
