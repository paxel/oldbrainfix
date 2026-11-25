import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The local setting that holds "Show button taps" off (obf-1143),
/// 'true' then; nothing kept, or 'false', is on, the default.
const tapMarksOffKey = 'tapMarksOff';

/// "Show button taps" in Settings ▸ Look (obf-1143), one per store, on
/// until switched off on this Device.
class TapMarkSwitch extends ValueNotifier<bool> {
  static final _of = Expando<TapMarkSwitch>('tap mark switch');

  final Store store;

  TapMarkSwitch._(this.store)
    : super(store.localSetting(tapMarksOffKey) != 'true');

  /// The switch of [store]'s Device.
  factory TapMarkSwitch.of(Store store) =>
      _of[store] ??= TapMarkSwitch._(store);

  /// Switches the marks on or off, and keeps it for this Device.
  void set(bool on) {
    on
        ? store.removeLocalSetting(tapMarksOffKey)
        : store.setLocalSetting(tapMarksOffKey, 'true');
    value = on;
  }
}

/// A green "!" for each button tap (obf-1143), as cat(a)log's paw: the
/// sign that the tap was registered. [shown] holds the marks still on
/// the screen, each where its tap landed, in global coordinates.
class TapMarks extends ChangeNotifier {
  final _shown = <({int id, Offset at})>[];
  var _next = 0;

  /// The marks on the screen now, oldest first.
  List<({int id, Offset at})> get shown => List.unmodifiable(_shown);

  /// Shows a mark above [at].
  void show(Offset at) {
    _shown.add((id: _next++, at: at));
    notifyListeners();
  }

  void _done(int id) {
    _shown.removeWhere((mark) => mark.id == id);
    notifyListeners();
  }
}

/// The ink of every button, its own splash as before, that calls [onTap]
/// with where the tap landed once the tap is confirmed: a tap, not a
/// press that became a scroll (obf-1143).
class TapMarkSplashFactory extends InteractiveInkFeatureFactory {
  final ValueChanged<Offset> onTap;

  const TapMarkSplashFactory(this.onTap);

  @override
  InteractiveInkFeature create({
    required MaterialInkController controller,
    required RenderBox referenceBox,
    required Offset position,
    required Color color,
    required TextDirection textDirection,
    bool containedInkWell = false,
    RectCallback? rectCallback,
    BorderRadius? borderRadius,
    ShapeBorder? customBorder,
    double? radius,
    VoidCallback? onRemoved,
  }) {
    void confirmed() {
      if (referenceBox.attached) onTap(referenceBox.localToGlobal(position));
    }

    // Material's own splash of the platform, as the theme picks it.
    if (defaultTargetPlatform == TargetPlatform.android && !kIsWeb) {
      return _MarkedSparkle(
        confirmed,
        controller: controller,
        referenceBox: referenceBox,
        position: position,
        color: color,
        textDirection: textDirection,
        containedInkWell: containedInkWell,
        rectCallback: rectCallback,
        borderRadius: borderRadius,
        customBorder: customBorder,
        radius: radius,
        onRemoved: onRemoved,
      );
    }
    return _MarkedRipple(
      confirmed,
      controller: controller,
      referenceBox: referenceBox,
      position: position,
      color: color,
      textDirection: textDirection,
      containedInkWell: containedInkWell,
      rectCallback: rectCallback,
      borderRadius: borderRadius,
      customBorder: customBorder,
      radius: radius,
      onRemoved: onRemoved,
    );
  }
}

class _MarkedSparkle extends InkSparkle {
  final VoidCallback confirmed;

  _MarkedSparkle(
    this.confirmed, {
    required super.controller,
    required super.referenceBox,
    required super.color,
    required super.position,
    required super.textDirection,
    required super.containedInkWell,
    super.rectCallback,
    super.borderRadius,
    super.customBorder,
    super.radius,
    super.onRemoved,
  });

  @override
  void confirm() {
    super.confirm();
    confirmed();
  }
}

class _MarkedRipple extends InkRipple {
  final VoidCallback confirmed;

  _MarkedRipple(
    this.confirmed, {
    required super.controller,
    required super.referenceBox,
    required super.color,
    required super.position,
    required super.textDirection,
    required super.containedInkWell,
    super.rectCallback,
    super.borderRadius,
    super.customBorder,
    super.radius,
    super.onRemoved,
  });

  @override
  void confirm() {
    super.confirm();
    confirmed();
  }
}

/// The [TapMarkSplashFactory] of a theme, for the buttons that do not
/// take a button theme's ink: the "+" ([TapMarked]).
class TapMarkTheme extends ThemeExtension<TapMarkTheme> {
  final TapMarkSplashFactory splash;

  const TapMarkTheme(this.splash);

  @override
  TapMarkTheme copyWith({TapMarkSplashFactory? splash}) =>
      TapMarkTheme(splash ?? this.splash);

  @override
  TapMarkTheme lerp(TapMarkTheme? other, double t) => this;
}

/// [child], a floating "+" or a fan's entry, with the ink that marks its
/// taps where the theme has it (obf-1143); as it is elsewhere.
class TapMarked extends StatelessWidget {
  final Widget child;

  const TapMarked({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final marks = theme.extension<TapMarkTheme>();
    if (marks == null) return child;
    return Theme(
      data: theme.copyWith(splashFactory: marks.splash),
      child: child,
    );
  }
}

/// [child], the whole app, with the green "!" of each mark of [marks]
/// drawn over it, dialogs and fans included: 56 high above the finger,
/// growing and fading out over 0.9 s, silent; where the system removes
/// animations it stands still and goes after 0.9 s. It takes no touch
/// and says nothing to a screen reader.
class TapMarkLayer extends StatelessWidget {
  final TapMarks marks;
  final Widget child;

  const TapMarkLayer({super.key, required this.marks, required this.child});

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.passthrough,
    children: [
      child,
      Positioned.fill(
        child: IgnorePointer(
          child: ExcludeSemantics(
            child: ListenableBuilder(
              listenable: marks,
              builder: (context, _) => Stack(
                clipBehavior: Clip.none,
                children: [
                  for (final mark in marks.shown)
                    _TapMarkView(
                      key: ValueKey(mark.id),
                      at: mark.at,
                      onDone: () => marks._done(mark.id),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    ],
  );
}

class _TapMarkView extends StatefulWidget {
  final Offset at;
  final VoidCallback onDone;

  const _TapMarkView({super.key, required this.at, required this.onDone});

  @override
  State<_TapMarkView> createState() => _TapMarkViewState();
}

class _TapMarkViewState extends State<_TapMarkView>
    with SingleTickerProviderStateMixin {
  /// How long a mark shows.
  static const _shows = Duration(milliseconds: 900);

  /// 0.9 s also where animations are removed: then it only stands.
  late final _controller =
      AnimationController(
        vsync: this,
        duration: _shows,
        animationBehavior: AnimationBehavior.preserve,
      )..addStatusListener((status) {
        if (status == AnimationStatus.completed) widget.onDone();
      });

  late final _fade = Tween(
    begin: 1.0,
    end: 0.0,
  ).animate(CurvedAnimation(parent: _controller, curve: Curves.easeIn));

  late final _grow = Tween(
    begin: 0.6,
    end: 1.4,
  ).animate(CurvedAnimation(parent: _controller, curve: Curves.easeOut));

  @override
  void initState() {
    super.initState();
    // Runs also where nothing of it is drawn, to end the mark.
    _controller.forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const size = 56.0;
    // The layer covers the whole app from its top left: global is local.
    // Above a button of the top bar it stays on the screen.
    final at = widget.at;
    final screen = MediaQuery.sizeOf(context);
    final safe = MediaQuery.paddingOf(context);
    final left = math.min(
      math.max(0.0, at.dx - size / 2),
      math.max(0.0, screen.width - size),
    );
    final top = math.max(safe.top, at.dy - size * 1.6);
    final mark = Icon(
      Icons.priority_high,
      key: const ValueKey('tapMark'),
      size: size,
      color: Colors.green.shade600,
    );
    return Positioned(
      left: left,
      // Above the finger, or the thumb covers it.
      top: top,
      child: MediaQuery.disableAnimationsOf(context)
          ? mark
          : FadeTransition(
              opacity: _fade,
              child: ScaleTransition(scale: _grow, child: mark),
            ),
    );
  }
}
