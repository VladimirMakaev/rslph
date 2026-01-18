use std::fmt;
use thiserror::Error;

/// VCS operation errors.
#[derive(Debug)]
pub enum VcsError {
    /// A VCS command failed to execute.
    CommandFailed { command: String, error: String },
    /// No changes to commit.
    NothingToCommit,
    /// Commit operation failed.
    CommitFailed(String),
    /// VCS detection failed.
    Detection(String),
}

impl fmt::Display for VcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VcsError::CommandFailed { command, error } => {
                write!(f, "VCS command '{}' failed: {}", command, error)
            }
            VcsError::NothingToCommit => write!(f, "Nothing to commit"),
            VcsError::CommitFailed(msg) => write!(f, "Commit failed: {}", msg),
            VcsError::Detection(msg) => write!(f, "VCS detection failed: {}", msg),
        }
    }
}

impl std::error::Error for VcsError {}

#[derive(Error, Debug)]
pub enum RslphError {
    #[error("Configuration error: {0}")]
    Config(#[from] Box<figment::Error>),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Progress file parse error: {0}")]
    ProgressParse(String),

    #[error("Subprocess error: {0}")]
    Subprocess(String),

    #[error("Process timeout after {0} seconds")]
    Timeout(u64),

    #[error("Process cancelled by user")]
    Cancelled,

    #[error("VCS error: {0}")]
    Vcs(#[from] VcsError),
}

impl From<figment::Error> for RslphError {
    fn from(err: figment::Error) -> Self {
        RslphError::Config(Box::new(err))
    }
}
