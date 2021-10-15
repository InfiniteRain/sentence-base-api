use crate::analyzer::{analyze_sentence, AnalysisResult};
use crate::responses::{
    Error, ErrorResponse, Fail, FailResponse, StandardResponse, Success, SuccessResponse,
};
use rocket::http::Status;
use rocket::Request;

#[catch(default)]
pub fn default(status: Status, _: &Request) -> StandardResponse {
    match status {
        s if s.code >= 400 && s.code < 500 => Fail(FailResponse::new(vec![s.to_string()], s)),
        _ => Error(ErrorResponse::new(status.to_string(), status)),
    }
}

#[get("/analyze?<sentence>")]
pub fn analyze(sentence: &str) -> StandardResponse<AnalysisResult> {
    if sentence.len() <= 0 {
        return Fail(FailResponse::new(
            vec!["The sentence should be a non-empty string.".to_string()],
            Status::UnprocessableEntity,
        ));
    }

    Success(SuccessResponse::new(AnalysisResult {
        morphemes: analyze_sentence(&sentence),
    }))
}
