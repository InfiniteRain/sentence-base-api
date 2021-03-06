use crate::database::DbConnection;
use crate::field_validator::validate;
use crate::frequency_list::JpFrequencyList;
use crate::models::mining_batch::MiningBatch;
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
pub struct NewSentenceRequest {
    #[validate(length(min = 1))]
    dictionary_form: String,
    #[validate(length(min = 1))]
    reading: String,
    #[validate(length(min = 1))]
    sentence: String,
}

#[derive(Deserialize, Serialize)]
pub struct NewSentenceResponse {
    pub sentence: UserSentenceEntry,
}

#[post("/sentences", format = "json", data = "<new_sentence_request>")]
pub fn new(
    new_sentence_request: Json<NewSentenceRequest>,
    database_connection: DbConnection,
    user: User,
    frequency_list: &State<JpFrequencyList>,
) -> ResponseResult<NewSentenceResponse> {
    let new_sentence_data = validate(new_sentence_request)?;

    let dictionary_form = new_sentence_data.dictionary_form.trim().to_string();
    let reading = new_sentence_data.reading.trim().to_string();
    let sentence = new_sentence_data.sentence.trim().to_string();

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

    Ok(SuccessResponse::new(NewSentenceResponse {
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

#[delete("/sentences/<sentence_id>")]
pub fn delete(sentence_id: i32, database_connection: DbConnection, user: User) -> ResponseResult {
    let pending_sentence = user
        .get_pending_sentence_by_id(&database_connection, sentence_id)
        .ok_or_else(|| {
            ErrorResponse::fail("Pending Sentence Not Found".to_string(), Status::NotFound)
        })?;

    pending_sentence
        .delete(&database_connection)
        .map_err(DB_ERROR_MAP_FN)?;

    Ok(SuccessResponse::new(()))
}

fn validate_sentences_length<T>(hash_set: &HashSet<T>) -> Result<(), ValidationError> {
    if hash_set.is_empty() {
        return Err(ValidationError::new("empty_set"));
    }

    Ok(())
}

#[derive(Validate, Deserialize)]
pub struct NewBatchRequest {
    #[validate(custom = "validate_sentences_length")]
    sentences: HashSet<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct NewBatchResponse {
    pub batch_id: i32,
}

#[post("/sentences/batches", format = "json", data = "<new_batch_request>")]
pub fn new_batch(
    new_batch_request: Json<NewBatchRequest>,
    database_connection: DbConnection,
    user: User,
) -> ResponseResult<NewBatchResponse> {
    let new_batch_data = validate(new_batch_request)?;

    let sentences: Vec<i32> = new_batch_data.sentences.into_iter().collect();

    let mining_batch = user
        .new_mining_batch(&database_connection, &sentences)
        .map_err(|err| match err {
            CommitSentencesError::DatabaseError(err) => DB_ERROR_MAP_FN(err),
            CommitSentencesError::InvalidSentencesProvided => ErrorResponse::fail(
                "Invalid Sentences Provided".to_string(),
                Status::UnprocessableEntity,
            ),
        })?;

    Ok(SuccessResponse::new(NewBatchResponse {
        batch_id: mining_batch.id,
    }))
}

#[derive(Serialize, Deserialize)]
pub struct GetBatchResponse {
    pub sentences: Vec<UserSentenceEntry>,
}

#[get("/sentences/batches/<mining_batch_id>")]
pub fn get_batch(
    mining_batch_id: i32,
    database_connection: DbConnection,
    user: User,
    frequency_list: &State<JpFrequencyList>,
) -> ResponseResult<GetBatchResponse> {
    let mining_batch = user
        .get_mining_batch_by_id(&database_connection, mining_batch_id)
        .ok_or_else(|| ErrorResponse::fail("Batch Not Found".to_string(), Status::NotFound))?;

    let sentences = mining_batch
        .get_sentences(&database_connection, frequency_list)
        .map_err(DB_ERROR_MAP_FN)?;

    Ok(SuccessResponse::new(GetBatchResponse { sentences }))
}

#[derive(Serialize)]
pub struct GetAllBatchesResponse {
    pub batches: Vec<MiningBatch>,
}

#[get("/sentences/batches")]
pub fn get_all_batches(
    database_connection: DbConnection,
    user: User,
) -> ResponseResult<GetAllBatchesResponse> {
    let batches = user
        .get_all_mining_batches(&database_connection)
        .map_err(DB_ERROR_MAP_FN)?;

    Ok(SuccessResponse::new(GetAllBatchesResponse { batches }))
}
