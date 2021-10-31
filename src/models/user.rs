use crate::database::Pool;
use crate::jwt::{extract_access_token_from_header, validate_token, TokenError, TokenType};
use crate::schema::users;
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::Serialize;
use rocket::State;

#[derive(Queryable, Serialize, Identifiable, AsChangeset)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub hash: String,
    #[serde(skip_serializing)]
    pub token_generation: i32,
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
    type Error = TokenError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let authorization_header = request.headers().get("Authorization").collect::<String>();

        let token = match extract_access_token_from_header(authorization_header) {
            Some(token) => token,
            None => return TokenError::NoToken.outcome(request),
        };

        let pool =
            try_outcome!(request
                .guard::<&State<Pool>>()
                .await
                .map_failure(|(status, _)| {
                    request.local_cache(|| TokenError::None);
                    (status, TokenError::None)
                }));

        match pool.get() {
            Ok(connection) => match validate_token(token, TokenType::Access, &connection) {
                Ok(user) => Outcome::Success(user),
                Err(error) => error.outcome(request),
            },
            Err(_) => TokenError::None.outcome(request),
        }
    }
}

impl User {
    pub fn find_by_id(database_connection: &PgConnection, user_id: i32) -> Option<User> {
        users::table
            .find(user_id)
            .get_result(database_connection)
            .ok()
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

    pub fn increment_token_generation(
        mut self,
        database_connection: &PgConnection,
    ) -> Result<i32, Error> {
        self.token_generation += 1;
        self.save_changes::<User>(database_connection)?;

        Ok(self.token_generation)
    }
}
