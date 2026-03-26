use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Docker command failed: {0}")]
    DockerCommand(String),

    #[error("Failed to parse Docker output: {0}")]
    DockerParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
