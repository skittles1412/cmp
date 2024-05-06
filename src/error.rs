use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::borrow::Cow;

// TODO: account for json deserialization error
pub enum ApiError {
    /// Something bad happened
    InternalServerError(anyhow::Error),
    /// The request is asking about an ID that doesn't exist
    NoSuchId,
    /// Bob has submitted a value already
    IdComparedAlready,
    /// Bob has not yet submitted a value
    IdNotCompared,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(value: E) -> Self {
        ApiError::InternalServerError(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct AppErrorRes {
            error: Cow<'static, str>,
        }

        match self {
            ApiError::InternalServerError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AppErrorRes {
                    error: err.to_string().into(),
                }),
            ),
            ApiError::NoSuchId => (
                StatusCode::BAD_REQUEST,
                Json(AppErrorRes {
                    error: "no comparison with the given id exists".into(),
                }),
            ),
            ApiError::IdComparedAlready => (
                StatusCode::BAD_REQUEST,
                Json(AppErrorRes {
                    error: "Bob has already entered his value for the given id".into(),
                }),
            ),
            ApiError::IdNotCompared => (
                StatusCode::BAD_REQUEST,
                Json(AppErrorRes {
                    error: "Bob has not entered his value for the given id".into(),
                }),
            ),
        }
        .into_response()
    }
}

pub struct AppError(pub anyhow::Error);

pub type AppResult<T> = Result<T, AppError>;

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(value: E) -> Self {
        AppError(value.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "INTERNAL SERVER ERROR (something has gone terribly wrong): {}",
                self.0
            ),
        )
            .into_response()
    }
}
