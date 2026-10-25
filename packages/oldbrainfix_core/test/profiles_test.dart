import 'dart:convert';
import 'dart:typed_data';

import 'package:image/image.dart' as img;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A picture of [width] × [height]: red on the left half, blue on the
/// right, as PNG.
Uint8List halves(int width, int height, {bool png = true}) {
  final image = img.Image(width: width, height: height);
  for (final p in image) {
    p
      ..r = p.x < width ~/ 2 ? 255 : 0
      ..g = 0
      ..b = p.x < width ~/ 2 ? 0 : 255;
  }
  return png ? img.encodePng(image) : img.encodeJpg(image);
}

/// The marker bytes of [jpeg]'s segments before the scan.
List<int> markers(Uint8List jpeg) {
  final out = <int>[];
  var i = 2;
  while (i + 4 <= jpeg.length && jpeg[i] == 0xFF) {
    final marker = jpeg[i + 1];
    if (marker == 0xDA) break;
    out.add(marker);
    i += 2 + ((jpeg[i + 2] << 8) | jpeg[i + 3]);
  }
  return out;
}

void expectProfilePicture(Uint8List? picture) {
  expect(picture, isNotNull);
  expect(picture!.sublist(0, 2), [0xFF, 0xD8]);
  final decoded = img.decodeJpg(picture)!;
  expect((decoded.width, decoded.height), (256, 256));
  expect(
    markers(picture).where((m) => (m >= 0xE1 && m <= 0xEF) || m == 0xFE),
    isEmpty,
  );
}

void main() {
  group('names', () {
    test('are trimmed and keep every script and emoji', () {
      expect(sanitizeProfileName('  Ada  '), 'Ada');
      expect(sanitizeProfileName('Jürgen Müller'), 'Jürgen Müller');
      expect(sanitizeProfileName('李小龍'), '李小龍');
      expect(sanitizeProfileName('مريم'), 'مريم');
      expect(
        sanitizeProfileName('Mum 👩\u200D👧 ❤\uFE0F'),
        'Mum 👩\u200D👧 ❤\uFE0F',
      );
    });

    test('lose control characters', () {
      expect(sanitizeProfileName('A\u0000d\u0007a\n\tB\u007F\u0085'), 'AdaB');
    });

    test('lose bidi controls', () {
      expect(
        sanitizeProfileName('\u202EAda\u202C \u2066Bo\u2069\u200E\u200F\u061C'),
        'Ada Bo',
      );
    });

    test('lose zero-width and invisible characters, before trimming', () {
      expect(sanitizeProfileName('\u200B Ada\u2060\uFEFF\u00AD '), 'Ada');
      expect(sanitizeProfileName('A\u200Bd\u3164a'), 'Ada');
    });

    test('keep a joiner only between two visible characters', () {
      expect(sanitizeProfileName('\u200DAda\u200D'), 'Ada');
      expect(sanitizeProfileName('A\u200D d'), 'A d');
      expect(sanitizeProfileName('A\u200D\u200Dd'), 'A\u200Dd');
      expect(sanitizeProfileName('می\u200Cخواهم'), 'می\u200Cخواهم');
    });

    test('keep a variation selector only after a visible character', () {
      expect(sanitizeProfileName('\uFE0FAda'), 'Ada');
      expect(sanitizeProfileName('❤\uFE0F\uFE0F'), '❤\uFE0F');
    });

    test('keep tag characters only inside a flag', () {
      const england =
          '\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}';
      expect(sanitizeProfileName('Ada $england'), 'Ada $england');
      expect(sanitizeProfileName('Ada\u{E0041}\u{E0042}'), 'Ada');
      expect(sanitizeProfileName('\u{E0001}Ada'), 'Ada');
    });

    test('are cut to 40 code points', () {
      expect(sanitizeProfileName('x' * 50), 'x' * 40);
      // Emoji outside the BMP count once, not twice.
      expect(sanitizeProfileName('😀' * 45), '😀' * 40);
      // A cut leaves no space or joiner at the end.
      expect(sanitizeProfileName('${'a' * 38} b'), '${'a' * 38} b');
      expect(sanitizeProfileName('${'a' * 39} bc'), 'a' * 39);
      expect(sanitizeProfileName('${'a' * 39}👩\u200D👧'), '${'a' * 39}👩');
    });

    test('lose half a surrogate pair', () {
      expect(sanitizeProfileName('A\uD800da\uDC00'), 'Ada');
    });

    test('are empty when nothing is left', () {
      expect(sanitizeProfileName(''), '');
      expect(sanitizeProfileName(' \u200B\u202E\n '), '');
    });
  });

  group('the placeholder', () {
    test('shows the initials of the first and the last word', () {
      expect(profileInitials('ada lovelace'), 'AL');
      expect(profileInitials('Ada King Lovelace'), 'AL');
      expect(profileInitials('Ada'), 'A');
      expect(profileInitials('  '), '');
      expect(profileInitials('👩\u200D👧 Mum'), '👩\u200D👧M');
      expect(profileInitials('Élodie'), 'É');
    });

    test('takes its colour from the id, always the same', () {
      expect(profileColor('abc'), profileColor('abc'));
      expect(profileColors, contains(profileColor('abc')));
      final colours = {for (var i = 0; i < 100; i++) profileColor('id$i')};
      expect(colours.length, greaterThan(6));
    });
  });

  group('the short code', () {
    final key = SigningKey.fromSeed(Uint8List(32)).publicKey;
    final other = SigningKey.fromSeed(Uint8List(32)..[0] = 1).publicKey;

    test('is 8 characters in two groups of four', () {
      final code = profileShortCode('p1', key);
      expect(
        code,
        matches(RegExp(r'^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$')),
      );
    });

    test('is the same for the same id and key', () {
      expect(profileShortCode('p1', key), profileShortCode('p1', key));
    });

    test('differs for a copied id with another key, and another id', () {
      expect(profileShortCode('p1', key), isNot(profileShortCode('p1', other)));
      expect(profileShortCode('p1', key), isNot(profileShortCode('p2', key)));
    });
  });

  group('pictures', () {
    test('a chosen square becomes a 256 JPEG without metadata', () {
      final source = halves(600, 400);
      expect(pictureSize(source), (width: 600, height: 400));
      final picture = makeProfilePicture(source, left: 300, top: 0, size: 400);
      expectProfilePicture(picture);
      // The right half was chosen: blue.
      final pixel = img.decodeJpg(picture!)!.getPixel(128, 128);
      expect(pixel.b, greaterThan(200));
      expect(pixel.r, lessThan(60));
    });

    test('a square outside the picture is moved inside it', () {
      final picture = makeProfilePicture(
        halves(300, 200),
        left: 500,
        top: -40,
        size: 900,
      );
      expectProfilePicture(picture);
    });

    test('Exif and comments of the source are not kept', () {
      final image = img.Image(width: 64, height: 64)
        ..exif.imageIfd['Make'] = 'Camera'
        ..exif.gpsIfd['GPSLatitude'] = 52.5;
      final jpeg = img.encodeJpg(image);
      expect(markers(jpeg), contains(0xE1));
      expectProfilePicture(squareProfilePicture(jpeg));
    });

    test('a picture received is made again and refused when too big', () {
      final small = halves(512, 512, png: false);
      expectProfilePicture(receivedProfilePicture(small));
      final big = Uint8List(profilePictureMaxBytes + 1)
        ..setAll(0, halves(512, 512));
      expect(receivedProfilePicture(big), isNull);
    });

    test('what is no picture is refused', () {
      expect(
        receivedProfilePicture(Uint8List.fromList(utf8.encode('hi'))),
        isNull,
      );
      expect(
        receivedProfilePicture(Uint8List.fromList([0xFF, 0xD8, 0xFF, 0, 1, 2])),
        isNull,
      );
      final png = halves(64, 64);
      expect(receivedProfilePicture(png.sublist(0, png.length ~/ 2)), isNull);
      expect(pictureSize(Uint8List(0)), isNull);
    });

    test('a canvas wider than the decoder takes is refused', () {
      Uint8List wide(int width) =>
          img.encodePng(img.Image(width: width, height: 1));
      expect(pictureSize(wide(8192)), (width: 8192, height: 1));
      expect(pictureSize(wide(8193)), isNull);
      expect(squareProfilePicture(wide(8193)), isNull);
      // Pictures from others are made at 256: anything above 1024 is
      // refused.
      expectProfilePicture(receivedProfilePicture(wide(1024)));
      expect(receivedProfilePicture(wide(1025)), isNull);
    });
  });

  group('the record', () {
    test('is written with its keys sorted and read back', () {
      final profile = Profile(
        id: 'p1',
        name: 'Ada',
        picture: Uint8List.fromList([1, 2, 3]),
        version: 2,
      );
      expect(
        profile.encode(),
        '{"id":"p1","name":"Ada","picture":"AQID","version":2}',
      );
      final back = Profile.parse(profile.encode())!;
      expect(back.encode(), profile.encode());
      expect(
        const Profile(id: 'p1', name: '', version: 1).encode(),
        '{"id":"p1","name":"","version":1}',
      );
    });

    test('is sanitised again when read, and refused when malformed', () {
      expect(
        Profile.parse('{"id":"p","name":"\\u202eAda","version":1}')!.name,
        'Ada',
      );
      for (final bad in [
        'nope',
        '[]',
        '{"name":"A","version":1}',
        '{"id":"","name":"A","version":1}',
        '{"id":"p","name":"A","version":0}',
        '{"id":"p","name":"A","version":"1"}',
        '{"id":"p","name":"A","picture":"%%","version":1}',
        '{"id":"p","name":3,"version":1}',
        // URL-safe or non-canonical base64, half a surrogate pair.
        '{"id":"p","name":"A","picture":"AQ-_","version":1}',
        '{"id":"p","name":"A","picture":"AQJ=","version":1}',
        '{"id":"p","name":"\\ud800A","version":1}',
        '{"id":"\\udc00","name":"A","version":1}',
      ]) {
        expect(Profile.parse(bad), isNull, reason: bad);
      }
    });
  });

  group('the own Profile', () {
    late Store store;
    setUp(
      () => store = Store.inMemory(
        newKey: () => SigningKey.fromSeed(Uint8List(32)),
      ),
    );
    tearDown(() => store.close());

    test('does not exist before the defaults are applied', () {
      expect(store.ownProfile, isNull);
      expect(store.ownProfileCode, isNull);
    });

    test('is made once with the defaults and this Device as first key', () {
      final made = store.applyProfileDefaults(name: ' Pixel 9 ');
      expect(made.name, 'Pixel 9');
      expect(made.version, 1);
      expect(made.picture, isNull);
      expect(store.ownProfile!.encode(), made.encode());
      expect(store.ownProfileFirstKey, store.signingKey.publicKey);
      expect(
        store.ownProfileCode,
        profileShortCode(made.id, store.signingKey.publicKey),
      );
      // The same defaults again change nothing.
      expect(
        store.applyProfileDefaults(name: 'Pixel 9').encode(),
        made.encode(),
      );
    });

    test('follows changed defaults until the user chose', () {
      final made = store.applyProfileDefaults(name: 'Pixel 9');
      final renamed = store.applyProfileDefaults(name: 'Ada’s phone');
      expect(
        (renamed.id, renamed.name, renamed.version),
        (made.id, 'Ada’s phone', 2),
      );
      store.renameOwnProfile(' Ada ');
      expect(store.ownProfile!.name, 'Ada');
      expect(store.ownProfile!.version, 3);
      final picture = squareProfilePicture(halves(64, 64))!;
      final withPicture = store.applyProfileDefaults(
        name: 'Other',
        picture: picture,
      );
      expect(withPicture.name, 'Ada');
      expect(withPicture.picture, picture);
      expect(withPicture.version, 4);
      store.setOwnProfilePicture(null);
      expect(store.ownProfile!.picture, isNull);
      expect(
        store.applyProfileDefaults(name: 'Other', picture: picture).picture,
        isNull,
      );
      expect(store.ownProfile!.version, 5);
    });

    test('keeps its version when nothing changed', () {
      store.applyProfileDefaults(name: 'Ada');
      expect(store.renameOwnProfile('Ada\u200B').version, 1);
      expect(store.setOwnProfilePicture(null).version, 1);
    });

    test('cannot be changed before it exists', () {
      expect(() => store.renameOwnProfile('Ada'), throwsStateError);
    });
  });
}
