use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use super::dto::{ApiErr, ApiErrorBody};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
            details: None,
        }
    }

    #[allow(dead_code)]
    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code,
            message: message.into(),
            details: None,
        }
    }

    #[allow(dead_code)]
    pub fn upstream(
        code: &'static str,
        message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code,
            message: message.into(),
            details,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
            details: None,
        }
    }

    fn to_body(&self) -> ApiErr {
        ApiErr {
            ok: false,
            error: ApiErrorBody {
                code: self.code.to_string(),
                message: self.message.clone(),
                details: self.details.clone(),
                trace_id: None,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.to_body())).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::internal(format!("{:#}", e))
    }
}
