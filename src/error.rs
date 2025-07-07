use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Archive error: {0}")]
    Archive(#[from] anyhow::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Package version not found in PKGBUILD")]
    VersionNotFound,

    #[error("SSH authentication failed")]
    SshAuthFailed,
}

pub type Result<T> = std::result::Result<T, AppError>;
