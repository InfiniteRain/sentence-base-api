use crate::helpers::{get_access_token_expiry_time, get_refresh_token_expiry_time};
use crate::models::user::User;
use crate::responses::ErrorResponse;
use diesel::PgConnection;
use hmac::{Hmac, NewMac};
use jwt::SignWithKey;
use jwt::VerifyWithKey;
use regex::Regex;
use rocket::http::Status;
use rocket::request::Outcome;
use rocket::serde::{Deserialize, Serialize};
use rocket::Request;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_jwt_secret_hmac() -> Hmac<Sha256> {
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET env variable should be set");
    Hmac::new_from_slice(jwt_secret.as_bytes()).expect("hmac should be created")
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Serialize, Deserialize)]
pub struct TokenClaims {
    pub iat: u64,
    pub exp: u64,
    pub sub: i32,
    pub gen: i32,
    pub typ: TokenType,
}

pub fn generate_token(user: &User, token_type: TokenType) -> Option<String> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let current_timestamp = get_current_timestamp();
    let expiry_time = match token_type {
        TokenType::Access => get_access_token_expiry_time(),
        TokenType::Refresh => get_refresh_token_expiry_time(),
    };

    let claims = TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + expiry_time,
        sub: user.id,
        gen: user.token_generation,
        typ: token_type,
    };

    claims.sign_with_key(&jwt_secret_hmac).ok()
}

#[derive(Debug, Copy, Clone)]
pub enum TokenError {
    None,
    NoToken,
    MalformedToken,
    IatInTheFuture,
    Expired,
    Revoked,
    InvalidSubject,
    InvalidType,
}

impl TokenError {
    pub fn outcome(self, request: &Request) -> Outcome<User, TokenError> {
        request.local_cache(|| self);
        Outcome::Failure((Status::Unauthorized, self))
    }
}

pub fn validate_token(
    token: String,
    token_type: TokenType,
    database_connection: &PgConnection,
) -> Result<User, TokenError> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let claims: TokenClaims = token
        .verify_with_key(&jwt_secret_hmac)
        .map_err(|_| TokenError::MalformedToken)?;
    let current_timestamp = get_current_timestamp();

    if claims.typ != token_type {
        return Err(TokenError::InvalidType);
    }

    if claims.iat > current_timestamp {
        return Err(TokenError::IatInTheFuture);
    }

    if claims.exp <= current_timestamp {
        return Err(TokenError::Expired);
    }

    let user =
        User::find_by_id(database_connection, claims.sub).ok_or(TokenError::InvalidSubject)?;

    if claims.gen != user.token_generation {
        return Err(TokenError::Revoked);
    }

    Ok(user)
}

pub fn token_error_to_response(token_error: &TokenError) -> ErrorResponse {
    let message = match token_error {
        TokenError::NoToken => "No Token Provided",
        TokenError::MalformedToken => "Malformed Token Provided",
        TokenError::IatInTheFuture => "Token with IAT in the Future Provided",
        TokenError::Expired => "Expired Token Provided",
        TokenError::Revoked => "Revoked Token Provided",
        TokenError::InvalidSubject => "Token with Invalid Subject Provided",
        TokenError::InvalidType => "Token with Invalid Type Provided",
        _ => {
            return ErrorResponse::error(
                "Unexpected Token Error".to_string(),
                Status::InternalServerError,
            )
        }
    };

    ErrorResponse::fail(message.to_string(), Status::Unauthorized)
}

pub fn extract_access_token_from_header(authorization_header: String) -> Option<String> {
    let header_regex = Regex::new(r"^Bearer\s+(.+)$").ok()?;
    Some(
        header_regex
            .captures(&authorization_header)?
            .get(1)?
            .as_str()
            .to_string(),
    )
}

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
