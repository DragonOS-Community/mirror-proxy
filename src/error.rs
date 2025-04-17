use actix_web::HttpResponse;
use askama::Template;

use crate::render::ErrorTemplate;

#[derive(Debug, Clone)]
pub enum HttpError {
    Forbidden {
        message: String,
        description: String,
    },
    NotFound {
        message: String,
        description: String,
    },
    BadRequest {
        message: String,
        description: String,
    },
    InternalServerError {
        message: String,
        description: String,
    },
}

impl HttpError {
    pub fn forbidden(message: &str, description: &str) -> Self {
        Self::Forbidden {
            message: message.to_string(),
            description: description.to_string(),
        }
    }

    pub fn not_found(message: &str, description: &str) -> Self {
        Self::NotFound {
            message: message.to_string(),
            description: description.to_string(),
        }
    }

    pub fn bad_request(message: &str, description: &str) -> Self {
        Self::BadRequest {
            message: message.to_string(),
            description: description.to_string(),
        }
    }

    pub fn internal_error(message: &str, description: &str) -> Self {
        Self::InternalServerError {
            message: message.to_string(),
            description: description.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Forbidden { .. } => 403,
            Self::NotFound { .. } => 404,
            Self::BadRequest { .. } => 400,
            Self::InternalServerError { .. } => 500,
        }
    }

    pub fn to_http_response(&self) -> HttpResponse {
        let (status_code, message, error_code, description) = match self {
            Self::Forbidden {
                message,
                description,
            } => (
                403,
                message.clone(),
                "FORBIDDEN".to_string(),
                description.clone(),
            ),
            Self::NotFound {
                message,
                description,
            } => (
                404,
                message.clone(),
                "NOT_FOUND".to_string(),
                description.clone(),
            ),
            Self::BadRequest {
                message,
                description,
            } => (
                400,
                message.clone(),
                "BAD_REQUEST".to_string(),
                description.clone(),
            ),
            Self::InternalServerError {
                message,
                description,
            } => (
                500,
                message.clone(),
                "INTERNAL_ERROR".to_string(),
                description.clone(),
            ),
        };

        let template = ErrorTemplate {
            title: format!("{} - 错误", status_code),
            status_code: status_code.to_string(),
            message,
            error_code,
            description,
            home_url: "/".to_string(),
        };

        match template.render() {
            Ok(html) => match status_code {
                403 => HttpResponse::Forbidden()
                    .content_type("text/html")
                    .body(html),
                404 => HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(html),
                400 => HttpResponse::BadRequest()
                    .content_type("text/html")
                    .body(html),
                500 => HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body(html),
                _ => HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body(html),
            },
            Err(e) => HttpResponse::InternalServerError()
                .body(format!("Failed to render error page: {}", e)),
        }
    }
}
