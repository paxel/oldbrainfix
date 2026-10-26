import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The colours of one brightness (UI laws, "Look"): a light teal ground
/// or an anthracite one, question marks a touch darker or brighter than
/// it, and cards that clearly stand out from both.
class LookTones {
  final Color background;
  final Color mark;
  final Color card;

  /// The emboss: a shade below and right, a highlight above and left.
  final Color shade;
  final Color highlight;

  /// The teal accent.
  final Color accent;

  const LookTones({
    required this.background,
    required this.mark,
    required this.card,
    required this.shade,
    required this.highlight,
    required this.accent,
  });

  static const light = LookTones(
    background: Color(0xFFCDE8E4),
    mark: Color(0xFFBCDDD8),
    card: Color(0xFFF8FCFB),
    shade: Color(0x663E7A73),
    highlight: Color(0xCCFFFFFF),
    accent: Color(0xFF00796B),
  );

  static const dark = LookTones(
    background: Color(0xFF26292C),
    mark: Color(0xFF31363A),
    card: Color(0xFF3E454A),
    shade: Color(0x99000000),
    highlight: Color(0x1FFFFFFF),
    accent: Color(0xFF4DB6AC),
  );

  static LookTones of(Brightness brightness) =>
      brightness == Brightness.dark ? dark : light;
}

/// The app's theme for [brightness]. Pages are transparent: the
/// question-mark background under them is the ground.
ThemeData lookTheme(Brightness brightness) {
  final tones = LookTones.of(brightness);
  final scheme = ColorScheme.fromSeed(
    seedColor: tones.accent,
    brightness: brightness,
  ).copyWith(primary: tones.accent, surface: tones.card);
  return ThemeData(
    colorScheme: scheme,
    scaffoldBackgroundColor: Colors.transparent,
    // The bar is plain ground: titles stay readable over scrolled cards.
    appBarTheme: AppBarTheme(
      backgroundColor: tones.background,
      scrolledUnderElevation: 0,
    ),
    // The "+" is a control over the question marks: the full accent,
    // not a pale container that melts into the light teal ground.
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: scheme.primary,
      foregroundColor: scheme.onPrimary,
    ),
    // Text fields sit on the question marks too and must stand out.
    inputDecorationTheme: InputDecorationThemeData(
      filled: true,
      fillColor: tones.card,
    ),
  );
}

/// The Device's theme choice as Flutter's theme mode.
ThemeMode themeModeOf(ThemeChoice choice) => switch (choice) {
  ThemeChoice.system => ThemeMode.system,
  ThemeChoice.light => ThemeMode.light,
  ThemeChoice.dark => ThemeMode.dark,
};

/// A softly embossed panel, visibly above the background (UI laws,
/// "Cards").
class Embossed extends StatelessWidget {
  final Widget child;

  /// Raised further above the background, for important cards.
  final bool strong;

  const Embossed({super.key, required this.child, this.strong = false});

  @override
  Widget build(BuildContext context) {
    final tones = LookTones.of(Theme.of(context).brightness);
    final lift = strong ? 2.0 : 1.0;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: tones.card,
        borderRadius: BorderRadius.circular(16),
        boxShadow: [
          BoxShadow(
            color: tones.shade,
            offset: Offset(4 * lift, 4 * lift),
            blurRadius: 10 * lift,
          ),
          BoxShadow(
            color: tones.highlight,
            offset: Offset(-3 * lift, -3 * lift),
            blurRadius: 8 * lift,
          ),
        ],
      ),
      child: child,
    );
  }
}

/// A row of text with its control (a switch, button or dropdown) on its
/// own line below the text, start-aligned with it, never beside it (UI
/// laws, "Dialogs and pages").
class ControlBelow extends StatelessWidget {
  final Widget? leading;
  final Widget title;
  final Widget? subtitle;
  final Widget control;
  final EdgeInsetsGeometry padding;

  /// A tap on the text, for a row whose text opens something. It is a
  /// convenience for the finger: the control stays the one target for
  /// assistive technology.
  final VoidCallback? onTap;

  const ControlBelow({
    super.key,
    this.leading,
    required this.title,
    this.subtitle,
    required this.control,
    this.padding = const EdgeInsetsDirectional.fromSTEB(16, 8, 24, 8),
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    Widget text = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DefaultTextStyle.merge(
          style: theme.textTheme.bodyLarge?.copyWith(color: scheme.onSurface),
          child: title,
        ),
        if (subtitle case final subtitle?)
          DefaultTextStyle.merge(
            style: theme.textTheme.bodyMedium?.copyWith(
              color: scheme.onSurfaceVariant,
            ),
            child: subtitle,
          ),
      ],
    );
    if (onTap case final onTap?) {
      text = InkWell(onTap: onTap, excludeFromSemantics: true, child: text);
    }
    return Padding(
      padding: padding,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (leading case final leading?) ...[
            IconTheme.merge(
              data: IconThemeData(color: scheme.onSurfaceVariant),
              child: ConstrainedBox(
                constraints: const BoxConstraints(minWidth: 24),
                child: leading,
              ),
            ),
            const SizedBox(width: 16),
          ],
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [text, const SizedBox(height: 4), control],
            ),
          ),
        ],
      ),
    );
  }
}

/// A [ControlBelow] with a switch: its text is the switch's semantic
/// label, as in a [SwitchListTile]. Only the switch toggles; the row
/// around it does not (UI laws, "Gestures").
class SwitchBelow extends StatelessWidget {
  /// The switch's own key, for finding and tapping it.
  final Key? switchKey;
  final Widget? leading;
  final Widget title;
  final Widget? subtitle;
  final bool value;
  final ValueChanged<bool>? onChanged;
  final EdgeInsetsGeometry padding;

  /// See [ControlBelow.onTap].
  final VoidCallback? onTap;

  const SwitchBelow({
    super.key,
    this.switchKey,
    this.leading,
    required this.title,
    this.subtitle,
    required this.value,
    required this.onChanged,
    this.padding = const EdgeInsetsDirectional.fromSTEB(16, 8, 24, 8),
    this.onTap,
  });

  @override
  Widget build(BuildContext context) => MergeSemantics(
    child: ControlBelow(
      leading: leading,
      title: title,
      subtitle: subtitle,
      padding: padding,
      onTap: onTap,
      control: Switch(key: switchKey, value: value, onChanged: onChanged),
    ),
  );
}

/// One option of a [RadioColumn].
class RadioOption<T> {
  final T value;
  final Widget title;
  final Widget? subtitle;

  /// An icon at the line's end.
  final Widget? secondary;

  /// The option's own key, for finding and tapping it.
  final Key? key;

  const RadioOption({
    required this.value,
    required this.title,
    this.subtitle,
    this.secondary,
    this.key,
  });
}

/// A single choice: a column of radio buttons, one option per line, two
/// options included (UI laws, "Dialogs and pages"). Without [onChanged]
/// the options show but do not change.
class RadioColumn<T> extends StatelessWidget {
  final T value;
  final List<RadioOption<T>> options;
  final ValueChanged<T>? onChanged;

  const RadioColumn({
    super.key,
    required this.value,
    required this.options,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final onChanged = this.onChanged;
    return RadioGroup<T>(
      groupValue: value,
      onChanged: (picked) {
        // A tap on the chosen option changes nothing.
        if (onChanged == null || picked == null || picked == value) return;
        onChanged(picked);
      },
      // Its own transparent Material, so the ink shows on an [Embossed]
      // panel too.
      child: Material(
        type: MaterialType.transparency,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final option in options)
              RadioListTile<T>(
                key: option.key,
                contentPadding: EdgeInsets.zero,
                enabled: onChanged != null,
                value: option.value,
                title: option.title,
                subtitle: option.subtitle,
                secondary: option.secondary,
              ),
          ],
        ),
      ),
    );
  }
}
