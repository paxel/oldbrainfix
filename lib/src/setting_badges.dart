import 'package:flutter/material.dart';

import 'l10n.dart';

/// One setting of a Note in the editor's row of badges (#160), drawn like
/// a shield: the setting's icon, then its current value. A screen reader
/// reads it as "setting: value". [onTap] gets where the finger was,
/// so a menu can open there; without it the badge only shows the value.
class SettingBadge extends StatefulWidget {
  /// The setting's name in words, for the screen reader.
  final String setting;

  /// The current value; none for a badge that only adds, such as "+".
  final String? value;

  /// The setting's icon, the same in the badge and its editor; [iconWidget]
  /// draws one that is no [IconData], such as the frog.
  final IconData? icon;
  final Widget? iconWidget;

  /// The icon's colour, such as an Area's; the accent otherwise.
  final Color? iconColour;
  final void Function(Offset at)? onTap;

  const SettingBadge({
    super.key,
    required this.setting,
    required this.value,
    this.icon,
    this.iconWidget,
    this.iconColour,
    this.onTap,
  });

  @override
  State<SettingBadge> createState() => _SettingBadgeState();
}

class _SettingBadgeState extends State<SettingBadge> {
  /// Where the finger went down last.
  Offset? _at;

  void _tapped() {
    final onTap = widget.onTap;
    if (onTap == null) return;
    // A screen reader's tap has no finger: the badge's middle then.
    final box = context.findRenderObject() as RenderBox?;
    final at =
        _at ??
        (box == null
            ? Offset.zero
            : box.localToGlobal(box.size.center(Offset.zero)));
    _at = null;
    onTap(at);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final value = widget.value;
    final shape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(8),
      side: BorderSide(color: scheme.outlineVariant),
    );
    final icon =
        widget.iconWidget ??
        Icon(widget.icon, size: 18, color: widget.iconColour ?? scheme.primary);
    return Semantics(
      button: widget.onTap != null,
      label: value == null
          ? widget.setting
          : context.t.settingBadgeLabel(widget.setting, value),
      excludeSemantics: true,
      onTap: widget.onTap == null ? null : _tapped,
      child: Material(
        color: scheme.surface,
        shape: shape,
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTapDown: widget.onTap == null
              ? null
              : (details) => _at = details.globalPosition,
          onTap: widget.onTap == null ? null : _tapped,
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 40),
            child: IntrinsicHeight(
              child: Row(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // The icon's part, tinted as a shield's label is.
                  ColoredBox(
                    color: scheme.secondaryContainer,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      child: Center(child: icon),
                    ),
                  ),
                  if (value != null)
                    Flexible(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 10,
                          vertical: 8,
                        ),
                        child: Center(
                          widthFactor: 1,
                          child: Text(value, style: theme.textTheme.labelLarge),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// The small editor a badge opens (#160): the setting's icon and its name
/// in words as the title, then [content], then [actions] and Close.
class BadgeDialog extends StatelessWidget {
  final String setting;
  final IconData? icon;
  final Widget? iconWidget;
  final Color? iconColour;
  final Widget content;
  final List<Widget> actions;

  const BadgeDialog({
    super.key,
    required this.setting,
    required this.content,
    this.icon,
    this.iconWidget,
    this.iconColour,
    this.actions = const [],
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: Row(
        children: [
          iconWidget ?? Icon(icon, color: iconColour ?? scheme.primary),
          const SizedBox(width: 12),
          Flexible(child: Text(setting)),
        ],
      ),
      content: SingleChildScrollView(child: content),
      actions: [
        ...actions,
        TextButton(
          key: const ValueKey('badge-close'),
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.t.close),
        ),
      ],
    );
  }
}

/// The switch of an on-or-off setting in its [BadgeDialog]: its value in
/// words, the switch below it (UI laws, "The control goes below its
/// text"). Each switch writes at once through [toggle].
class BadgeSwitch extends StatefulWidget {
  final String setting;
  final bool on;
  final VoidCallback toggle;

  const BadgeSwitch({
    super.key,
    required this.setting,
    required this.on,
    required this.toggle,
  });

  @override
  State<BadgeSwitch> createState() => _BadgeSwitchState();
}

class _BadgeSwitchState extends State<BadgeSwitch> {
  late var _on = widget.on;

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(_on ? t.settingOn : t.settingOff),
        const SizedBox(height: 8),
        Semantics(
          label: widget.setting,
          child: Switch(
            key: const ValueKey('badge-switch'),
            value: _on,
            onChanged: (_) {
              widget.toggle();
              setState(() => _on = !_on);
            },
          ),
        ),
      ],
    );
  }
}
