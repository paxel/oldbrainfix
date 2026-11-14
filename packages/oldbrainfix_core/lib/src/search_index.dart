import 'package:meta/meta.dart';
import 'package:sqlite3/sqlite3.dart';

import 'notes.dart';
import 'store.dart';
import 'tallies.dart';
import 'text_boxes.dart';
import 'text_format.dart';

/// The full-text index of what search reads (ADR 0048), the same in
/// both cores (`search_index.rs`): beside the log in the same database,
/// a row for every entry that wrote a text search reads, in lower case
/// as search lowers it, in an FTS5 table with the trigram tokenizer.
///
/// It only narrows: it names every entity one of whose texts ever held a
/// query, and search reads those as they are now, so an entry that no
/// longer counts, an older value or one a role leaves out is no more
/// than a candidate too many. Before each search it takes in the entries
/// written since it last did, by the log's high-water mark, whichever
/// connection wrote them, and it is built anew when its tables or its
/// mark are missing or of another version.
@internal
abstract final class SearchIndex {
  /// The version of what the rows hold; another one is built anew.
  static const version = 1;

  /// The entities whose texts, at some time, held [wanted], in lower
  /// case and not blank, and those the shown pending sets change: every
  /// entity whose text holds it now, and maybe more. Null when the index
  /// cannot be used, such as a SQLite without FTS5: search then reads
  /// every Note.
  static Set<String>? candidates(Store store, String wanted) {
    final db = store.database;
    try {
      _catchUp(db);
      final rows = wanted.runes.length >= 3
          ? db.select(
              'SELECT DISTINCT entity FROM search_texts '
              'WHERE search_texts MATCH ? AND instr(text, ?) > 0',
              ['"${wanted.replaceAll('"', '""')}"', wanted],
            )
          // Trigrams find nothing shorter than three characters.
          : db.select(
              'SELECT DISTINCT entity FROM search_texts '
              'WHERE instr(text, ?) > 0',
              [wanted],
            );
      return {for (final r in rows) r['entity'] as String}
        ..addAll(store.shownEntities);
    } on Exception {
      // A SQLite without FTS5, a database it cannot write: as before.
      return null;
    }
  }

  /// What the index holds of the entry that wrote [value] to [field], in
  /// lower case: every way search reads such a text, a line apart, or
  /// null for a field search does not read. A `text` is read as a Note's
  /// with its boxes as written, a Section's and a Line's.
  static String? textOf(String field, String? value) {
    if (value == null) return null;
    if (field == NoteKeys.text) {
      final stored = StoredText.read(value);
      final shown = stored.editable
          ? TextBoxes.strip(stored.text)
          : stored.text;
      return {
        stored.visible,
        stored.text,
        TextFormats.visible(stored.format, shown),
      }.join('\n').toLowerCase();
    }
    if (field == TallyKeys.line ||
        field.startsWith(NoteKeys.transcriptPrefix) ||
        field.startsWith(NoteKeys.ocrPrefix) ||
        field.startsWith(NoteKeys.fileTextPrefix)) {
      return value.toLowerCase();
    }
    if (field.startsWith(NoteKeys.filePrefix)) {
      final hash = field.substring(NoteKeys.filePrefix.length);
      return Attachment.read(hash, value).name.toLowerCase();
    }
    return null;
  }

  /// Takes in the entries written since the index's mark, or all of them
  /// when it is missing or of another version, in one transaction (or
  /// the batch it runs in).
  static void _catchUp(Database db) {
    if (_held(db) == _newest(db)) return;
    final own = db.autocommit;
    if (own) db.execute('BEGIN IMMEDIATE');
    try {
      // Read again under the write lock: another connection may have
      // written, or caught up, since.
      final newest = _newest(db);
      var held = _held(db);
      if (held == null || held > newest) {
        db.execute('''
          DROP TABLE IF EXISTS search_texts;
          CREATE VIRTUAL TABLE search_texts USING fts5(
            entity UNINDEXED, text, tokenize = 'trigram case_sensitive 1'
          );
          CREATE TABLE IF NOT EXISTS search_state (
            key   TEXT PRIMARY KEY,
            value INTEGER NOT NULL
          );
          DELETE FROM search_state;
        ''');
        db.execute(
          "INSERT INTO search_state (key, value) VALUES ('version', ?)",
          [version],
        );
        held = 0;
      }
      if (held < newest) {
        final add = db.prepare(
          'INSERT INTO search_texts (entity, text) VALUES (?, ?)',
        );
        try {
          final rows = db.select(
            'SELECT entity, field, value FROM entries '
            'WHERE seq > ? AND seq <= ? AND value IS NOT NULL AND $_read',
            [held, newest],
          );
          for (final r in rows) {
            final text = textOf(r['field'] as String, r['value'] as String?);
            if (text != null && text.isNotEmpty) {
              add.execute([r['entity'] as String, text]);
            }
          }
        } finally {
          add.close();
        }
      }
      db.execute(
        "INSERT OR REPLACE INTO search_state (key, value) VALUES ('seq', ?)",
        [newest],
      );
      if (own) db.execute('COMMIT');
    } catch (_) {
      if (own) db.execute('ROLLBACK');
      rethrow;
    }
  }

  /// The fields [textOf] reads, as SQL.
  static const _read =
      "(field IN ('${NoteKeys.text}', '${TallyKeys.line}') "
      "OR substr(field, 1, ${NoteKeys.transcriptPrefix.length}) = "
      "'${NoteKeys.transcriptPrefix}' "
      "OR substr(field, 1, ${NoteKeys.ocrPrefix.length}) = "
      "'${NoteKeys.ocrPrefix}' "
      "OR substr(field, 1, ${NoteKeys.filePrefix.length}) = "
      "'${NoteKeys.filePrefix}' "
      "OR substr(field, 1, ${NoteKeys.fileTextPrefix.length}) = "
      "'${NoteKeys.fileTextPrefix}')";

  static int _newest(Database db) =>
      db.select('SELECT COALESCE(MAX(seq), 0) AS n FROM entries').first['n']
          as int;

  /// The mark the index holds, or null when its tables, its mark or its
  /// version are missing or another.
  static int? _held(Database db) {
    final tables = db.select(
      "SELECT name FROM sqlite_master WHERE name IN "
      "('search_texts', 'search_state')",
    );
    if (tables.length != 2) return null;
    final state = {
      for (final r in db.select('SELECT key, value FROM search_state'))
        r['key'] as String: r['value'] as int,
    };
    if (state['version'] != version) return null;
    return state['seq'];
  }
}
