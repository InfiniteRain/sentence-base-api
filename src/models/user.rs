use crate::database::Pool;
use crate::frequency_list::JpFrequencyList;
use crate::helpers::get_maximum_pending_sentences;
use crate::jwt::{extract_access_token_from_header, validate_token, TokenError, TokenType};
use crate::models::sentence::Sentence;
use crate::models::word::Word;
use crate::schema::sentences::columns::is_pending;
use crate::schema::users;
use crate::schema::words::dsl::words;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::NaiveDateTime;
use diesel;
use diesel::expression::count::count_star;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};
use itertools::Itertools;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::Serialize;
use rocket::State;
use std::collections::HashMap;

#[derive(Queryable, Serialize, Identifiable, AsChangeset, PartialEq)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub hash: String,
    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub token_generation: i32,
}

/*        id -> Int4,
username -> Text,
email -> Text,
hash -> Text,
created_at -> Timestamptz,
updated_at -> Timestamptz,
token_generation -> Int4,*/

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

#[derive(Serialize)]
pub struct UserSentenceEntry {
    sentence_id: i32,
    sentence: String,
    dictionary_form: String,
    reading: String,
    mining_frequency: i32,
    dictionary_frequency: usize,
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
        &mut self,
        database_connection: &PgConnection,
    ) -> Result<i32, Error> {
        self.token_generation += 1;
        self.save_changes::<User>(database_connection)?;

        Ok(self.token_generation)
    }

    // todo add "is_"
    pub fn pending_sentence_limit_reached(
        &self,
        database_connection: &PgConnection,
    ) -> Result<bool, Error> {
        let pending_sentences: i64 = Sentence::belonging_to(self)
            .filter(is_pending.eq(true))
            .select(count_star())
            .first(database_connection)?;

        Ok(pending_sentences >= get_maximum_pending_sentences() as i64)
    }

    pub fn get_pending_sentences(
        &self,
        database_connection: &PgConnection,
        frequency_list: &JpFrequencyList,
    ) -> Result<Vec<UserSentenceEntry>, Error> {
        let rows: Vec<(Sentence, Word)> = Sentence::belonging_to(self)
            .filter(is_pending.eq(true))
            .inner_join(words)
            .load(database_connection)?;

        let mut frequency_groups: HashMap<i32, Vec<UserSentenceEntry>> = HashMap::new();

        for (sentence, word) in rows {
            frequency_groups
                .entry(word.frequency)
                .or_default()
                .push(UserSentenceEntry {
                    sentence_id: sentence.id,
                    sentence: sentence.sentence,
                    dictionary_form: word.dictionary_form.clone(),
                    reading: word.reading.clone(),
                    mining_frequency: word.frequency,
                    dictionary_frequency: frequency_list
                        .get_frequency(word.dictionary_form, word.reading),
                })
        }

        Ok(frequency_groups
            .into_values()
            .map(|user_sentence_entries| {
                user_sentence_entries
                    .into_iter()
                    .sorted_by(|lhs, rhs| lhs.dictionary_frequency.cmp(&rhs.dictionary_frequency))
                    .collect::<Vec<UserSentenceEntry>>()
            })
            .sorted_by(|lhs, rhs| {
                lhs[0]
                    .mining_frequency
                    .cmp(&rhs[0].mining_frequency)
                    .reverse()
            })
            .flatten()
            .collect::<Vec<UserSentenceEntry>>())
    }
}
