use crate::jwt::{token_error_to_response, TokenError};
use crate::responses::ErrorResponse;
use rocket::http::Status;
use rocket::Request;

#[catch(default)]
pub fn default(status: Status, request: &Request) -> ErrorResponse {
    let token_validation_error = request.local_cache(|| TokenError::None);

    match token_validation_error {
        TokenError::NoToken
        | TokenError::MalformedToken
        | TokenError::Revoked
        | TokenError::IatInTheFuture
        | TokenError::Expired
        | TokenError::InvalidSubject
        | TokenError::InvalidType => token_error_to_response(token_validation_error),
        _ => match status {
            s if s.code >= 400 && s.code < 500 => ErrorResponse::fail(s.to_string(), s),
            _ => ErrorResponse::error(status.to_string(), status),
        },
    }
}
