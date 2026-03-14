use crate::app::AppState;
use crate::config::AuthMode;
use crate::errors::ApiError;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, HeaderName};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

pub async fn require_auth<B>(
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, ApiError>
where
    B: Send + 'static,
{
    match state.config.auth.mode {
        AuthMode::None => Ok(next.run(request).await),
        AuthMode::Basic => {
            validate_basic_auth(&state, request.headers())?;
            Ok(next.run(request).await)
        }
        AuthMode::Bearer => {
            validate_bearer_auth(&state, request.headers())?;
            Ok(next.run(request).await)
        }
        AuthMode::ReverseProxy => {
            validate_reverse_proxy_auth(&state, request.headers())?;
            Ok(next.run(request).await)
        }
    }
}

fn validate_basic_auth(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<(), ApiError> {
    let challenge = Some(r#"Basic realm="lianli-backend""#.to_string());
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized {
            message: "missing Authorization header".to_string(),
            www_authenticate: challenge.clone(),
        })?;

    let encoded = header
        .strip_prefix("Basic ")
        .ok_or_else(|| ApiError::Unauthorized {
            message: "expected Basic Authorization header".to_string(),
            www_authenticate: challenge.clone(),
        })?;

    let decoded = STANDARD
        .decode(encoded.trim())
        .map_err(|_| ApiError::Unauthorized {
            message: "invalid Basic credentials encoding".to_string(),
            www_authenticate: challenge.clone(),
        })?;
    let decoded = String::from_utf8(decoded).map_err(|_| ApiError::Unauthorized {
        message: "invalid Basic credentials encoding".to_string(),
        www_authenticate: challenge.clone(),
    })?;
    let (username, password) = decoded
        .split_once(':')
        .ok_or_else(|| ApiError::Unauthorized {
            message: "invalid Basic credentials format".to_string(),
            www_authenticate: challenge.clone(),
        })?;

    if state.config.auth.basic_username.as_deref() == Some(username)
        && state.config.auth.basic_password.as_deref() == Some(password)
    {
        return Ok(());
    }

    Err(ApiError::Unauthorized {
        message: "invalid username or password".to_string(),
        www_authenticate: challenge,
    })
}

fn validate_bearer_auth(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<(), ApiError> {
    let challenge = Some(r#"Bearer realm="lianli-backend""#.to_string());
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized {
            message: "missing Authorization header".to_string(),
            www_authenticate: challenge.clone(),
        })?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized {
            message: "expected Bearer Authorization header".to_string(),
            www_authenticate: challenge.clone(),
        })?;

    if state.config.auth.bearer_token.as_deref() == Some(token.trim()) {
        return Ok(());
    }

    Err(ApiError::Unauthorized {
        message: "invalid bearer token".to_string(),
        www_authenticate: challenge,
    })
}

fn validate_reverse_proxy_auth(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<(), ApiError> {
    let header_name = state
        .config
        .auth
        .proxy_header
        .as_deref()
        .ok_or_else(|| ApiError::Internal("reverse proxy auth header missing in config".to_string()))?;
    let header_name = HeaderName::from_bytes(header_name.as_bytes())
        .map_err(|_| ApiError::Internal("reverse proxy auth header is invalid".to_string()))?;
    let value = headers
        .get(&header_name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if value.is_some() {
        return Ok(());
    }

    Err(ApiError::Unauthorized {
        message: format!("missing reverse proxy auth header: {}", header_name.as_str()),
        www_authenticate: None,
    })
}
