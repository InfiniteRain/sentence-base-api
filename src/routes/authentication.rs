use crate::database;
use crate::field_validator::validate;
use crate::models::user::{User, UserRegistrationError};
use crate::responses::{ErrorResponse, ResponseResult, SuccessResponse};
use rocket::http::Status;
use rocket::serde::{json::Json, Deserialize};
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct NewUserData {
    #[validate(length(min = 3))]
    username: String,
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[post("/authenticate/register", format = "json", data = "<new_user>")]
pub fn register(
    new_user: Json<NewUserData>,
    connection: database::DbConnection,
) -> ResponseResult<User> {
    let new_user = validate(new_user)?;

    let username = new_user.username.trim();
    let email = new_user.email.trim();

    let registration_result = User::register(
        &connection,
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
