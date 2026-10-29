import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'area_look.dart';
import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'system_bars.dart';

/// The frog that marks Frogs, silent: there is none among the Material
/// icons.
class FrogIcon extends StatelessWidget {
  final double size;

  const FrogIcon({super.key, this.size = 22});

  @override
  Widget build(BuildContext context) => ExcludeSemantics(
    child: Text(
      '\u{1F438}',
      // The emoji font by its name as well, where the text's own font
      // has no frog and nothing falls back on its own (the screenshots).
      style: TextStyle(
        fontSize: size,
        height: 1,
        fontFamilyFallback: const ['Noto Color Emoji'],
      ),
    ),
  );
}

/// A Frog with its Note and the Area that Note is filed in, read once
/// per change.
typedef PlacedFrog = ({Frog frog, Note note, Area? area});

/// [frog] with its Note and Area; null when its Note is gone for good.
PlacedFrog? placeFrog(Frogs frogs, Places places, Frog frog) {
  final note = frogs.note(frog);
  if (note == null) return null;
  final place = note.place;
  final topic = place == null ? null : places.topicById(place);
  final area = place == null ? null : places.areaById(topic?.areaId ?? place);
  return (frog: frog, note: note, area: area);
}

/// Opens the Frog view.
Future<void> openFrogs(BuildContext context, Notes notes, Places places) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => FrogsScreen(notes: notes, places: places),
      ),
    );

/// A long press on a Frog's row opens a menu where the finger is, with
/// Delete for who may manage it (ADR 0013), which asks once and moves
/// the Frog to the trash; its Note stays. Nothing opens for others.
Future<void> frogMenu(
  BuildContext context,
  Frogs frogs,
  Frog frog,
  Offset at,
) async {
  final store = frogs.store;
  if (!store.mayManage(store.areaOf(frog.id))) return;
  final t = context.t;
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
  if (overlay == null) return;
  final choice = await showMenu<bool>(
    context: context,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: [PopupMenuItem(value: true, child: Text(t.delete))],
  );
  if (choice != true || !context.mounted) return;
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      content: Text(t.deleteFrogQuestion),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        TextButton(
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.delete),
        ),
      ],
    ),
  );
  if (sure != true) return;
  frogs.trash(frog.id);
}

/// One Frog as an embossed row: the box to check it off, its Note's
/// text, struck through when Done, and its Area's colour and icon. Under
/// the text an optional [subtitle] and, with [onAnother], "Another one".
/// Tapping the row opens its Note; a long press opens [onMenu] where the
/// finger is.
class FrogRow extends StatelessWidget {
  final Frog frog;
  final Area? area;

  /// The text shown: the Note's, or the masked one.
  final String title;
  final String? subtitle;

  /// Checks it off or unchecks it; the box is disabled when null.
  final VoidCallback? onToggle;
  final VoidCallback onTap;
  final void Function(Offset at) onMenu;

  /// Swaps it for the next Frog; no button when null.
  final VoidCallback? onAnother;

  const FrogRow({
    super.key,
    required this.frog,
    required this.area,
    required this.title,
    this.subtitle,
    required this.onToggle,
    required this.onTap,
    required this.onMenu,
    this.onAnother,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final colour = areaColour(area?.colour);
    final done = frog.isDone;
    final subtitle = this.subtitle;
    final onAnother = this.onAnother;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: GestureDetector(
              onLongPressStart: (details) => onMenu(details.globalPosition),
              child: InkWell(
                onTap: onTap,
                child: Container(
                  decoration: colour == null
                      ? null
                      : BoxDecoration(
                          border: Border(
                            left: BorderSide(color: colour, width: 6),
                          ),
                        ),
                  padding: const EdgeInsets.fromLTRB(4, 4, 16, 4),
                  child: Row(
                    children: [
                      Checkbox(
                        value: done,
                        semanticLabel: t.markDone,
                        onChanged: switch (onToggle) {
                          null => null,
                          final onToggle => (_) => onToggle(),
                        },
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              title,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: text.bodyLarge?.copyWith(
                                decoration: done
                                    ? TextDecoration.lineThrough
                                    : null,
                              ),
                            ),
                            if (subtitle != null)
                              Text(subtitle, style: text.bodySmall),
                            // The control goes below its text.
                            if (onAnother != null)
                              TextButton.icon(
                                onPressed: onAnother,
                                icon: const Icon(Icons.swap_horiz),
                                label: Text(t.anotherFrog),
                              ),
                          ],
                        ),
                      ),
                      if (areaIconData(area?.icon) case final icon?)
                        Padding(
                          padding: const EdgeInsets.only(left: 8),
                          child: Icon(icon, size: 20, color: colour),
                        ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// The Frog view: the open Frogs, oldest first, each with a box to check
/// it off; below, a folded "Done" section, closed until opened, with
/// those checked off, most recently first, each with the date, whose box
/// unchecks it. Whether it is open is a setting of this Device. Tapping
/// a row opens its Note; a long press offers Delete. Frogs of closed
/// locked Areas stay out.
class FrogsScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const FrogsScreen({super.key, required this.notes, required this.places});

  @override
  State<FrogsScreen> createState() => _FrogsScreenState();
}

class _FrogsScreenState extends State<FrogsScreen> with ReloadsOnLogChange {
  /// The local setting that keeps the "Done" section open.
  static const _doneOpenKey = 'frogsDoneOpen';

  late final _frogs = Frogs(widget.notes.store);
  late List<PlacedFrog> _open;
  late List<PlacedFrog> _done;
  late bool _doneOpen = widget.notes.store.localSetting(_doneOpenKey) == 'true';

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _load();
  }

  void _load() {
    final lock = LockScope.of(context);
    List<PlacedFrog> placed(List<Frog> frogs) => [
      for (final frog in frogs)
        if (placeFrog(_frogs, widget.places, frog) case final placed?)
          if (!(lock?.hides(placed.note) ?? false)) placed,
    ];
    _open = placed(_frogs.open());
    _done = placed(_frogs.done());
  }

  void _setDoneOpen(bool open) {
    _doneOpen = open;
    widget.notes.store.setLocalSetting(_doneOpenKey, '$open');
  }

  void _toggle(Frog frog) {
    if (frog.isDone) {
      _frogs.unmarkDone(frog.id);
    } else {
      _frogs.markDone(frog.id);
    }
    setState(_load);
  }

  Future<void> _openNote(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    setState(_load);
  }

  Widget _row(PlacedFrog placed, {String? subtitle}) {
    final (:frog, :note, :area) = placed;
    return FrogRow(
      key: ValueKey('frog-${frog.id}'),
      frog: frog,
      area: area,
      title: noteText(context.t, note),
      subtitle: subtitle,
      onToggle: widget.notes.store.mayCheckOff(frog.id)
          ? () => _toggle(frog)
          : null,
      onTap: () => _openNote(note),
      onMenu: (at) => frogMenu(context, _frogs, frog, at),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final date = DateFormat.yMd(Localizations.localeOf(context).toString());
    return Scaffold(
      appBar: AppBar(title: Text(t.frogsTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          if (_open.isEmpty)
            BrowserEmpty(text: t.frogsEmpty)
          else
            for (final placed in _open) _row(placed),
          if (_done.isNotEmpty)
            ExpansionTile(
              key: const ValueKey('frogsDone'),
              shape: const Border(),
              collapsedShape: const Border(),
              leading: const Icon(Icons.check_circle_outline),
              title: Text(t.frogsDone),
              initiallyExpanded: _doneOpen,
              onExpansionChanged: _setDoneOpen,
              children: [
                for (final placed in _done)
                  _row(
                    placed,
                    subtitle: switch (placed.frog.done) {
                      final done? => t.frogDoneOn(date.format(done.toLocal())),
                      null => null,
                    },
                  ),
              ],
            ),
        ],
      ),
    );
  }
}
