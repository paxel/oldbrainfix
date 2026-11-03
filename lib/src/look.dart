import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

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

/// How much "Fat fingers" grows text, icons and tap targets (#224).
const fatFingersScale = 1.3;

/// How much this Device grows text, icons and tap targets (#224): 1, or
/// [fatFingersScale] with "Fat fingers" on. The theme carries it, so
/// the measures of the Agenda's rows follow it with the icons.
class LookScale extends ThemeExtension<LookScale> {
  final double factor;

  const LookScale(this.factor);

  /// The factor of the theme around [context]; 1 without one.
  static double of(BuildContext context) =>
      Theme.of(context).extension<LookScale>()?.factor ?? 1;

  @override
  LookScale copyWith({double? factor}) => LookScale(factor ?? this.factor);

  @override
  LookScale lerp(LookScale? other, double t) =>
      other == null ? this : LookScale(factor + (other.factor - factor) * t);
}

/// The text scaler of [base], the system's font size, times [factor]
/// (#224): "Fat fingers" grows the text on top of what the system set.
class TimesTextScaler extends TextScaler {
  final TextScaler base;
  final double factor;

  const TimesTextScaler(this.base, this.factor);

  @override
  double scale(double fontSize) => base.scale(fontSize) * factor;

  @override
  double get textScaleFactor => scale(14) / 14;

  @override
  bool operator ==(Object other) =>
      other is TimesTextScaler && other.base == base && other.factor == factor;

  @override
  int get hashCode => Object.hash(base, factor);
}

/// Grows the text of everything under it by [fatFingersScale] while [on]
/// (#224), on top of the system's font size.
class FatFingersText extends StatelessWidget {
  final bool on;
  final Widget child;

  const FatFingersText({super.key, required this.on, required this.child});

  @override
  Widget build(BuildContext context) {
    if (!on) return child;
    final media = MediaQuery.of(context);
    return MediaQuery(
      data: media.copyWith(
        textScaler: TimesTextScaler(media.textScaler, fatFingersScale),
      ),
      child: child,
    );
  }
}

/// [child] grown by the [LookScale] of the theme, in its room and on the
/// screen alike, for a control whose size the density does not reach:
/// the switch (#224).
class LookGrown extends SingleChildRenderObjectWidget {
  const LookGrown({super.key, required Widget super.child});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderGrown(LookScale.of(context));

  @override
  void updateRenderObject(BuildContext context, RenderGrown renderObject) =>
      renderObject.factor = LookScale.of(context);
}

class RenderGrown extends RenderProxyBox {
  double _factor;

  RenderGrown(this._factor);

  set factor(double factor) {
    if (factor == _factor) return;
    _factor = factor;
    markNeedsLayout();
  }

  Matrix4 get _transform => Matrix4.diagonal3Values(_factor, _factor, 1);

  BoxConstraints _shrunk(BoxConstraints constraints) => BoxConstraints(
    minWidth: constraints.minWidth / _factor,
    maxWidth: constraints.maxWidth / _factor,
    minHeight: constraints.minHeight / _factor,
    maxHeight: constraints.maxHeight / _factor,
  );

  // Without a child it is what a RenderProxyBox without one is: as
  // small as allowed, with nothing to paint or hit.

  @override
  void performLayout() {
    final child = this.child;
    if (child == null) {
      size = computeSizeForNoChild(constraints);
      return;
    }
    child.layout(_shrunk(constraints), parentUsesSize: true);
    size = constraints.constrain(child.size * _factor);
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) => switch (child) {
    final child? => constraints.constrain(
      child.getDryLayout(_shrunk(constraints)) * _factor,
    ),
    null => computeSizeForNoChild(constraints),
  };

  @override
  double computeMinIntrinsicWidth(double height) =>
      (child?.getMinIntrinsicWidth(height / _factor) ?? 0) * _factor;

  @override
  double computeMaxIntrinsicWidth(double height) =>
      (child?.getMaxIntrinsicWidth(height / _factor) ?? 0) * _factor;

  @override
  double computeMinIntrinsicHeight(double width) =>
      (child?.getMinIntrinsicHeight(width / _factor) ?? 0) * _factor;

  @override
  double computeMaxIntrinsicHeight(double width) =>
      (child?.getMaxIntrinsicHeight(width / _factor) ?? 0) * _factor;

  @override
  void paint(PaintingContext context, Offset offset) {
    final child = this.child;
    if (child == null) return;
    if (_factor == 1) return context.paintChild(child, offset);
    layer = context.pushTransform(
      needsCompositing,
      offset,
      _transform,
      (context, offset) => context.paintChild(child, offset),
      oldLayer: layer as TransformLayer?,
    );
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    final child = this.child;
    if (child == null) return false;
    return result.addWithPaintTransform(
      transform: _transform,
      position: position,
      hitTest: (result, position) => child.hitTest(result, position: position),
    );
  }

  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) =>
      transform.multiply(_transform);
}

/// The app's theme for [brightness]. Pages are transparent: the
/// question-mark background under them is the ground. With [fatFingers]
/// (#224) icons and tap targets are [fatFingersScale] times their size:
/// the icons through the icon themes, the targets through the density,
/// whose largest step adds 4 dp a side to a 48 dp target.
ThemeData lookTheme(Brightness brightness, {bool fatFingers = false}) {
  final tones = LookTones.of(brightness);
  final scheme = ColorScheme.fromSeed(
    seedColor: tones.accent,
    brightness: brightness,
  ).copyWith(primary: tones.accent, surface: tones.card);
  final scale = fatFingers ? fatFingersScale : 1.0;
  // Material's own sizes, grown: 24 for an icon, 18 for a button's.
  final icon = 24 * scale;
  final buttonIcon = WidgetStatePropertyAll(18 * scale);
  final density = kMinInteractiveDimension * (scale - 1) / 4;
  final grown = VisualDensity(horizontal: density, vertical: density);
  return ThemeData(
    colorScheme: scheme,
    scaffoldBackgroundColor: Colors.transparent,
    extensions: [LookScale(scale)],
    visualDensity: fatFingers ? grown : null,
    iconTheme: fatFingers ? IconThemeData(size: icon) : null,
    // Material 3's icon buttons take no density: their target is set
    // as their least size, 48 grown.
    iconButtonTheme: fatFingers
        ? IconButtonThemeData(
            style: ButtonStyle(
              iconSize: WidgetStatePropertyAll(icon),
              minimumSize: WidgetStatePropertyAll(
                Size.square(kMinInteractiveDimension * scale),
              ),
            ),
          )
        : null,
    textButtonTheme: fatFingers
        ? TextButtonThemeData(style: ButtonStyle(iconSize: buttonIcon))
        : null,
    filledButtonTheme: fatFingers
        ? FilledButtonThemeData(style: ButtonStyle(iconSize: buttonIcon))
        : null,
    outlinedButtonTheme: fatFingers
        ? OutlinedButtonThemeData(style: ButtonStyle(iconSize: buttonIcon))
        : null,
    elevatedButtonTheme: fatFingers
        ? ElevatedButtonThemeData(style: ButtonStyle(iconSize: buttonIcon))
        : null,
    // The bar is plain ground: titles stay readable over scrolled cards.
    appBarTheme: AppBarTheme(
      backgroundColor: tones.background,
      scrolledUnderElevation: 0,
      toolbarHeight: fatFingers ? kToolbarHeight * scale : null,
      leadingWidth: fatFingers ? kToolbarHeight * scale : null,
      // Material's colours of the bar's icons, with their size.
      iconTheme: fatFingers
          ? IconThemeData(color: scheme.onSurface, size: icon)
          : null,
      actionsIconTheme: fatFingers
          ? IconThemeData(color: scheme.onSurfaceVariant, size: icon)
          : null,
    ),
    // The "+" is a control over the question marks: the full accent,
    // not a pale container that melts into the light teal ground.
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: scheme.primary,
      foregroundColor: scheme.onPrimary,
      iconSize: fatFingers ? icon : null,
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
      control: LookGrown(
        child: Switch(key: switchKey, value: value, onChanged: onChanged),
      ),
    ),
  );
}

/// One option of a [RadioColumn] or of [ChoiceChips].
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

/// A long or wordy single choice: a column of radio buttons, one option
/// per line (UI laws, "Dialogs and pages"); a short one is [ChoiceChips].
/// Without [onChanged] the options show but do not change.
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

/// A short single choice (#226): a wrapping row of choice chips, the one
/// [value] selected, as the Routine editor's weekday chips look (UI laws,
/// "Dialogs and pages"). An option's [RadioOption.secondary] is its chip's
/// icon; one with a subtitle belongs in a [RadioColumn]. A screen reader
/// hears a radio group of buttons, the chosen one selected. Without
/// [onChanged] the chips show but do not change.
class ChoiceChips<T> extends StatelessWidget {
  final T value;
  final List<RadioOption<T>> options;
  final ValueChanged<T>? onChanged;

  const ChoiceChips({
    super.key,
    required this.value,
    required this.options,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    assert(
      options.every((o) => o.subtitle == null),
      'a choice with details is a RadioColumn',
    );
    final onChanged = this.onChanged;
    return Semantics(
      container: true,
      role: SemanticsRole.radioGroup,
      child: Wrap(
        spacing: 4,
        runSpacing: 4,
        children: [
          for (final option in options)
            MergeSemantics(
              child: Semantics(
                inMutuallyExclusiveGroup: true,
                child: ChoiceChip(
                  key: option.key,
                  avatar: option.secondary,
                  // A check over the icon would hide it.
                  showCheckmark: option.secondary == null,
                  label: option.title,
                  selected: option.value == value,
                  // A tap on the chosen option changes nothing.
                  onSelected: onChanged == null
                      ? null
                      : (_) {
                          if (option.value != value) onChanged(option.value);
                        },
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// A number with a minus and a plus button, and its [unit] after it: the
/// one way to pick a small count, never a typed field, so it holds no
/// invalid value. Minus stops at [min], plus at [max] when given.
class CountStepper extends StatelessWidget {
  final int value;
  final ValueChanged<int> onChanged;
  final int min;
  final int? max;
  final String? unit;

  const CountStepper({
    super.key,
    required this.value,
    required this.onChanged,
    this.min = 1,
    this.max,
    this.unit,
  });

  @override
  Widget build(BuildContext context) {
    final max = this.max;
    return Wrap(
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        IconButton(
          tooltip: context.t.fewer,
          onPressed: value > min ? () => onChanged(value - 1) : null,
          icon: const Icon(Icons.remove),
        ),
        Text('$value', style: Theme.of(context).textTheme.titleMedium),
        IconButton(
          tooltip: context.t.more,
          onPressed: max == null || value < max
              ? () => onChanged(value + 1)
              : null,
          icon: const Icon(Icons.add),
        ),
        if (unit case final unit?) Text(unit),
      ],
    );
  }
}
