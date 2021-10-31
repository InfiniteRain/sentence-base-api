use crate::jwt::TokenValidationError;
use crate::responses::ErrorResponse;
use rocket::http::Status;
use rocket::Request;

#[catch(default)]
pub fn default(status: Status, request: &Request) -> ErrorResponse {
    let token_validation_error = request.local_cache(|| TokenValidationError::None);

    match token_validation_error {
        TokenValidationError::NoToken => {
            ErrorResponse::fail("No Token Provided".to_string(), Status::Unauthorized)
        }
        TokenValidationError::MalformedToken => {
            ErrorResponse::fail("Malformed Token Provided".to_string(), Status::Unauthorized)
        }
        TokenValidationError::IatInTheFuture => ErrorResponse::fail(
            "Token with IAT in the Future Provided".to_string(),
            Status::Unauthorized,
        ),
        TokenValidationError::Expired => {
            ErrorResponse::fail("Expired Token Provided".to_string(), Status::Unauthorized)
        }
        TokenValidationError::InvalidSubject => ErrorResponse::fail(
            "Token with Invalid Subject Provided".to_string(),
            Status::Unauthorized,
        ),
        _ => match status {
            s if s.code >= 400 && s.code < 500 => ErrorResponse::fail(s.to_string(), s),
            _ => ErrorResponse::error(status.to_string(), status),
        },
    }
}
