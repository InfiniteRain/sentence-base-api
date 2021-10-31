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

pub fn get_access_token_expiry_time() -> u64 {
    let default_expiry_time = 3600;
    match std::env::var("JWT_EXPIRY_TIME") {
        Ok(seconds) => seconds.parse::<u64>().unwrap_or(default_expiry_time),
        Err(_) => default_expiry_time,
    }
}

#[derive(Serialize, Deserialize)]
pub struct AccessClaims {
    pub iat: u64,
    pub exp: u64,
    pub sub: i32,
    pub gen: i32,
    pub typ: u8,
}

pub fn generate_access_token(user: &User) -> Option<String> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let claims = AccessClaims {
        iat: current_timestamp,
        exp: current_timestamp + get_access_token_expiry_time(),
        sub: user.id,
        gen: user.token_generation,
        typ: 0,
    };

    claims.sign_with_key(&jwt_secret_hmac).ok()
}

#[derive(Debug, Copy, Clone)]
pub enum AccessTokenError {
    None,
    ServiceUnavailable,
    NoToken,
    MalformedToken,
    IatInTheFuture,
    Expired,
    Revoked,
    InvalidSubject,
    InvalidType,
}

impl AccessTokenError {
    pub fn outcome(self, request: &Request) -> Outcome<User, AccessTokenError> {
        request.local_cache(|| self);
        Outcome::Failure((Status::Unauthorized, self))
    }
}

pub fn validate_access_token(
    token: String,
    database_connection: &PgConnection,
) -> Result<User, AccessTokenError> {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let claims: AccessClaims = token
        .verify_with_key(&jwt_secret_hmac)
        .map_err(|_| AccessTokenError::MalformedToken)?;
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if claims.typ != 0 {
        return Err(AccessTokenError::InvalidType);
    }

    if claims.iat > current_timestamp {
        return Err(AccessTokenError::IatInTheFuture);
    }

    if claims.exp <= current_timestamp {
        return Err(AccessTokenError::Expired);
    }

    let user = User::find_by_id(database_connection, claims.sub)
        .ok_or(AccessTokenError::InvalidSubject)?;

    if claims.gen != user.token_generation {
        return Err(AccessTokenError::Revoked);
    }

    Ok(user)
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
