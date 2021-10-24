use crate::responses::ErrorResponse;
use rocket::http::Status;
use rocket::Request;

#[catch(default)]
pub fn default(status: Status, _: &Request) -> ErrorResponse {
    match status {
        s if s.code >= 400 && s.code < 500 => ErrorResponse::fail(s.to_string(), s),
        _ => ErrorResponse::error(status.to_string(), status),
    }
}
