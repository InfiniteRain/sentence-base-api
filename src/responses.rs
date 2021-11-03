use std::io::Cursor;

use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", rename = "success")]
pub struct SuccessResponse<T: Serialize> {
    data: T,
    #[serde(skip)]
    http_status: Status,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(data: T) -> SuccessResponse<T> {
        SuccessResponse {
            data,
            http_status: Status::Ok,
        }
    }

    pub fn get_data(&self) -> &T {
        &self.data
    }
}

impl<'r, T: Serialize> Responder<'r, 'static> for SuccessResponse<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        generate_response(&self, self.http_status)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ErrorType {
    Fail,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    status: ErrorType,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasons: Option<Vec<String>>,
    #[serde(skip)]
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
        generate_response(&self, self.http_status)
    }
}

fn generate_response<T: Serialize>(
    responder: &T,
    http_status: Status,
) -> response::Result<'static> {
    let json = serde_json::to_string(&responder).unwrap();

    Response::build()
        .sized_body(json.len(), Cursor::new(json))
        .header(ContentType::new("application", "json"))
        .status(http_status)
        .ok()
}

pub type ResponseResult<T = ()> = Result<SuccessResponse<T>, ErrorResponse>;
