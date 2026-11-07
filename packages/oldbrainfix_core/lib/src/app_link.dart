import 'dart:convert';

/// What an app link opens (#261, ADR 0046).
enum AppLinkKind {
  /// A Note, by its id.
  note,

  /// An Area or a Topic, by its id.
  place,
  agenda,
  unsorted,

  /// The Settings, or one of their [AppLink.settingsSections].
  settings,

  /// All Routines and Reminders.
  routines,
  backups,
  sync,
  profile,
  frogs,
  newSession,

  /// A page of the user guide on the website, its start page without
  /// one.
  guide,

  /// A file in the app's shared folder, by its path inside it.
  file,
}

/// A link to a place in the app (#261, ADR 0046): an ordinary Markdown
/// link whose destination has the app's scheme, `oldbrainfix://`, such
/// as `oldbrainfix://note/<id>` or `oldbrainfix://settings/backups`. Ids
/// are the same on every Device, so a link survives sync; a plain
/// Markdown app shows its label. [parse] knows every form; anything else
/// with the scheme is no app link and opens nothing.
class AppLink {
  final AppLinkKind kind;

  /// The Note's or the place's id, the Settings' section, the guide's
  /// page or the file's path inside the shared folder, its folders
  /// separated by `/`; null where the kind has none.
  final String? target;

  /// The box of the guide's page, its `#` part.
  final String? anchor;

  const AppLink(this.kind, [this.target, this.anchor]);

  /// The app's scheme with its `//`.
  static const scheme = 'oldbrainfix://';

  /// The website's user guide, a language's folder and `guide/` after it.
  static const site = 'https://paxel.github.io/oldbrainfix';

  /// The sections of the Settings a link can open, on either app; one
  /// this app has not opens the Settings at their top.
  static const settingsSections = [
    'look',
    'units',
    'agenda',
    'alarms',
    'nudges',
    'capture',
    'quick-note',
    'calendar',
    'privacy',
    'typing',
    'history',
    'import',
    'features',
    'computer',
  ];

  /// The link of [url], or null when it is none: the scheme in any case
  /// of ASCII letters, then one of the forms [url] writes, as written.
  /// No query; a `#` only after a guide's page. A file's path has no
  /// empty part, no `.` or `..`, and no `\`, `:` or control character,
  /// so it never leads out of the shared folder.
  static AppLink? parse(String url) {
    if (url.length < scheme.length) return null;
    final head = String.fromCharCodes([
      for (final c in url.substring(0, scheme.length).codeUnits)
        c >= 0x41 && c <= 0x5A ? c | 0x20 : c,
    ]);
    if (head != scheme) return null;
    var rest = url.substring(scheme.length);
    if (rest.contains('?')) return null;
    String? anchor;
    final hash = rest.indexOf('#');
    if (hash >= 0) {
      anchor = rest.substring(hash + 1);
      rest = rest.substring(0, hash);
    }
    final parts = rest.split('/');
    if (anchor != null && !(parts.length == 2 && parts[0] == 'guide')) {
      return null;
    }
    AppLink? only(AppLinkKind kind) => parts.length == 1 ? AppLink(kind) : null;
    switch (parts[0]) {
      case 'note' || 'area':
        if (parts.length != 2) return null;
        final id = _decode(parts[1]);
        if (id == null || id.isEmpty) return null;
        return AppLink(
          parts[0] == 'note' ? AppLinkKind.note : AppLinkKind.place,
          id,
        );
      case 'agenda':
        return only(AppLinkKind.agenda);
      case 'unsorted':
        return only(AppLinkKind.unsorted);
      case 'routines':
        return only(AppLinkKind.routines);
      case 'profile':
        return only(AppLinkKind.profile);
      case 'frogs':
        return only(AppLinkKind.frogs);
      case 'session':
        return parts.length == 2 && parts[1] == 'new'
            ? const AppLink(AppLinkKind.newSession)
            : null;
      case 'settings':
        if (parts.length == 1) return const AppLink(AppLinkKind.settings);
        if (parts.length != 2) return null;
        return switch (parts[1]) {
          'backups' => const AppLink(AppLinkKind.backups),
          'sync' => const AppLink(AppLinkKind.sync),
          final s when settingsSections.contains(s) => AppLink(
            AppLinkKind.settings,
            s,
          ),
          _ => null,
        };
      case 'guide':
        if (parts.length == 1) return const AppLink(AppLinkKind.guide);
        if (parts.length != 2 || !_slug(parts[1])) return null;
        if (anchor != null && !_slug(anchor)) return null;
        return AppLink(AppLinkKind.guide, parts[1], anchor);
      case 'file':
        if (parts.length < 2) return null;
        final path = <String>[];
        for (final part in parts.skip(1)) {
          final name = _decode(part);
          if (name == null ||
              name.isEmpty ||
              name == '.' ||
              name == '..' ||
              name.codeUnits.any(
                (c) =>
                    c < 0x20 ||
                    c == 0x7F ||
                    c == 0x2F ||
                    c == 0x5C ||
                    c == 0x3A,
              )) {
            return null;
          }
          path.add(name);
        }
        return AppLink(AppLinkKind.file, path.join('/'));
    }
    return null;
  }

  /// The link as it is written, which [parse] reads back: ids and a
  /// file's names %-encoded but for ASCII letters, digits, `-`, `.`,
  /// `_`, `~`, `:` and `@` (a file's `:` encoded too), upper-case hex.
  String get url {
    final t = target;
    return scheme +
        switch (kind) {
          AppLinkKind.note => 'note/${_encode(t ?? '')}',
          AppLinkKind.place => 'area/${_encode(t ?? '')}',
          AppLinkKind.agenda => 'agenda',
          AppLinkKind.unsorted => 'unsorted',
          AppLinkKind.settings => t == null ? 'settings' : 'settings/$t',
          AppLinkKind.routines => 'routines',
          AppLinkKind.backups => 'settings/backups',
          AppLinkKind.sync => 'settings/sync',
          AppLinkKind.profile => 'profile',
          AppLinkKind.frogs => 'frogs',
          AppLinkKind.newSession => 'session/new',
          AppLinkKind.guide =>
            t == null ? 'guide' : 'guide/$t${anchor == null ? '' : '#$anchor'}',
          AppLinkKind.file =>
            'file/${(t ?? '').split('/').map((n) => _encode(n, file: true)).join('/')}',
        };
  }

  /// The guide's page on the website in [language], English where the
  /// site has none: its start page, or the page with its box.
  String guideAddress(String language) {
    final lang = language == 'de' ? 'de' : 'en';
    final page = target;
    final box = anchor;
    return '$site/$lang/guide/'
        '${page == null ? '' : '$page.html'}'
        '${box == null ? '' : '#$box'}';
  }

  @override
  bool operator ==(Object other) =>
      other is AppLink &&
      other.kind == kind &&
      other.target == target &&
      other.anchor == anchor;

  @override
  int get hashCode => Object.hash(kind, target, anchor);

  @override
  String toString() => url;

  /// Lower-case ASCII letters, digits and `-`, at least one.
  static bool _slug(String s) =>
      s.isNotEmpty &&
      s.codeUnits.every(
        (c) =>
            (c >= 0x61 && c <= 0x7A) || (c >= 0x30 && c <= 0x39) || c == 0x2D,
      );

  static bool _kept(int c, {required bool file}) =>
      (c >= 0x41 && c <= 0x5A) ||
      (c >= 0x61 && c <= 0x7A) ||
      (c >= 0x30 && c <= 0x39) ||
      c == 0x2D ||
      c == 0x2E ||
      c == 0x5F ||
      c == 0x7E ||
      c == 0x40 ||
      (c == 0x3A && !file);

  static String _encode(String s, {bool file = false}) {
    final out = StringBuffer();
    for (final b in utf8.encode(s)) {
      if (_kept(b, file: file)) {
        out.writeCharCode(b);
      } else {
        out.write('%${b.toRadixString(16).toUpperCase().padLeft(2, '0')}');
      }
    }
    return out.toString();
  }

  /// [s] with its `%XX` decoded as UTF-8; null when one is broken.
  static String? _decode(String s) {
    final raw = utf8.encode(s);
    final bytes = <int>[];
    for (var i = 0; i < raw.length; i++) {
      if (raw[i] != 0x25) {
        bytes.add(raw[i]);
        continue;
      }
      if (i + 2 >= raw.length) return null;
      final high = _hex(raw[i + 1]);
      final low = _hex(raw[i + 2]);
      if (high < 0 || low < 0) return null;
      bytes.add(high * 16 + low);
      i += 2;
    }
    try {
      return utf8.decode(bytes);
    } on FormatException {
      return null;
    }
  }

  static int _hex(int c) => switch (c) {
    >= 0x30 && <= 0x39 => c - 0x30,
    >= 0x41 && <= 0x46 => c - 0x37,
    >= 0x61 && <= 0x66 => c - 0x57,
    _ => -1,
  };
}
