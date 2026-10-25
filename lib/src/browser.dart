import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'profile.dart';

/// Where swipe left leads in the notes browser: the last selected Area
/// and Topic (UI laws, "Navigation"). Lives as long as the app, so the
/// way back in survives leaving the browser.
class BrowserTrail {
  String? _area;
  String? _topic;

  /// The last selected Area, or null when none or it is gone.
  Area? area(Places places) {
    final id = _area;
    return id == null ? null : places.areaById(id);
  }

  /// The last selected Topic if it lives in [areaId], or null.
  Topic? topicIn(Places places, String areaId) {
    final id = _topic;
    final topic = id == null ? null : places.topicById(id);
    return topic?.areaId == areaId ? topic : null;
  }

  void choseArea(Area area) => _area = area.id;

  void choseTopic(Topic topic) {
    _area = topic.areaId;
    _topic = topic.id;
  }
}

/// Swipe right goes back one level, swipe left forward into the last
/// selected Area or Topic, when there is one.
class BrowserSwipes extends StatelessWidget {
  /// Below this horizontal speed (pixels per second) a drag is no swipe.
  static const _minSpeed = 300.0;

  final VoidCallback? onForward;
  final Widget child;

  const BrowserSwipes({super.key, required this.child, this.onForward});

  void _ended(BuildContext context, DragEndDetails details) {
    final speed = details.primaryVelocity ?? 0;
    if (speed > _minSpeed) {
      Navigator.of(context).maybePop();
    } else if (speed < -_minSpeed) {
      onForward?.call();
    }
  }

  @override
  Widget build(BuildContext context) => GestureDetector(
    // Empty space swipes too, not only the cards and rows.
    behavior: HitTestBehavior.opaque,
    onHorizontalDragEnd: (details) => _ended(context, details),
    child: child,
  );
}

/// The search field on top of a browser level, with ✕ to clear it.
class BrowserSearchField extends StatelessWidget {
  final TextEditingController controller;
  final String hint;
  final ValueChanged<String> onChanged;

  const BrowserSearchField({
    super.key,
    required this.controller,
    required this.hint,
    required this.onChanged,
  });

  void _clear() {
    controller.clear();
    onChanged('');
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
    child: TextField(
      controller: controller,
      onChanged: onChanged,
      textInputAction: TextInputAction.search,
      decoration: InputDecoration(
        hintText: hint,
        prefixIcon: const Icon(Icons.search),
        suffixIcon: controller.text.isEmpty
            ? null
            : IconButton(
                tooltip: context.t.clearSearch,
                onPressed: _clear,
                icon: const Icon(Icons.close),
              ),
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(16)),
      ),
    ),
  );
}

/// Notes as a diary of cards, each with "file into…", "merge into…"
/// and, with [onSchedule], Routine or Reminder, as far as this Device
/// may; tapping a card opens it.
List<Widget> diaryCards(
  BuildContext context,
  List<PlacedNote> shown,
  void Function(Note note) onFile,
  void Function(Note note) onMerge,
  void Function(Note note) onOpen, {
  void Function(Note note, bool reminder)? onSchedule,
}) {
  final when = DateFormat.yMd(Localizations.localeOf(context).toString())
      .add_Hm();
  return [
    for (final (:note, :area, :topic, :changes, :schedules, :author) in shown)
      NoteCard(
        note: note,
        when: when,
        area: area,
        topic: topic,
        author: author,
        // What a Member may not do to it is not offered (ADR 0013).
        onFile: changes ? () => onFile(note) : null,
        onMerge: changes ? () => onMerge(note) : null,
        onSchedule: onSchedule == null || !schedules
            ? null
            : (reminder) => onSchedule(note, reminder),
        onTap: () => onOpen(note),
      ),
  ];
}

/// The app bar buttons of a diary: order by time or by Importance, and
/// hide or show the unimportant Notes. Each says what tapping it does.
List<Widget> diaryActions(
  BuildContext context, {
  required DiaryOrder order,
  required bool hideUnimportant,
  required ValueChanged<DiaryOrder> onOrder,
  required ValueChanged<bool> onHideUnimportant,
}) {
  final t = context.t;
  final byImportance = order == DiaryOrder.importance;
  return [
    IconButton(
      tooltip: byImportance ? t.orderByTime : t.orderByImportance,
      isSelected: byImportance,
      onPressed: () =>
          onOrder(byImportance ? DiaryOrder.time : DiaryOrder.importance),
      icon: const Icon(Icons.sort),
      selectedIcon: const Icon(Icons.label_important_outline),
    ),
    IconButton(
      tooltip: hideUnimportant ? t.showUnimportant : t.hideUnimportant,
      isSelected: hideUnimportant,
      onPressed: () => onHideUnimportant(!hideUnimportant),
      icon: const Icon(Icons.visibility_outlined),
      selectedIcon: const Icon(Icons.visibility_off_outlined),
    ),
  ];
}

/// The app bar button of the person filter (#53): a menu at the button
/// with "Me", every person of People and, while it filters, "Everyone";
/// each choice puts that person in or out. It shows as selected while
/// it filters.
class PersonFilterButton extends StatelessWidget {
  final Store store;
  final PersonFilter filter;
  final ValueChanged<PersonFilter> onChanged;

  const PersonFilterButton({
    super.key,
    required this.store,
    required this.filter,
    required this.onChanged,
  });

  Future<void> _open(BuildContext button) async {
    final t = button.t;
    final box = button.findRenderObject() as RenderBox?;
    final overlay = Overlay.of(button).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final picked = await showMenu<PersonFilter>(
      context: button,
      position: RelativeRect.fromRect(
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size,
        Offset.zero & overlay.size,
      ),
      items: [
        // The heading the choices continue: "Show Notes by…" "Me".
        PopupMenuItem(enabled: false, child: Text(t.personFilter)),
        CheckedPopupMenuItem(
          key: const ValueKey('personFilter-me'),
          value: filter.toggledMe(),
          checked: filter.me,
          child: Text(t.personFilterMe),
        ),
        for (final person in store.peopleShown)
          CheckedPopupMenuItem(
            key: ValueKey('personFilter-${person.id}'),
            value: filter.toggled(person.id),
            checked: filter.people.contains(person.id),
            child: Row(
              children: [
                ProfileAvatar(profile: person, size: 24),
                const SizedBox(width: 8),
                Flexible(child: Text(profileDisplayName(t, person.name))),
              ],
            ),
          ),
        if (!filter.isEveryone)
          PopupMenuItem(
            key: const ValueKey('personFilter-everyone'),
            value: PersonFilter.everyone,
            child: Text(t.personFilterEveryone),
          ),
      ],
    );
    if (picked != null) onChanged(picked);
  }

  @override
  Widget build(BuildContext context) => Builder(
    builder: (button) => IconButton(
      key: const ValueKey('personFilter'),
      tooltip: context.t.personFilter,
      isSelected: !filter.isEveryone,
      onPressed: () => _open(button),
      icon: const Icon(Icons.people_outline),
      selectedIcon: const Icon(Icons.people),
    ),
  );
}

/// Who the person filter keeps, as chips under the search field, each
/// with ✕ to take that person out; nothing while it keeps everyone, so
/// Notes never leave the list unseen.
class PersonFilterChips extends StatelessWidget {
  final Store store;
  final PersonFilter filter;
  final ValueChanged<PersonFilter> onChanged;

  const PersonFilterChips({
    super.key,
    required this.store,
    required this.filter,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    if (filter.isEveryone) return const SizedBox.shrink();
    final t = context.t;
    final people = filter.people.toList()..sort();
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Wrap(
        spacing: 8,
        runSpacing: 4,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          Text(t.personFilterShowing),
          if (filter.me)
            InputChip(
              key: const ValueKey('personChip-me'),
              label: Text(t.personFilterMe),
              onDeleted: () => onChanged(filter.toggledMe()),
            ),
          for (final id in people)
            InputChip(
              key: ValueKey('personChip-$id'),
              avatar: ProfileAvatar(profile: store.personShown(id), size: 24),
              label: Text(profileDisplayName(t, store.personShown(id).name)),
              onDeleted: () => onChanged(filter.toggled(id)),
            ),
        ],
      ),
    );
  }
}

/// A line of text on an embossed panel, for a level with nothing to show.
class BrowserEmpty extends StatelessWidget {
  final String text;

  const BrowserEmpty({super.key, required this.text});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(24),
    child: Embossed(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(text, textAlign: TextAlign.center),
      ),
    ),
  );
}
