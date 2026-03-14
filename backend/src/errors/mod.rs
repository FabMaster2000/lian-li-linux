use crate::daemon::DaemonError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {message}")]
    Unauthorized {
        message: String,
        www_authenticate: Option<String>,
    },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("daemon error: {0}")]
    DaemonMessage(String),
    #[error("daemon error: {0}")]
    DaemonTransport(#[from] DaemonError),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorInfo,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorInfo {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::DaemonMessage(_) => StatusCode::BAD_GATEWAY,
            ApiError::DaemonTransport(_) => StatusCode::BAD_GATEWAY,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Unauthorized { .. } => "UNAUTHORIZED",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::DaemonMessage(_) => "DAEMON_ERROR",
            ApiError::DaemonTransport(_) => "DAEMON_ERROR",
            ApiError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    fn details(&self) -> Value {
        match self {
            ApiError::Unauthorized { .. } => json!({ "source": "auth" }),
            ApiError::DaemonMessage(_) => json!({ "source": "daemon_response" }),
            ApiError::DaemonTransport(_) => json!({ "source": "daemon_transport" }),
            _ => json!({}),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let www_authenticate = match &self {
            ApiError::Unauthorized {
                www_authenticate,
                ..
            } => www_authenticate.clone(),
            _ => None,
        };
        let body = ApiErrorBody {
            error: ApiErrorInfo {
                code: self.code(),
                message: self.to_string(),
                details: self.details(),
            },
        };
        let mut response = (self.status(), Json(body)).into_response();
        if let Some(www_authenticate) = www_authenticate {
            response.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_str(&www_authenticate)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("Basic")),
            );
        }
        response
    }
}
