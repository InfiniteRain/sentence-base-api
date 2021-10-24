use crate::analyzer::{analyze_sentence, AnalysisResult};
use crate::responses::{
    Error, ErrorResponse, Fail, FailResponse, StandardResponse, Success, SuccessResponse,
};
use rocket::http::Status;
use rocket::Request;

#[get("/analyze?<sentence>")]
pub fn analyze(sentence: &str) -> StandardResponse<AnalysisResult> {
    if sentence.is_empty() {
        return Fail(FailResponse::new(
            vec!["The sentence should be a non-empty string.".to_string()],
            Status::UnprocessableEntity,
        ));
    }

    Success(SuccessResponse::new(AnalysisResult {
        morphemes: analyze_sentence(sentence),
    }))
}
