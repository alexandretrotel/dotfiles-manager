use std::path::PathBuf;

use thiserror::Error;

/// Result alias used across the library.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All errors the library can produce.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("command '{cmd}' failed with status {status:?}: {stderr}")]
    CommandFailure {
        cmd: String,
        status: Option<i32>,
        stderr: String,
    },

    #[error("no git repository found in {}", path.display())]
    NoGitRepository { path: PathBuf },

    #[error("{} already exists and is not empty", path.display())]
    DataDirAlreadyExists { path: PathBuf },

    #[error("'{0}' is not a valid GitHub repo; use a URL or `owner/repo`")]
    InvalidRepoSpec(String),

    #[error("profile '{0}' does not exist")]
    ProfileNotFound(String),

    #[error("profile '{0}' already exists")]
    ProfileExists(String),

    #[error("profile name cannot be empty")]
    EmptyProfileName,

    #[error("profile name can only contain letters, numbers, hyphens, and underscores")]
    InvalidProfileName(String),

    #[error("cannot delete active profile '{0}'; switch to another profile first")]
    DeleteActiveProfile(String),

    #[error("password cannot be empty")]
    EmptyPassword,

    #[error("keychain error: {0}")]
    Keyring(String),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("could not determine home directory")]
    NoHomeDir,

    /// An error with an attached human-readable context message, preserving
    /// the original error as its source (see `WrapErr`).
    #[error("{msg}")]
    Context {
        msg: String,
        #[source]
        source: Box<Error>,
    },

    #[error("{0}")]
    Message(String),
}

impl From<keyring_core::Error> for Error {
    fn from(e: keyring_core::Error) -> Self {
        Error::Keyring(e.to_string())
    }
}

impl From<age::EncryptError> for Error {
    fn from(e: age::EncryptError) -> Self {
        Error::Encryption(e.to_string())
    }
}

impl From<age::DecryptError> for Error {
    fn from(e: age::DecryptError) -> Self {
        Error::Encryption(e.to_string())
    }
}

/// Attach a context message to an error, preserving the cause chain.
pub(crate) trait WrapErr<T> {
    /// Wrap the error (if any) with a fixed context message.
    fn wrap_err(self, msg: impl Into<String>) -> Result<T>;
    /// Wrap the error (if any) with a lazily-computed context message.
    fn wrap_err_with(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E: Into<Error>> WrapErr<T> for std::result::Result<T, E> {
    fn wrap_err(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|e| Error::Context {
            msg: msg.into(),
            source: Box::new(e.into()),
        })
    }

    fn wrap_err_with(self, f: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|e| Error::Context {
            msg: f(),
            source: Box::new(e.into()),
        })
    }
}
