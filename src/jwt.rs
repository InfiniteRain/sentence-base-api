use crate::models::user::User;
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

pub fn get_jwt_expiry_time() -> u64 {
    let default_expiry_time = 3600;
    match std::env::var("JWT_EXPIRY_TIME") {
        Ok(seconds) => seconds.parse::<u64>().unwrap_or(default_expiry_time),
        Err(_) => default_expiry_time,
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuthenticationClaims {
    pub iat: u64,
    pub exp: u64,
    pub sub: i32,
}

pub fn generate_authentication_token(user: &User) -> Option<String> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let claims = AuthenticationClaims {
        iat: current_timestamp,
        exp: current_timestamp + get_jwt_expiry_time(),
        sub: user.id,
    };

    claims.sign_with_key(&jwt_secret_hmac).ok()
}

#[derive(Debug, Copy, Clone)]
pub enum TokenValidationError {
    None,
    ServiceUnavailable,
    NoToken,
    MalformedToken,
    IatInTheFuture,
    Expired,
    InvalidSubject,
}

impl TokenValidationError {
    pub fn outcome(self, request: &Request) -> Outcome<User, TokenValidationError> {
        request.local_cache(|| self);
        Outcome::Failure((Status::Unauthorized, self))
    }
}

pub fn validate_authentication_token(
    token: String,
    database_connection: &PgConnection,
) -> Result<User, TokenValidationError> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let claims: AuthenticationClaims = token
        .verify_with_key(&jwt_secret_hmac)
        .map_err(|_| TokenValidationError::MalformedToken)?;
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if claims.iat > current_timestamp {
        return Err(TokenValidationError::IatInTheFuture);
    }

    if claims.exp <= current_timestamp {
        return Err(TokenValidationError::Expired);
    }

    User::find_by_id(database_connection, claims.sub)
        .ok_or_else(|| TokenValidationError::InvalidSubject)
}

pub fn extract_token_from_header(authorization_header: String) -> Option<String> {
    let header_regex = Regex::new(r"^Bearer\s+(.+)$").ok()?;
    Some(
        header_regex
            .captures(&authorization_header)?
            .get(1)?
            .as_str()
            .to_string(),
    )
}
