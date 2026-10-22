use std::process::ExitCode;

use reqwest::StatusCode;
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub service: &'static str,
    pub operation: &'static str,
    pub status: Option<u16>,
    pub details: Option<Value>,
}

impl AppError {
    pub fn invalid_input(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::new("invalid_input", service, operation, message)
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new("config_error", "config", "load", message)
    }

    pub fn service_config(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::new("config_error", service, operation, message)
    }

    pub fn auth(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::new("auth_error", service, operation, message)
    }

    pub fn unsupported_auth(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            code: "unsupported_auth",
            message: message.into(),
            service,
            operation,
            status: None,
            details,
        }
    }

    pub fn not_found(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::new("not_found", service, operation, message)
    }

    pub fn api(
        service: &'static str,
        operation: &'static str,
        status: StatusCode,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        let code = if status == StatusCode::NOT_FOUND {
            "not_found"
        } else if status == StatusCode::TOO_MANY_REQUESTS {
            "rate_limited"
        } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            "auth_error"
        } else {
            "provider_api_error"
        };
        Self {
            code,
            message: message.into(),
            service,
            operation,
            status: Some(status.as_u16()),
            details,
        }
    }

    pub fn internal(
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::new("internal_error", service, operation, message)
    }

    pub fn gateway(
        service: &'static str,
        operation: &'static str,
        status: StatusCode,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            code: "gateway_error",
            message: message.into(),
            service,
            operation,
            status: Some(status.as_u16()),
            details,
        }
    }

    fn new(
        code: &'static str,
        service: &'static str,
        operation: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            service,
            operation,
            status: None,
            details: None,
        }
    }

    pub fn to_json_line(&self) -> String {
        let value = json!({
            "code": self.code,
            "message": self.message,
            "service": self.service,
            "operation": self.operation,
            "status": self.status,
            "details": self.details,
        });
        serde_json::to_string(&value).unwrap_or_else(|_| {
            "{\"code\":\"internal_error\",\"message\":\"failed to serialize error\"}".to_string()
        })
    }

    pub fn exit_code(&self) -> ExitCode {
        let code = match self.code {
            "invalid_input" => 2,
            "config_error" | "auth_error" | "unsupported_auth" => 3,
            "provider_api_error" => 4,
            "not_found" => 5,
            "rate_limited" => 6,
            _ => 1,
        };
        ExitCode::from(code)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::internal("io", "read", value.to_string())
    }
}
