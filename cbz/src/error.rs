use std::io;
use thiserror::Error;
use zip::result::ZipError;

#[derive(Error, Debug)]
pub enum CbzError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Zip error: {0}")]
    Zip(#[from] ZipError),

    #[error("Invalid image format")]
    InvalidImageFormat,

    #[error("Entry not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, CbzError>;
