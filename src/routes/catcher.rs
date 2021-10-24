use crate::responses::{Error, ErrorResponse, Fail, FailResponse, StandardResponse};
use rocket::http::Status;
use rocket::Request;

#[catch(default)]
pub fn default(status: Status, _: &Request) -> StandardResponse {
    match status {
        s if s.code >= 400 && s.code < 500 => Fail(FailResponse::new(vec![s.to_string()], s)),
        _ => Error(ErrorResponse::new(status.to_string(), status)),
    }
}
