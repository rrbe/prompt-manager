use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("prompt not found: {0}")]
    PromptNotFound(String),

    #[error("prompt already exists: {0}")]
    PromptAlreadyExists(String),

    #[error("missing variable: {0}")]
    MissingVariable(String),

    #[error("duplicate variable source: {0}")]
    DuplicateVariableSource(String),

    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("editor exited with status {0}")]
    EditorFailed(std::process::ExitStatus),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid YAML front matter: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

impl Error {
    pub fn is_broken_pipe(&self) -> bool {
        matches!(self, Self::Io(error) if error.kind() == io::ErrorKind::BrokenPipe)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
