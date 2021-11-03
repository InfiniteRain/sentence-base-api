use crate::database::DbConnection;
use crate::field_validator::validate;
use crate::frequency_list::JpFrequencyList;
use crate::models::sentence::Sentence;
use crate::models::user::{CommitSentencesError, User, UserSentenceEntry};
use crate::models::word::Word;
use crate::responses::{ErrorResponse, ResponseResult, SuccessResponse};
use diesel::result::Error;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use std::collections::HashSet;
use validator::{Validate, ValidationError};

const DB_ERROR_MAP_FN: fn(Error) -> ErrorResponse =
    |_| ErrorResponse::error("Unexpected Error".to_string(), Status::InternalServerError);

#[derive(Validate, Deserialize)]
pub struct AddSentenceRequest {
    #[validate(length(min = 1))]
    dictionary_form: String,
    #[validate(length(min = 1))]
    reading: String,
    #[validate(length(min = 1))]
    sentence: String,
}

#[derive(Deserialize, Serialize)]
pub struct AddSentenceResponse {
    pub sentence: UserSentenceEntry,
}

#[post("/sentences", format = "json", data = "<sentence_request>")]
pub fn add(
    sentence_request: Json<AddSentenceRequest>,
    database_connection: DbConnection,
    user: User,
    frequency_list: &State<JpFrequencyList>,
) -> ResponseResult<AddSentenceResponse> {
    let sentence_data = validate(sentence_request)?;

    let dictionary_form = sentence_data.dictionary_form.trim().to_string();
    let reading = sentence_data.reading.trim().to_string();
    let sentence = sentence_data.sentence.trim().to_string();

    let is_limit_reached = user
        .is_pending_sentence_limit_reached(&database_connection)
        .map_err(DB_ERROR_MAP_FN)?;

    if is_limit_reached {
        return Err(ErrorResponse::fail(
            "Pending Sentences Limit Reached".to_string(),
            Status::TooManyRequests,
        ));
    }

    let word_entry =
        Word::new_or_increase_frequency(&database_connection, &user, &dictionary_form, &reading)
            .map_err(DB_ERROR_MAP_FN)?;
    let sentence_entry = Sentence::new(&database_connection, &user, &word_entry, &sentence)
        .map_err(DB_ERROR_MAP_FN)?;

    Ok(SuccessResponse::new(AddSentenceResponse {
        sentence: UserSentenceEntry::new(&word_entry, &sentence_entry, frequency_list),
    }))
}

#[derive(Serialize)]
pub struct GetSentenceResponse {
    sentences: Vec<UserSentenceEntry>,
}

#[get("/sentences")]
pub fn get(
    database_connection: DbConnection,
    user: User,
    frequency_list: &State<JpFrequencyList>,
) -> ResponseResult<GetSentenceResponse> {
    let pending_sentences = user
        .get_pending_sentences(&database_connection, frequency_list)
        .map_err(DB_ERROR_MAP_FN)?;

    Ok(SuccessResponse::new(GetSentenceResponse {
        sentences: pending_sentences,
    }))
}

fn validate_sentences_length<T>(hash_set: &HashSet<T>) -> Result<(), ValidationError> {
    if hash_set.is_empty() {
        return Err(ValidationError::new("empty_set"));
    }

    Ok(())
}

#[derive(Validate, Deserialize)]
pub struct BatchRequest {
    #[validate(custom = "validate_sentences_length")]
    sentences: HashSet<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct BatchResponse {
    pub batch_id: i32,
}

#[post("/sentences/batches", format = "json", data = "<batch_request>")]
pub fn new_batch(
    batch_request: Json<BatchRequest>,
    database_connection: DbConnection,
    user: User,
) -> ResponseResult<BatchResponse> {
    let batch_data = validate(batch_request)?;

    let sentences: Vec<i32> = batch_data.sentences.into_iter().collect();

    let mining_batch = user
        .commit_batch(&database_connection, &sentences)
        .map_err(|err| match err {
            CommitSentencesError::DatabaseError(err) => DB_ERROR_MAP_FN(err),
            CommitSentencesError::InvalidSentencesProvided => ErrorResponse::fail(
                "Invalid Sentences Provided".to_string(),
                Status::UnprocessableEntity,
            ),
        })?;

    Ok(SuccessResponse::new(BatchResponse {
        batch_id: mining_batch.id,
    }))
}
