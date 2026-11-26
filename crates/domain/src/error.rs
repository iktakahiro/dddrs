//! Shared domain-level persistence error definitions.

use std::error::Error;
use thiserror::Error;

/// Errors returned by repository and transactional persistence ports.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Resource changed since it was read")]
    Conflict,
    #[error("Repository operation failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}
