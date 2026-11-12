import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'remove_media_test.dart' show found, photo, syncAll;
import 'shared_areas_test.dart' show exchange, ticking;

Uint8List _b(String text) => Uint8List.fromList(utf8.encode(text));

NewFile _file(String name, String bytes, {int? pages, String type = ''}) =>
    NewFile(name: name, bytes: _b(bytes), pages: pages, mediaType: type);

DateTime _wall(DateTime utc) => utc;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
  });
  tearDown(() => ann.close());

  List<String> hashes(List<({String noteId, String hash})> blobs) => [
    for (final b in blobs) b.hash,
  ];

  test('a file is attached as it is and waits to be read where its kind '
      'allows', () {
    final big = Uint8List(FileTexts.phoneBytes + 1);
    final note = notes.capture(
      'Papers',
      files: [
        _file('Lease.pdf', '%PDF-1.4 lease', pages: 2),
        _file('Scan.pdf', '%PDF-1.4 scan', pages: 40),
        _file('notes.md', '# Notes'),
        NewFile(name: 'big.txt', bytes: big),
        _file('plan.dwg', 'dwg'),
      ],
    );
    expect(
      [for (final a in note.attachments) a.name],
      ['Lease.pdf', 'Scan.pdf', 'notes.md', 'big.txt', 'plan.dwg'],
    );
    expect(
      [for (final a in note.attachments) a.computer],
      [false, true, false, true, false],
    );
    final lease = note.attachments.first;
    expect(lease.mediaType, 'application/pdf');
    expect((lease.size, lease.pages), (14, 2));
    expect(
      ann.current(note.id, NoteKeys.file(lease.hash)),
      '{"name":"Lease.pdf","pages":2,"size":14,"type":"application/pdf"}',
    );
    expect(notes.fileBytes(lease.hash), _b('%PDF-1.4 lease'));
    expect(ann.photo(lease.hash), isNull);
    String h(int i) => note.attachments[i].hash;
    expect(hashes(notes.unreadAttachments(computer: false)), [h(0), h(2)]);
    expect(hashes(notes.unreadAttachments(computer: true)), [
      h(0),
      h(1),
      h(2),
      h(3),
    ]);
    notes.setAttachmentText(note.id, h(0), '  Rent \r\n\r\n\r\n due ');
    notes.setAttachmentText(note.id, h(2), '');
    final read = notes.byId(note.id)!;
    expect(read.attachmentTexts[h(0)], 'Rent\n\n due');
    expect(hashes(notes.unreadAttachments(computer: true)), [h(1), h(3)]);
    expect(
      () => notes.setAttachmentText(note.id, '0' * 64, 'x'),
      throwsArgumentError,
    );
    expect(
      () => notes.attachFile('note:none', _file('a.txt', 'a')),
      throwsArgumentError,
    );
    expect(read.aloneFile, isNull, reason: 'it has words and five files');
    notes.trash(note.id);
    expect(notes.unreadAttachments(computer: true), isEmpty);
  });

  test('a Note made from a file alone shows the file', () {
    final letter = notes.capture('', files: [_file('Letter.odt', 'PK')]);
    expect(letter.aloneFile?.name, 'Letter.odt');
    final hash = notes.attachFile(letter.id, _file('Other.ods', 'PK2'));
    expect(notes.byId(letter.id)!.aloneFile, isNull);
    final withPhoto = notes.capture(
      '',
      photos: [photo(1)],
      files: [_file('a.zip', 'z')],
    );
    expect(withPhoto.aloneFile, isNull);
    final worded = notes.capture('About it', files: [_file('a.zip', 'z')]);
    expect(worded.aloneFile, isNull);
    expect(notes.byId(letter.id)!.attachments[1].hash, hash);
  });

  test('removing a file\'s text keeps the file and removing a file keeps '
      'its text', () {
    final note = notes.capture(
      '',
      files: [
        _file('Lease.txt', 'lease'),
        _file('Bill.txt', 'bill'),
        _file('x.zip', 'zip'),
      ],
    );
    final [lease, bill, zip] = [for (final a in note.attachments) a.hash];
    notes.setAttachmentText(note.id, lease, 'Rent due');
    notes.setAttachmentText(note.id, bill, 'Power bill');
    // Nothing would be left of a file without a text (ADR 0020).
    expect(notes.mayRemoveAttachment(note.id, zip), isFalse);
    expect(() => notes.removeAttachment(note.id, zip), throwsStateError);
    notes.removeAttachmentText(note.id, bill);
    var read = notes.byId(note.id)!;
    expect(read.removedAttachmentTexts, [bill]);
    expect(read.attachments.any((a) => a.hash == bill), isTrue);
    expect(notes.fileBytes(bill), isNotNull);
    expect(hashes(notes.unreadAttachments(computer: true)), isNot([bill]));
    expect(notes.mayRemoveAttachmentText(note.id, bill), isFalse);
    notes.removeAttachment(note.id, lease);
    read = notes.byId(note.id)!;
    expect(read.attachments.any((a) => a.hash == lease), isFalse);
    expect([for (final a in read.removedAttachments) a.name], ['Lease.txt']);
    expect(read.attachmentTexts[lease], 'Rent due');
    expect(notes.fileBytes(lease), isNull, reason: 'its bytes go');
    expect(ann.knowsBlob('file/$lease'), isFalse);
    expect(ann.missingBlobs(), isEmpty);
    notes.setAttachmentText(note.id, bill, 'Power bill, March');
    expect(notes.byId(note.id)!.removedAttachmentTexts, isEmpty);
  });

  test('what another Device wrote of a file is read as data only', () {
    final hash = 'a' * 64;
    Attachment read(String value) => Attachment.read(hash, value);
    final plain = read(
      '{"name":"../../.bashrc","size":5,"type":"Text/Plain; charset=utf-8"}',
    );
    expect((plain.name, plain.mediaType), ('.bashrc', 'text/plain'));
    expect((plain.size, plain.pages, plain.computer), (5, null, false));
    final crafted = read(
      r'{"computer":"yes","name":"inv\u202Efdp.exe\u0007","pages":1.0,'
      r'"size":-3,"type":"<script>"}',
    );
    expect(crafted.name, 'invfdp.exe');
    expect(crafted.mediaType, unknownType);
    expect((crafted.size, crafted.pages, crafted.computer), (0, null, false));
    final huge = read(
      '{"name":"..","pages":9007199254740993,"size":9007199254740992}',
    );
    expect((huge.name, huge.pages, huge.size), ('', null, 1 << 53));
    for (final garbled in ['', 'null', '[1]', '"name"', '{"name":3}']) {
      final a = read(garbled);
      expect((a.name, a.mediaType, a.size), ('', unknownType, 0));
    }
    expect(cleanName(r'C:\Users\ann\ Brief.pdf '), 'Brief.pdf');
    expect(cleanName('ä' * 300).runes, hasLength(255));
    expect(
      cleanType('application/vnd.oasis.opendocument.text'),
      'application/vnd.oasis.opendocument.text',
    );
    expect(cleanType('text'), unknownType);
    expect(cleanType('tëxt/plain'), unknownType);
  });

  test('attachment texts and file names are searched and a removed text is '
      'not', () {
    final letter = notes.capture('', files: [_file('Lease.txt', 'lease')]);
    final hash = letter.attachments.single.hash;
    notes.setAttachmentText(letter.id, hash, 'Rent due on the first');
    final target = notes.capture('Papers');
    final merged = notes.capture('', files: [_file('plan.dwg', 'dwg')]);
    notes.merge(merged.id, target.id);
    expect(found(ann, 'RENT DUE'), [letter.id]);
    expect(found(ann, 'lease.t'), [letter.id]);
    expect(found(ann, 'plan.dwg'), [target.id]);
    notes.removeAttachmentText(letter.id, hash);
    expect(found(ann, 'rent'), isEmpty);
    expect(found(ann, 'lease'), [letter.id]);
  });

  test('a file and its text travel to the other Device', () {
    final desk = Store.inMemory(clock: clock)
      ..applyProfileDefaults(name: 'Ann');
    addTearDown(desk.close);
    syncAll(desk, ann);
    final scan = notes.capture(
      '',
      files: [_file('Scan.pdf', '%PDF-1.4 thick', pages: 30)],
    );
    final hash = scan.attachments.single.hash;
    expect(
      notes.unreadAttachments(computer: false),
      isEmpty,
      reason: 'left to the computer',
    );
    syncAll(desk, ann);
    expect(desk.missingBlobs(), isEmpty);
    expect(Notes(desk).fileBytes(hash), _b('%PDF-1.4 thick'));
    expect(hashes(Notes(desk).unreadAttachments(computer: true)), [hash]);
    Notes(desk).setAttachmentText(scan.id, hash, 'Page one\n\nPage two');
    syncAll(desk, ann);
    expect(notes.byId(scan.id)!.attachmentTexts[hash], 'Page one\n\nPage two');
  });

  test('a Member\'s machine gives another\'s file its text and only the '
      'owner removes it', () {
    final ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
    addTearDown(ben.close);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    final lease = notes.capture(
      'Lease',
      place: area,
      files: [_file('Lease.docx', 'PK lease'), _file('Bill.txt', 'bill')],
    );
    final [docx, bill] = [for (final a in lease.attachments) a.hash];
    notes.setAttachmentText(lease.id, bill, 'Power bill');
    syncAll(ben, ann);
    // Ben's machine reads the file for Ann's Note.
    Notes(ben).setAttachmentText(lease.id, docx, 'Rent due');
    expect(Notes(ben).mayRemoveAttachmentText(lease.id, bill), isFalse);
    // As a modified app would write them: a removal and a file of his.
    ben.append(lease.id, NoteKeys.fileText(bill), null);
    ben.append(
      lease.id,
      NoteKeys.file('b' * 64),
      '{"name":"x.exe","size":1,"type":"application/x"}',
    );
    syncAll(ben, ann);
    for (final store in [ann, ben]) {
      final read = Notes(store).byId(lease.id)!;
      expect(read.attachmentTexts[docx], 'Rent due');
      expect(read.attachmentTexts[bill], 'Power bill');
      expect(read.removedAttachmentTexts, isEmpty);
      expect(read.attachments, hasLength(2), reason: 'his file is ignored');
    }
  });

  test('an attached file goes in a Pack with its name only when ticked and '
      'its text without it', () {
    final ben = device(clock, 'Ben');
    final papers = Places(ann).addArea('Papers').id;
    final lease = notes.capture(
      'Lease',
      place: papers,
      files: [
        _file(
          'Lease.pdf',
          '%PDF-1.4 lease',
          pages: 12,
          type: 'application/pdf',
        ),
        _file('plan.dwg', 'dwg'),
      ],
    );
    final pdf = lease.attachments.first.hash;
    notes.setAttachmentText(lease.id, pdf, 'Rent due');
    final pack = ann.packOf(lease.id);
    expect(
      [for (final c in pack.tree().single.children.single.children) c.kind],
      [PackItemKind.file, PackItemKind.fileText, PackItemKind.file],
    );
    final ticks = pack.exportTicks();
    expect(ticks, contains(PackItemKeys.fileText(0, 0)));
    expect(ticks, isNot(contains(PackItemKeys.file(0, 0))));
    final opened = ben.openPack(ann.writePack(pack, ticks));
    final files = opened.pack.notes.single.attachments;
    expect(files, hasLength(1), reason: 'the file without a text stays out');
    expect(files.single.hasFile, isFalse);
    expect(files.single.text, 'Rent due');
    void add(OpenedPack opened) => ben.addPack(
      opened,
      opened.pack.allKeys(),
      claim: true,
      wallClock: _wall,
      suffix: 'Pack',
      noArea: 'From a Pack',
    );
    add(opened);
    final added = Notes(ben).all().single;
    expect(added.attachments, isEmpty);
    expect(added.removedAttachments, hasLength(1));
    expect(added.attachmentTexts.values, ['Rent due']);

    add(ben.openPack(ann.writePack(pack, pack.allKeys())));
    final second = Notes(ben).all().firstWhere((n) => n.attachments.isNotEmpty);
    expect(
      [for (final a in second.attachments) a.name],
      ['Lease.pdf', 'plan.dwg'],
    );
    expect(second.attachments.first.pages, 12);
    expect(second.attachments.first.computer, isTrue);
    expect(second.attachments[1].mediaType, 'application/octet-stream');
    expect(ben.file(pdf), _b('%PDF-1.4 lease'));
    expect(second.attachmentTexts[pdf], 'Rent due');
  });

  test('the local network carries a file as large as a bundle does', () {
    expect(maxBlobBytes, maxBundleBlobBytes);
    expect(maxBlobBytes, maxFileBytes);
  });
}
