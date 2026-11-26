mod repository;
mod row;

pub use repository::SqliteTodoRepository;

use domain::todo::RepositoryError;
use std::error::Error;

fn storage(error: impl Error + Send + Sync + 'static) -> RepositoryError {
    RepositoryError::Storage(Box::new(error))
}
