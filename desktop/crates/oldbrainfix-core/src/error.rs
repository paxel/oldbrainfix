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
}

pub type Result<T> = std::result::Result<T, Error>;
