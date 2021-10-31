use crate::database::Pool;
use crate::jwt::{extract_token_from_header, validate_authentication_token, TokenValidationError};
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::Serialize;
use rocket::State;

use crate::schema::users;

#[derive(Queryable, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub hash: String,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub hash: String,
}

#[derive(Debug)]
pub enum UserRegistrationError {
    DuplicateEmail,
    DuplicateUsername,
    FailedToHash,
}

impl From<Error> for UserRegistrationError {
    fn from(err: Error) -> UserRegistrationError {
        if let Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = &err {
            match info.constraint_name() {
                Some("users_username_key") => return UserRegistrationError::DuplicateUsername,
                Some("users_email_key") => return UserRegistrationError::DuplicateEmail,
                _ => {}
            }
        }

        panic!("Error registering a user: {:?}", err)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = TokenValidationError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let authorization_header = request.headers().get("Authorization").collect::<String>();

        let token = match extract_token_from_header(authorization_header) {
            Some(token) => token,
            None => return TokenValidationError::NoToken.outcome(&request),
        };

        let pool =
            try_outcome!(request
                .guard::<&State<Pool>>()
                .await
                .map_failure(|(status, _)| {
                    request.local_cache(|| TokenValidationError::ServiceUnavailable);
                    (status, TokenValidationError::ServiceUnavailable)
                }));

        match pool.get() {
            Ok(connection) => match validate_authentication_token(token, &connection) {
                Ok(user) => Outcome::Success(user),
                Err(error) => error.outcome(&request),
            },
            Err(_) => TokenValidationError::ServiceUnavailable.outcome(&request),
        }
    }
}

impl User {
    pub fn find_by_id(database_connection: &PgConnection, id: i32) -> Option<User> {
        users::table.find(id).get_result(database_connection).ok()
    }

    pub fn find_by_credentials(
        database_connection: &PgConnection,
        email: String,
        password: String,
    ) -> Option<User> {
        let user = users::table
            .filter(users::email.eq(email))
            .get_result::<User>(database_connection)
            .ok()?;

        if verify(password, &user.hash).ok()? {
            Some(user)
        } else {
            None
        }
    }

    pub fn register(
        database_connection: &PgConnection,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, UserRegistrationError> {
        let hashing_cost = match std::env::var("HASHING_COST") {
            Ok(cost) => cost.parse::<u32>().unwrap_or(DEFAULT_COST),
            Err(_) => DEFAULT_COST,
        };

        let new_user = NewUser {
            username,
            email,
            hash: hash(password, hashing_cost).map_err(|_| UserRegistrationError::FailedToHash)?,
        };

        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(database_connection)
            .map_err(Into::into)
    }
}
