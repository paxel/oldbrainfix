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

  const Embossed({super.key, required this.child});

  @override
  Widget build(BuildContext context) {
    final tones = LookTones.of(Theme.of(context).brightness);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: tones.card,
        borderRadius: BorderRadius.circular(16),
        boxShadow: [
          BoxShadow(
            color: tones.shade,
            offset: const Offset(4, 4),
            blurRadius: 10,
          ),
          BoxShadow(
            color: tones.highlight,
            offset: const Offset(-3, -3),
            blurRadius: 8,
          ),
        ],
      ),
      child: child,
    );
  }
}
