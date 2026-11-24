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
    /// An Area offered from afar that was not taken (#108), and why.
    #[error("area offer: {0:?}")]
    Offer(crate::offers::OfferError),
    /// A Backup that could not be restored, and why.
    #[error("backup: {0:?}")]
    Backup(crate::backup::Problem),
    /// A LAN sync this Device joined that did not happen, and why.
    #[error("sync: {0:?}")]
    Sync(crate::join::SyncFailure),
    /// Sync once waited while an AI Task did. It no longer does
    /// (obf-1076) and the core returns this nowhere; kept only while the
    /// desktop still matches it, to go with its last use.
    #[error("sync is paused until the pending changes are decided")]
    SyncPaused,
    /// The AI port was to let Docker containers connect, and this
    /// computer has no Docker bridge (`docker0`, #167): it stays closed.
    #[error("no Docker bridge (docker0) found")]
    NoDockerBridge,
    /// A database a newer version of the app saved, with the
    /// `user_version` it holds, as the Dart core's `StoreNeedsUpdate`
    /// (ADR 0034): it is not opened, and nothing in it changed.
    #[error("a database of a newer version ({0})")]
    StoreNeedsUpdate(i64),
    /// Sharing something Hidden from AI (#267, ADR 0049), as the Dart
    /// core's `HiddenFromAiRefused`: filing a marked Note into a Shared
    /// Area, sharing a hidden Area or one holding hidden Notes, sending a
    /// hidden Note as a Sent Item, or putting one into a Pack. Holds the
    /// Notes and Areas whose marks stand in the way, sorted; nothing
    /// changed.
    #[error("hidden from AI: {}", .0.join(", "))]
    HiddenFromAi(Vec<String>),
    /// A vault that was not rebuilt (#267, ADR 0050), and why: nothing
    /// was written.
    #[error("vault: {0:?}")]
    Vault(crate::vault::VaultProblem),
}

pub type Result<T> = std::result::Result<T, Error>;
