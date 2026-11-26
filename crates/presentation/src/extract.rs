use crate::error::ApiError;
use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Request},
    http::{StatusCode, request::Parts},
};
use domain::todo::{Description, Title, TodoId};
use serde::Deserialize;

pub(crate) struct TodoPath(pub TodoId);

impl<S: Send + Sync> FromRequestParts<S> for TodoPath {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        let Path(raw) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError(e.status(), e.body_text()))?;
        raw.parse()
            .map(Self)
            .map_err(|_| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "Invalid Todo UUID".into()))
    }
}

pub(crate) struct TodoInput {
    pub title: Title,
    pub description: Option<Description>,
}

impl<S: Send + Sync> FromRequest<S> for TodoInput {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, ApiError> {
        #[derive(Deserialize)]
        struct Payload {
            title: String,
            description: Option<String>,
        }
        let Json(raw) = Json::<Payload>::from_request(request, state)
            .await
            .map_err(|e| ApiError(e.status(), e.body_text()))?;
        Ok(Self {
            title: raw.title.try_into()?,
            description: raw
                .description
                .filter(|s| !s.is_empty())
                .map(Description::try_from)
                .transpose()?,
        })
    }
}
