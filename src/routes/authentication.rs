use crate::database;
use crate::field_validator::validate;
use crate::jwt::{generate_token, token_error_to_response, validate_token, TokenType};
use crate::models::user::{User, UserRegistrationError};
use crate::responses::{ErrorResponse, ResponseResult, SuccessResponse};
use rocket::http::Status;
use rocket::serde::{json::Json, Deserialize, Serialize};
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct RegisterRequest {
    #[validate(length(min = 3))]
    username: String,
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[post("/auth/register", format = "json", data = "<new_user>")]
pub fn register(
    register_request: Json<RegisterRequest>,
    database_connection: database::DbConnection,
) -> ResponseResult<User> {
    let register_data = validate(register_request)?;

    let username = register_data.username.trim();
    let email = register_data.email.trim();

    let registration_result = User::register(
        &database_connection,
        username.to_string(),
        email.to_string(),
        register_data.password.to_string(),
    );

    match registration_result {
        Ok(user) => Ok(SuccessResponse::new(user)),
        Err(error) => Err(ErrorResponse::fail_with_reasons(
            "Validation Error".to_string(),
            vec![match error {
                UserRegistrationError::DuplicateEmail => "duplicate email".to_string(),
                UserRegistrationError::DuplicateUsername => "duplicate username".to_string(),
                UserRegistrationError::FailedToHash => "password hash failed".to_string(),
            }],
            Status::Conflict,
        )),
    }
}

#[derive(Validate, Deserialize)]
pub struct LoginRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 1))]
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    access_token: String,
    refresh_token: String,
}

#[post("/auth/login", format = "json", data = "<login_request>")]
pub fn login(
    login_request: Json<LoginRequest>,
    database_connection: database::DbConnection,
) -> ResponseResult<LoginResponse> {
    let login_data = validate(login_request)?;

    let email = login_data.email.trim().to_string();
    let password = login_data.password;

    let user =
        User::find_by_credentials(&database_connection, email, password).ok_or_else(|| {
            ErrorResponse::fail("Invalid Credentials".to_string(), Status::Unauthorized)
        })?;

    let error_map_fn = || {
        ErrorResponse::error(
            "Failed to sign JWT".to_string(),
            Status::InternalServerError,
        )
    };
    let access_token = generate_token(&user, TokenType::Access).ok_or_else(error_map_fn)?;
    let refresh_token = generate_token(&user, TokenType::Refresh).ok_or_else(error_map_fn)?;

    Ok(SuccessResponse::new(LoginResponse {
        access_token,
        refresh_token,
    }))
}

#[derive(Validate, Deserialize)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    access_token: String,
    refresh_token: String,
}

#[post("/auth/refresh", format = "json", data = "<refresh_request>")]
pub fn refresh(
    refresh_request: Json<RefreshRequest>,
    database_connection: database::DbConnection,
) -> ResponseResult<RefreshResponse> {
    let refresh_data = validate(refresh_request)?;
    let user = validate_token(
        refresh_data.refresh_token,
        TokenType::Refresh,
        &database_connection,
    )
    .map_err(|error| token_error_to_response(&error))?;

    let error_map_fn = || {
        ErrorResponse::error(
            "Failed to sign JWT".to_string(),
            Status::InternalServerError,
        )
    };
    let access_token = generate_token(&user, TokenType::Access).ok_or_else(error_map_fn)?;
    let refresh_token = generate_token(&user, TokenType::Refresh).ok_or_else(error_map_fn)?;

    Ok(SuccessResponse::new(RefreshResponse {
        access_token,
        refresh_token,
    }))
}

#[get("/auth/me")]
pub fn me(user: User) -> ResponseResult<User> {
    Ok(SuccessResponse::new(user))
}
