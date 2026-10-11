use thiserror::Error;

use crate::backend;

#[derive(Debug, Error)]
pub enum OcsideError {
    #[error("Failed to initialise runtime")]
    RuntimeInitFailure,
    #[error("Invalid parameter")]
    InvalidParam,
    #[error("Invalid string")]
    InvalidString,
    #[error("Class not found")]
    ClassNotFound,
    #[error("Class already registered")]
    ClassRegistered,
    #[error("Failed to create the window")]
    WindowCreateFailed,
    #[error("Poll error response")]
    PollError,
    #[error("Backend error")]
    BackendError(#[from] backend::BackendError),
}

pub trait CaptureError<T, E> {
    fn last_error(self) -> Result<T, E>;
}

impl<T> CaptureError<T, OcsideError> for Result<T, OcsideError> {
    fn last_error(self) -> Result<T, OcsideError> {
        if let Err(err) = &self {
            println!("Error {}", err);
        }
        self
    }
}
