use std::fmt;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::contracts::{ApiErrorDetail, ApiErrorResponse};

#[derive(Debug)]
pub enum WebError {
    Unauthorized(&'static str),
    Forbidden(&'static str),
    RateLimited(&'static str),
    NotFound(&'static str),
    Conflict {
        message: &'static str,
        field: Option<&'static str>,
    },
    InvalidInput {
        message: &'static str,
        field: Option<&'static str>,
    },
    ValidationFailed(String),
    Internal(String),
}

impl WebError {
    pub fn invalid_input(field: &'static str, message: &'static str) -> Self {
        Self::InvalidInput {
            message,
            field: Some(field),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            Self::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::RateLimited(_) => "rate_limited",
            Self::NotFound(_) => "not_found",
            Self::Conflict { .. } => "conflict",
            Self::InvalidInput { .. } => "invalid_input",
            Self::ValidationFailed(_) => "validation_failed",
            Self::Internal(_) => "internal_error",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Unauthorized(message)
            | Self::Forbidden(message)
            | Self::RateLimited(message)
            | Self::NotFound(message) => message,
            Self::Conflict { message, .. }
            | Self::InvalidInput { message, .. } => message,
            Self::ValidationFailed(message) => message,
            Self::Internal(message) => message,
        }
    }

    fn field(&self) -> Option<&str> {
        match self {
            Self::Conflict { field, .. } | Self::InvalidInput { field, .. } => {
                field.map(str::trim).filter(|value| !value.is_empty())
            }
            _ => None,
        }
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for WebError {}

impl From<std::io::Error> for WebError {
    fn from(value: std::io::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<rusqlite::Error> for WebError {
    fn from(value: rusqlite::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<argon2::password_hash::Error> for WebError {
    fn from(value: argon2::password_hash::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<tokio::task::JoinError> for WebError {
    fn from(value: tokio::task::JoinError) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<reqwest::Error> for WebError {
    fn from(value: reqwest::Error) -> Self {
        Self::internal(value.to_string())
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ApiErrorResponse {
            error: ApiErrorDetail {
                code: self.code().to_string(),
                message: self.message().to_string(),
                field: self.field().map(ToString::to_string),
            },
        };
        (status, Json(body)).into_response()
    }
}
