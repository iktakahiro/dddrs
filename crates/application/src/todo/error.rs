use domain::{error::RepositoryError, todo::TransitionError};
use std::error::Error;
use thiserror::Error;

/// Application-layer error returned by Todo use cases.
///
/// Persistence errors are translated here so presentation does not depend on
/// `RepositoryError`.
#[derive(Debug, Error)]
pub enum UseCaseError {
    #[error("Todo not found")]
    NotFound,
    #[error(transparent)]
    Transition(#[from] TransitionError),
    #[error("Todo changed since it was read")]
    Conflict,
    #[error("Unexpected repository failure")]
    Unexpected(#[source] Box<dyn Error + Send + Sync>),
}

impl From<RepositoryError> for UseCaseError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::Conflict => Self::Conflict,
            RepositoryError::Storage(source) => Self::Unexpected(source),
        }
    }
}
