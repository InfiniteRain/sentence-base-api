use crate::analyzer::{analyze_sentence, Morpheme};
use crate::field_validator::validate;
use crate::models::user::User;
use crate::responses::{ResponseResult, SuccessResponse};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct AnalyzeRequest {
    #[validate(length(min = 1))]
    sentence: String,
}

#[derive(Serialize)]
pub struct AnalyzeResponse {
    pub morphemes: Vec<Morpheme>,
}

#[post("/analyze", format = "json", data = "<analyze_request>")]
pub fn analyze(
    analyze_request: Json<AnalyzeRequest>,
    _user: User,
) -> ResponseResult<AnalyzeResponse> {
    let analyze_data = validate(analyze_request)?;

    Ok(SuccessResponse::new(AnalyzeResponse {
        morphemes: analyze_sentence(&analyze_data.sentence),
    }))
}
