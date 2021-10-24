use crate::analyzer::{analyze_sentence, AnalysisResult};
use crate::responses::{ErrorResponse, ResponseResult, SuccessResponse};
use rocket::http::Status;

#[get("/analyze?<sentence>")]
pub fn analyze(sentence: &str) -> ResponseResult<AnalysisResult> {
    if sentence.is_empty() {
        return Err(ErrorResponse::fail(
            "The sentence should be a non-empty string.".to_string(),
            Status::UnprocessableEntity,
        ));
    }

    Ok(SuccessResponse::new(AnalysisResult {
        morphemes: analyze_sentence(sentence),
    }))
}
