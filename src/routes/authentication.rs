use crate::database;
use crate::field_validator::validate;
use crate::jwt::generate_access_token;
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
    new_user: Json<RegisterRequest>,
    database_connection: database::DbConnection,
) -> ResponseResult<User> {
    let new_user = validate(new_user)?;

    let username = new_user.username.trim();
    let email = new_user.email.trim();

    let registration_result = User::register(
        &database_connection,
        username.to_string(),
        email.to_string(),
        new_user.password.to_string(),
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
pub struct AuthenticationRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 1))]
    password: String,
}

#[derive(Serialize)]
pub struct AuthenticationResponse {
    token: String,
}

#[post("/auth/login", format = "json", data = "<authentication_request>")]
pub fn login(
    authentication_request: Json<AuthenticationRequest>,
    database_connection: database::DbConnection,
) -> ResponseResult<AuthenticationResponse> {
    let authentication_data = validate(authentication_request)?;

    let email = authentication_data.email.trim().to_string();
    let password = authentication_data.password;

    let user =
        User::find_by_credentials(&database_connection, email, password).ok_or_else(|| {
            ErrorResponse::fail("Invalid Credentials".to_string(), Status::Unauthorized)
        })?;

    let token = generate_access_token(&user).ok_or_else(|| {
        ErrorResponse::error(
            "Failed to sign JWT".to_string(),
            Status::InternalServerError,
        )
    })?;

    Ok(SuccessResponse::new(AuthenticationResponse { token }))
}

#[get("/auth/me")]
pub fn me(user: User) -> ResponseResult<User> {
    Ok(SuccessResponse::new(user))
}
