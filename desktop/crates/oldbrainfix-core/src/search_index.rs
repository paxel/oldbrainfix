//! The full-text index of what search reads (ADR 0048), as the Dart
//! core's `SearchIndex` keeps it: beside the log in the same database, a
//! row for every entry that wrote a text search reads, in lower case as
//! search lowers it, in an FTS5 table with the trigram tokenizer.
//!
//! It only narrows: it names every entity one of whose texts ever held a
//! query, and search reads those as they are now, so an entry that no
//! longer counts, an older value or one a role leaves out is no more
//! than a candidate too many. Before each search it takes in the entries
//! written since it last did, by the log's high-water mark, whichever
//! connection wrote them, and it is built anew when its tables or its
//! mark are missing or of another version.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};

use crate::Result;
use crate::notes::{Attachment, keys};
use crate::search::lower_case;
use crate::store::Store;
use crate::tallies::keys as tally_keys;
use crate::text_boxes;
use crate::text_format::{self, StoredText};

/// The version of what the rows hold; another one is built anew.
pub(crate) const VERSION: i64 = 1;

/// The entities whose texts, at some time, held `wanted`, in lower case
/// and not blank, and those the shown pending sets change: every entity
/// whose text holds it now, and maybe more. `None` when the index cannot
/// be used, such as a SQLite without FTS5: search then reads every Note.
pub(crate) fn candidates(store: &Store, wanted: &str) -> Option<HashSet<String>> {
    let mut found = store
        .with_connection(|db| {
            catch_up(db)?;
            matching(db, wanted)
        })
        .ok()?;
    found.extend(store.shown().ok()?.iter().map(|e| e.entity.clone()));
    Some(found)
}

/// What the index holds of the entry that wrote `value` to `field`, in
/// lower case: every way search reads such a text, a line apart, or
/// `None` for a field search does not read. A `text` is read as a Note's
/// with its boxes as written, a Section's and a Line's.
pub(crate) fn text_of(field: &str, value: &str) -> Option<String> {
    if field == keys::TEXT {
        let stored = StoredText::read(Some(value));
        let shown = if stored.editable {
            text_boxes::strip(&stored.text)
        } else {
            stored.text.clone()
        };
        let mut forms: Vec<String> = Vec::new();
        for form in [
            stored.visible(),
            stored.text.clone(),
            text_format::visible(&stored.format, &shown),
        ] {
            if !forms.contains(&form) {
                forms.push(form);
            }
        }
        return Some(lower_case(&forms.join("\n")));
    }
    if field == tally_keys::LINE
        || field.starts_with(keys::TRANSCRIPT_PREFIX)
        || field.starts_with(keys::OCR_PREFIX)
        || field.starts_with(keys::FILE_TEXT_PREFIX)
    {
        return Some(lower_case(value));
    }
    field
        .strip_prefix(keys::FILE_PREFIX)
        .map(|hash| lower_case(&Attachment::read(hash, value).name))
}

/// The entities of the rows holding `wanted`.
fn matching(db: &Connection, wanted: &str) -> Result<HashSet<String>> {
    let mut found = HashSet::new();
    if wanted.chars().count() >= 3 {
        let phrase = format!("\"{}\"", wanted.replace('"', "\"\""));
        let mut stmt = db.prepare(
            "SELECT DISTINCT entity FROM search_texts \
             WHERE search_texts MATCH ? AND instr(text, ?) > 0",
        )?;
        for entity in stmt.query_map(params![phrase, wanted], |r| r.get(0))? {
            found.insert(entity?);
        }
    } else {
        // Trigrams find nothing shorter than three characters.
        let mut stmt =
            db.prepare("SELECT DISTINCT entity FROM search_texts WHERE instr(text, ?) > 0")?;
        for entity in stmt.query_map(params![wanted], |r| r.get(0))? {
            found.insert(entity?);
        }
    }
    Ok(found)
}

/// Takes in the entries written since the index's mark, or all of them
/// when it is missing or of another version, in one transaction (or the
/// batch it runs in).
fn catch_up(db: &Connection) -> Result<()> {
    if held(db)? == Some(newest(db)?) {
        return Ok(());
    }
    if !db.is_autocommit() {
        return fill(db);
    }
    // Rolled back when it is dropped before the commit.
    let tx = rusqlite::Transaction::new_unchecked(db, rusqlite::TransactionBehavior::Immediate)?;
    fill(db)?;
    tx.commit()?;
    Ok(())
}

fn fill(db: &Connection) -> Result<()> {
    // Read again under the write lock: another connection may have
    // written, or caught up, since.
    let newest = newest(db)?;
    let held = match held(db)? {
        Some(held) if held <= newest => held,
        _ => {
            db.execute_batch(
                "DROP TABLE IF EXISTS search_texts;
                 CREATE VIRTUAL TABLE search_texts USING fts5(
                   entity UNINDEXED, text, tokenize = 'trigram case_sensitive 1'
                 );
                 CREATE TABLE IF NOT EXISTS search_state (
                   key   TEXT PRIMARY KEY,
                   value INTEGER NOT NULL
                 );
                 DELETE FROM search_state;",
            )?;
            db.execute(
                "INSERT INTO search_state (key, value) VALUES ('version', ?)",
                params![VERSION],
            )?;
            0
        }
    };
    if held < newest {
        let mut add = db.prepare("INSERT INTO search_texts (entity, text) VALUES (?, ?)")?;
        let mut stmt = db.prepare(&format!(
            "SELECT entity, field, value FROM entries \
             WHERE seq > ? AND seq <= ? AND value IS NOT NULL AND {}",
            read_fields()
        ))?;
        let mut rows = stmt.query(params![held, newest])?;
        while let Some(row) = rows.next()? {
            let entity: String = row.get(0)?;
            let field: String = row.get(1)?;
            let value: String = row.get(2)?;
            if let Some(text) = text_of(&field, &value)
                && !text.is_empty()
            {
                add.execute(params![entity, text])?;
            }
        }
    }
    db.execute(
        "INSERT OR REPLACE INTO search_state (key, value) VALUES ('seq', ?)",
        params![newest],
    )?;
    Ok(())
}

/// The fields [`text_of`] reads, as SQL.
fn read_fields() -> String {
    let prefix = |p: &str| format!("substr(field, 1, {}) = '{p}'", p.chars().count());
    format!(
        "(field IN ('{}', '{}') OR {} OR {} OR {} OR {})",
        keys::TEXT,
        tally_keys::LINE,
        prefix(keys::TRANSCRIPT_PREFIX),
        prefix(keys::OCR_PREFIX),
        prefix(keys::FILE_PREFIX),
        prefix(keys::FILE_TEXT_PREFIX),
    )
}

fn newest(db: &Connection) -> Result<i64> {
    Ok(
        db.query_row("SELECT COALESCE(MAX(seq), 0) FROM entries", [], |r| {
            r.get(0)
        })?,
    )
}

/// The mark the index holds, or `None` when its tables, its mark or its
/// version are missing or another.
fn held(db: &Connection) -> Result<Option<i64>> {
    let tables: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_master WHERE name IN ('search_texts', 'search_state')",
        [],
        |r| r.get(0),
    )?;
    if tables != 2 {
        return Ok(None);
    }
    let mut stmt = db.prepare("SELECT key, value FROM search_state")?;
    let state: HashMap<String, i64> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if state.get("version") != Some(&VERSION) {
        return Ok(None);
    }
    Ok(state.get("seq").copied())
}

#[cfg(test)]
mod tests;
