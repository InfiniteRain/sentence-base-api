fn get_int_env_with_default(name: &str, default: u64) -> u64 {
    match std::env::var(name) {
        Ok(seconds) => seconds.parse::<u64>().unwrap_or(default),
        Err(_) => default,
    }
}

pub fn get_access_token_expiry_time() -> u64 {
    get_int_env_with_default("JWT_ACCESS_TOKEN_EXPIRY_TIME", 3600)
}

pub fn get_refresh_token_expiry_time() -> u64 {
    get_int_env_with_default("JWT_REFRESH_TOKEN_EXPIRY_TIME", 43800)
}

pub fn get_maximum_pending_sentences() -> u64 {
    get_int_env_with_default("MAXIMUM_PENDING_SENTENCES", 250)
}
