use std::path::PathBuf;

use thiserror::Error;

use crate::blog::Field;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("post is empty")]
    EmptyPost,
    #[error("YAML header is missing")]
    MissingHeader,
    #[error("unable to parse {invalid} DateTime")]
    DateTimeParse{
        invalid: String,
        source: chrono::ParseError,
    },

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum UserError {
    #[error("provided repository url ({0}) is invalid")]
    InvalidRepoUrl(String),
}

#[derive(Debug, Error)]
pub enum BlogError {
    #[error("provided repository ({expected}) doesn't match existing one ({existing})")]
    RepoMismatch { expected: String, existing: String },
    #[error("provided root directory path doesn't exist or points to a file: {0}")]
    InvalidRoot(PathBuf),

    #[error(transparent)]
    Format(#[from] FormatError),
    #[error(transparent)]
    User(#[from] UserError),
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
