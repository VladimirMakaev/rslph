use thiserror::Error;

#[derive(Error, Debug)]
pub enum RslphError {
    #[error("Configuration error: {0}")]
    Config(#[from] figment::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Progress file parse error: {0}")]
    ProgressParse(String),
}
