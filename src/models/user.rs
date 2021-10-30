use bcrypt::{hash, verify, DEFAULT_COST};
use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};
use rocket::serde::Serialize;

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
