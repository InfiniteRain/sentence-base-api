use crate::database::DbConnection;
use crate::field_validator::validate;
use crate::frequency_list::JpFrequencyList;
use crate::models::sentence::Sentence;
use crate::models::user::{User, UserSentenceEntry};
use crate::models::word::Word;
use crate::responses::{ErrorResponse, ResponseResult, SuccessResponse};
use diesel::result::Error;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct AddSentenceRequest {
    #[validate(length(min = 1))]
    dictionary_form: String,
    #[validate(length(min = 1))]
    reading: String,
    #[validate(length(min = 1))]
    sentence: String,
}

#[post("/sentences", format = "json", data = "<sentence_request>")]
pub fn add(
    sentence_request: Json<AddSentenceRequest>,
    database_connection: DbConnection,
    user: User,
) -> ResponseResult {
    let sentence_data = validate(sentence_request)?;

    let dictionary_form = sentence_data.dictionary_form.trim().to_string();
    let reading = sentence_data.reading.trim().to_string();
    let sentence = sentence_data.sentence.trim().to_string();

    let error_map_fn = |_error: Error| {
        ErrorResponse::error("Unexpected Error".to_string(), Status::InternalServerError)
    };
    let is_limit_reached = user
        .pending_sentence_limit_reached(&database_connection)
        .map_err(error_map_fn)?;

    if is_limit_reached {
        return Err(ErrorResponse::fail(
            "Pending Sentences Limit Reached".to_string(),
            Status::TooManyRequests,
        ));
    }

    let word_entry =
        Word::add_or_increase_frequency(&database_connection, &user, &dictionary_form, &reading)
            .map_err(error_map_fn)?;
    Sentence::add(&database_connection, &user, &word_entry, &sentence).map_err(error_map_fn)?;

    Ok(SuccessResponse::new(()))
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
        .map_err(|_| {
            ErrorResponse::error("Unexpected Error".to_string(), Status::InternalServerError)
        })?;

    Ok(SuccessResponse::new(GetSentenceResponse {
        sentences: pending_sentences,
    }))
}
