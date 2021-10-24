use std::io::Cursor;

use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::json;
use rocket::serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "status", rename = "success")]
pub struct SuccessResponse<T: Serialize> {
    data: T,
    #[serde(skip_serializing)]
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

impl<'r, T: Serialize> Responder<'r, 'static> for SuccessResponse<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let json = serde_json::to_string(&self).unwrap();

        Response::build()
            .sized_body(json.len(), Cursor::new(json))
            .header(ContentType::new("application", "json"))
            .status(self.http_status)
            .ok()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorType {
    Fail,
    Error,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    status: ErrorType,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasons: Option<Vec<String>>,
    #[serde(skip_serializing)]
    http_status: Status,
}

impl ErrorResponse {
    pub fn error(message: String, http_status: Status) -> ErrorResponse {
        ErrorResponse {
            status: ErrorType::Error,
            message,
            reasons: None,
            http_status,
        }
    }

    pub fn fail(message: String, http_status: Status) -> ErrorResponse {
        ErrorResponse {
            status: ErrorType::Fail,
            message,
            reasons: None,
            http_status,
        }
    }

    pub fn fail_with_reasons(
        message: String,
        reasons: Vec<String>,
        http_status: Status,
    ) -> ErrorResponse {
        ErrorResponse {
            status: ErrorType::Fail,
            message,
            reasons: Some(reasons),
            http_status,
        }
    }
}

impl<'r> Responder<'r, 'static> for ErrorResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let json = serde_json::to_string(&self).unwrap();

        Response::build()
            .sized_body(json.len(), Cursor::new(json))
            .header(ContentType::new("application", "json"))
            .status(self.http_status)
            .ok()
    }
}

pub type ResponseResult<T = ()> = Result<SuccessResponse<T>, ErrorResponse>;
