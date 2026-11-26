use crate::error::ApiError;
use application::todo::UseCaseError;
use axum::http::StatusCode;
use domain::todo::{RepositoryError, ValidationError};

impl From<ValidationError> for ApiError {
    fn from(error: ValidationError) -> Self {
        Self(StatusCode::UNPROCESSABLE_ENTITY, error.to_string())
    }
}

impl From<UseCaseError> for ApiError {
    fn from(error: UseCaseError) -> Self {
        let status = match &error {
            UseCaseError::NotFound => StatusCode::NOT_FOUND,
            UseCaseError::Transition(_) => StatusCode::BAD_REQUEST,
            UseCaseError::Repository(RepositoryError::Conflict) => StatusCode::CONFLICT,
            UseCaseError::Repository(RepositoryError::Storage(cause)) => {
                eprintln!("Repository error: {cause}");
                return Self(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".into(),
                );
            }
        };
        Self(status, error.to_string())
    }
}
