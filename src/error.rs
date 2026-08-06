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
    InvalidRepo(String),

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

    #[error("incorrect password")]
    IncorrectPassword,

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

impl Error {
    /// Whether this error (or one it wraps via [`Error::Context`]) is an
    /// [`Error::IncorrectPassword`] — i.e. decryption failed specifically
    /// because the passphrase was wrong, not because the file was missing,
    /// corrupt, or unreadable for some other reason.
    pub fn is_password_error(&self) -> bool {
        match self {
            Error::IncorrectPassword => true,
            Error::Context { source, .. } => source.is_password_error(),
            _ => false,
        }
    }
}

impl From<keyring_core::Error> for Error {
    /// Wrap a system keychain lookup/store failure.
    fn from(e: keyring_core::Error) -> Self {
        Error::Keyring(e.to_string())
    }
}

impl From<age::EncryptError> for Error {
    /// Wrap an age encryption failure.
    fn from(e: age::EncryptError) -> Self {
        Error::Encryption(e.to_string())
    }
}

impl From<age::DecryptError> for Error {
    /// Wrap an age decryption failure, distinguishing a wrong passphrase
    /// ([`Error::IncorrectPassword`]) from other decrypt failures.
    fn from(e: age::DecryptError) -> Self {
        match e {
            // Passphrase-based (scrypt) recipients report a wrong password
            // as a generic AEAD decryption failure, since age can't tell a
            // bad key from tampered ciphertext.
            age::DecryptError::DecryptionFailed => Error::IncorrectPassword,
            other => Error::Encryption(other.to_string()),
        }
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

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::*;

    #[test]
    fn profile_not_found_displays_profile_name() {
        assert_eq!(
            Error::ProfileNotFound("work".to_string()).to_string(),
            "profile 'work' does not exist"
        );
    }

    #[test]
    fn empty_password_has_fixed_message() {
        assert_eq!(Error::EmptyPassword.to_string(), "password cannot be empty");
    }

    #[test]
    fn invalid_repo_spec_displays_the_offending_spec() {
        assert_eq!(
            Error::InvalidRepo("nope".to_string()).to_string(),
            "'nope' is not a valid GitHub repo; use a URL or `owner/repo`"
        );
    }

    #[test]
    fn context_error_displays_its_message_not_the_source() {
        let err = Error::Context {
            msg: "failed to read config".to_string(),
            source: Box::new(Error::EmptyPassword),
        };
        assert_eq!(err.to_string(), "failed to read config");
    }

    #[test]
    fn wrap_err_attaches_message_and_preserves_source() {
        let result: std::result::Result<(), Error> =
            Err(Error::EmptyPassword).wrap_err("could not load profile");

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "could not load profile");
        match &err {
            Error::Context { msg, source } => {
                assert_eq!(msg, "could not load profile");
                assert!(matches!(**source, Error::EmptyPassword));
            }
            other => panic!("expected Error::Context, got {other:?}"),
        }
        assert_eq!(
            StdError::source(&err).unwrap().to_string(),
            "password cannot be empty"
        );
    }

    #[test]
    fn wrap_err_with_computes_message_lazily() {
        let result: std::result::Result<(), Error> =
            Err(Error::EmptyPassword).wrap_err_with(|| format!("attempt {}", 1 + 1));

        assert_eq!(result.unwrap_err().to_string(), "attempt 2");
    }

    #[test]
    fn wrap_err_passes_through_ok_values_unchanged() {
        let result: std::result::Result<i32, std::io::Error> = Ok(42);
        assert_eq!(result.wrap_err("unused").unwrap(), 42);
    }

    #[test]
    fn encrypt_error_converts_to_encryption_variant() {
        let err: Error = age::EncryptError::MissingRecipients.into();
        assert!(matches!(err, Error::Encryption(_)));
    }

    #[test]
    fn decrypt_error_decryption_failed_maps_to_incorrect_password() {
        let err: Error = age::DecryptError::DecryptionFailed.into();
        assert!(matches!(err, Error::IncorrectPassword));
    }

    #[test]
    fn other_decrypt_errors_map_to_encryption_variant() {
        let err: Error = age::DecryptError::InvalidHeader.into();
        assert!(matches!(err, Error::Encryption(_)));

        let err: Error = age::DecryptError::InvalidMac.into();
        assert!(matches!(err, Error::Encryption(_)));
    }

    #[test]
    fn is_password_error_true_for_incorrect_password() {
        assert!(Error::IncorrectPassword.is_password_error());
    }

    #[test]
    fn is_password_error_true_when_wrapped_once_in_context() {
        let err = Error::Context {
            msg: "decrypting bundle".to_string(),
            source: Box::new(Error::IncorrectPassword),
        };
        assert!(err.is_password_error());
    }

    #[test]
    fn is_password_error_true_when_wrapped_twice_in_context() {
        let err = Error::Context {
            msg: "outer".to_string(),
            source: Box::new(Error::Context {
                msg: "inner".to_string(),
                source: Box::new(Error::IncorrectPassword),
            }),
        };
        assert!(err.is_password_error());
    }

    #[test]
    fn is_password_error_false_for_unrelated_error() {
        assert!(!Error::EmptyPassword.is_password_error());
    }

    #[test]
    fn is_password_error_false_for_wrapped_io_error() {
        let io_err = std::io::Error::other("disk full");
        let err = Error::Context {
            msg: "reading file".to_string(),
            source: Box::new(Error::Io(io_err)),
        };
        assert!(!err.is_password_error());
    }
}
