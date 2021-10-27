use crate::responses::ErrorResponse;
use rocket::http::Status;
use rocket::serde::json::Json;
use validator::Validate;

pub fn validate<T: Validate>(data: Json<T>) -> Result<T, ErrorResponse> {
    let data = data.into_inner();
    match data.validate() {
        Ok(_) => Ok(data),
        Err(err) => Err(ErrorResponse::fail_with_reasons(
            "Validation Error".to_string(),
            err.field_errors()
                .iter()
                .map(|(field_name, field_errs)| {
                    field_errs
                        .iter()
                        .map(|fe| {
                            format!(
                                "field \"{}\" does not satisfy the \"{}\" rule: {:?}",
                                field_name, fe.code, fe.params
                            )
                        })
                        .collect::<Vec<String>>()
                })
                .flatten()
                .collect(),
            Status::UnprocessableEntity,
        )),
    }
}
