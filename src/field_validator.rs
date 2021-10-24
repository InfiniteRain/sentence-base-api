use crate::responses::ErrorResponse;
use rocket::http::Status;
use validator::{Validate, ValidationError, ValidationErrors, ValidationErrorsKind};

pub fn validate<T: Validate>(data: T) -> Result<(), ErrorResponse> {
    match data.validate() {
        Ok(_) => Ok(()),
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
            Status::NotFound,
        )),
    }
}
