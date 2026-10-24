import 'notes.dart';
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

  /// An Area's icon, one of [areaIcons].
  static const icon = 'icon';

  /// The Area a Topic lives in.
  static const area = 'area';
}

/// The icons an Area can carry, as keys every front end draws its own way.
const areaIcons = [
  'home',
  'work',
  'car',
  'health',
  'school',
  'kids',
  'kitchen',
  'garden',
  'shopping',
  'money',
  'pets',
  'sport',
  'travel',
  'star',
];

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

  const Topic({
    required this.id,
    required this.areaId,
    required this.name,
    this.starter,
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
    return areaById(store.batch(() => _appendArea(trimmed, starter)))!;
  }

  /// Creates a Topic named [name] inside the Area [areaId]. Only an Area
  /// holds Topics: there is no third level.
  Topic addTopic(String areaId, String name, {String? starter}) {
    if (areaById(areaId) == null) {
      throw ArgumentError.value(areaId, 'areaId', 'not an Area');
    }
    final trimmed = _name(name);
    return topicById(
      store.batch(() => _appendTopic(areaId, trimmed, starter)),
    )!;
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
    final topics = [
      for (final id in store.entitiesOfType(PlaceKeys.typeTopic))
        if (store.current(id, PlaceKeys.area) == areaId) ?topicById(id),
    ];
    topics.sort((a, b) => a.name.compareTo(b.name));
    return topics;
  }

  /// How many Notes sit in [id]; for an Area, its Topics' Notes count too.
  int notesInside(String id) => _notesInside(id).length;

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
  /// [notesGo] says: back to Unsorted or into the trash.
  void deleteArea(String areaId, NotesGo notesGo) {
    _requireArea(areaId);
    if (notesGo == NotesGo.area) {
      throw ArgumentError.value(notesGo, 'notesGo', 'an Area has no Area');
    }
    store.batch(() {
      _moveNotes(areaId, notesGo, null);
      for (final topic in topics(areaId)) {
        store.append(topic.id, PlaceKeys.type, null);
      }
      store.append(areaId, PlaceKeys.type, null);
    });
  }

  /// Plants the Starter Set entries whose slugs are in [picked]. A Topic
  /// is planted only together with its Area.
  void plantStarterSet(Set<String> picked) {
    store.batch(() {
      for (final area in starterSet) {
        if (!picked.contains(area.slug)) continue;
        final areaId = _appendArea(area.name, area.slug);
        for (final topic in area.topics) {
          if (!picked.contains(topic.slug)) continue;
          _appendTopic(areaId, topic.name, topic.slug);
        }
      }
    });
  }

  /// Whether [id] is an Area or a Topic that exists.
  bool isPlace(String id) => areaById(id) != null || topicById(id) != null;

  List<Note> _notesInside(String id) => Notes(store).inside(id);

  /// Sends the Notes inside [id] where [notesGo] says; [areaId] is where
  /// [NotesGo.area] leads.
  void _moveNotes(String id, NotesGo notesGo, String? areaId) {
    final notes = Notes(store);
    for (final note in _notesInside(id)) {
      switch (notesGo) {
        case NotesGo.unsorted:
          // Back to Unsorted, even when it spawned a Routine (CONTEXT.md).
          store.append(note.id, NoteKeys.place, null);
          if (note.spawned) store.append(note.id, NoteKeys.spawned, null);
        case NotesGo.area:
          store.append(note.id, NoteKeys.place, areaId);
        case NotesGo.trash:
          notes.trash(note.id);
      }
    }
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
