use crate::responses::{ResponseResult, SuccessResponse};

#[get("/")]
pub fn get() -> ResponseResult {
    Ok(SuccessResponse::new(()))
}
