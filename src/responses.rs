use std::io::Cursor;

use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::json;
use rocket::serde::Serialize;

pub struct SuccessResponse<T: Serialize> {
    data: T,
    http_status: Status,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(data: T) -> SuccessResponse<T> {
        SuccessResponse {
            data,
            http_status: Status::Ok,
        }
    }

    pub fn with_http_status(data: T, http_status: Status) -> SuccessResponse<T> {
        SuccessResponse { data, http_status }
    }
}

pub struct FailResponse {
    reasons: Vec<String>,
    http_status: Status,
}

impl FailResponse {
    pub fn new(reasons: Vec<String>, http_status: Status) -> FailResponse {
        FailResponse {
            reasons,
            http_status,
        }
    }
}

pub struct ErrorResponse {
    message: String,
    http_status: Status,
}

impl ErrorResponse {
    pub fn new(message: String, http_status: Status) -> ErrorResponse {
        ErrorResponse {
            message,
            http_status,
        }
    }
}

pub enum StandardResponse<T: Serialize = ()> {
    Success(SuccessResponse<T>),
    Fail(FailResponse),
    Error(ErrorResponse),
}

impl<'r, T: Serialize> Responder<'r, 'static> for StandardResponse<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let (json, http_status) = match self {
            StandardResponse::Success(r) => (
                json!({
                    "status": "success",
                    "data": r.data
                })
                .to_string(),
                r.http_status,
            ),
            StandardResponse::Fail(r) => (
                json!({
                    "status": "fail",
                    "reasons": r.reasons
                })
                .to_string(),
                r.http_status,
            ),
            StandardResponse::Error(r) => (
                json!({
                    "status": "error",
                    "message": r.message
                })
                .to_string(),
                r.http_status,
            ),
        };

        Response::build()
            .sized_body(json.len(), Cursor::new(json))
            .header(ContentType::new("application", "json"))
            .status(http_status)
            .ok()
    }
}

pub use self::StandardResponse::{Error, Fail, Success};
