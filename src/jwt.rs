use crate::models::user::User;
use hmac::{Hmac, NewMac};
use jwt::SignWithKey;
use rocket::serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").expect("JWT_SECRET env variable should be set")
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
    let jwt_secret = get_jwt_secret();
    let hmac: Hmac<Sha256> = Hmac::new_from_slice(jwt_secret.as_bytes()).ok()?;
    let current_time = SystemTime::now();

    let claims = AuthenticationClaims {
        iat: current_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        exp: current_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .add(Duration::from_secs(get_jwt_expiry_time()))
            .as_secs(),
        sub: user.id,
    };

    claims.sign_with_key(&hmac).ok()
}
