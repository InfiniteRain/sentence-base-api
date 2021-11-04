use crate::database::Pool;
use crate::frequency_list::JpFrequencyList;
use crate::helpers::get_maximum_pending_sentences;
use crate::jwt::{extract_access_token_from_header, validate_token, TokenError, TokenType};
use crate::models::mining_batch::MiningBatch;
use crate::models::sentence::Sentence;
use crate::models::word::Word;
use crate::schema::sentences::dsl::sentences as dsl_sentences;
use crate::schema::sentences::{
    id as schema_sentences_id, is_pending as schema_sentences_is_pending,
    mining_batch_id as schema_sentences_mining_batch_id,
};
use crate::schema::users;
use crate::schema::words::dsl::words as dsl_words;
use crate::schema::words::{id as schema_words_id, is_mined as schema_words_is_mined};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::NaiveDateTime;
use diesel;
use diesel::dsl::any;
use diesel::expression::count::count_star;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error};
use itertools::Itertools;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Deserialize, Serialize};
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

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub hash: String,
}

#[derive(Deserialize, Serialize)]
pub struct UserSentenceEntry {
    pub sentence_id: i32,
    pub sentence: String,
    pub dictionary_form: String,
    pub reading: String,
    pub mining_frequency: i32,
    pub dictionary_frequency: usize,
}

impl UserSentenceEntry {
    pub fn new(word: &Word, sentence: &Sentence, frequency_list: &JpFrequencyList) -> Self {
        UserSentenceEntry {
            sentence_id: sentence.id,
            sentence: sentence.sentence.clone(),
            dictionary_form: word.dictionary_form.clone(),
            reading: word.reading.clone(),
            mining_frequency: word.frequency,
            dictionary_frequency: frequency_list
                .get_frequency(&word.dictionary_form, &word.reading),
        }
    }
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

pub enum CommitSentencesError {
    DatabaseError(Error),
    InvalidSentencesProvided,
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

    pub fn is_pending_sentence_limit_reached(
        &self,
        database_connection: &PgConnection,
    ) -> Result<bool, Error> {
        let pending_sentences: i64 = Sentence::belonging_to(self)
            .filter(schema_sentences_is_pending.eq(true))
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
            .filter(schema_sentences_is_pending.eq(true))
            .inner_join(dsl_words)
            .load(database_connection)?;

        let mut frequency_groups: HashMap<i32, Vec<UserSentenceEntry>> = HashMap::new();

        for (sentence, word) in rows {
            frequency_groups
                .entry(word.frequency)
                .or_default()
                .push(UserSentenceEntry::new(&word, &sentence, frequency_list));
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

    pub fn commit_batch(
        &self,
        database_connection: &PgConnection,
        sentence_ids: &[i32],
    ) -> Result<MiningBatch, CommitSentencesError> {
        let rows: Vec<(Sentence, Word)> = Sentence::belonging_to(self)
            .filter(schema_sentences_is_pending.eq(true))
            .filter(schema_sentences_id.eq(any(sentence_ids)))
            .inner_join(dsl_words)
            .load(database_connection)
            .map_err(CommitSentencesError::DatabaseError)?;

        if rows.len() != sentence_ids.len() {
            return Err(CommitSentencesError::InvalidSentencesProvided);
        }

        let mining_batch = MiningBatch::new(database_connection, self)
            .map_err(CommitSentencesError::DatabaseError)?;

        diesel::update(dsl_sentences.filter(schema_sentences_id.eq(any(sentence_ids))))
            .set((
                schema_sentences_is_pending.eq(false),
                schema_sentences_mining_batch_id.eq(mining_batch.id),
            ))
            .execute(database_connection)
            .map_err(CommitSentencesError::DatabaseError)?;

        let batch_words = dsl_words.filter(
            schema_words_id.eq(any(rows
                .into_iter()
                .map(|(_, word)| word.id)
                .collect::<Vec<i32>>())),
        );

        diesel::update(batch_words)
            .set(schema_words_is_mined.eq(true))
            .execute(database_connection)
            .map_err(CommitSentencesError::DatabaseError)?;

        Ok(mining_batch)
    }

    pub fn get_sentence_batch(
        &self,
        database_connection: &PgConnection,
        batch: &MiningBatch,
        frequency_list: &JpFrequencyList,
    ) -> Result<Vec<UserSentenceEntry>, Error> {
        let rows: Vec<(Sentence, Word)> = Sentence::belonging_to(batch)
            .inner_join(dsl_words)
            .load(database_connection)?;

        let sentences = rows
            .into_iter()
            .map(|(sentence, word)| UserSentenceEntry::new(&word, &sentence, frequency_list))
            .collect();

        Ok(sentences)
    }
}
