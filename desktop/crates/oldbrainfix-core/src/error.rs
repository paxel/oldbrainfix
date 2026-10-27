/// What can go wrong reading or writing the log.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    /// Keeping a photo's or a voice recording's bytes.
    #[error("file: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
    /// A file that is no sync bundle this app can read.
    #[error("no sync bundle")]
    NotABundle,
    /// A sync bundle holding only formats newer than this version reads,
    /// as the Dart core's `BundleNeedsUpdate` (ADR 0019): this app needs
    /// an update.
    #[error("a sync bundle of a newer version")]
    BundleNeedsUpdate,
    /// Blocking a person who manages these Shared Areas leaves them, and
    /// was not asked to (ADR 0021): nothing changed.
    #[error("blocking leaves the Shared Areas the person manages")]
    MustLeaveAreas(Vec<String>),
    /// A Sent Item from a person the user blocked (ADR 0021).
    #[error("a Sent Item from a blocked person")]
    FromBlocked,
    /// A Backup that could not be restored, and why.
    #[error("backup: {0:?}")]
    Backup(crate::backup::Problem),
}

pub type Result<T> = std::result::Result<T, Error>;
