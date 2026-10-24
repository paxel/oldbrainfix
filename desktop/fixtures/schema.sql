CREATE TABLE entries (
        seq      INTEGER PRIMARY KEY AUTOINCREMENT,
        device   TEXT NOT NULL,
        dseq     INTEGER NOT NULL,
        entity   TEXT NOT NULL,
        field    TEXT NOT NULL,
        value    TEXT,
        date     TEXT NOT NULL,
        author   TEXT NOT NULL,
        recorded TEXT NOT NULL,
        sig      TEXT
      );
CREATE UNIQUE INDEX idx_entries_device_dseq
        ON entries (device, dseq);
CREATE INDEX idx_entries_entity_field
        ON entries (entity, field);
CREATE TABLE local_settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
