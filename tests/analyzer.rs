use common::*;
use rocket::http::Status;
use sentence_base::jwt::TokenType;
use serde_json::json;

mod common;

const SENTENCES: [&'static str; 1] = ["これはペンです。"];

#[test]
fn analyze_should_require_auth() {
    let (client, _) = create_client();

    let response = send_post_request_with_json(
        &client,
        "/analyze",
        json!({
            "sentence": SENTENCES[0]
        }),
    );
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn analyze_should_validate() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/analyze",
        &access_token,
        json!({
            "sentence": ""
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(&json, vec!["sentence".to_string()]);
}

#[test]
fn analyze_should_morphemalize() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/analyze",
        &access_token,
        json!({
            "sentence": SENTENCES[0]
        }),
    );
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let morphemes = json
        .get("data")
        .unwrap()
        .as_object()
        .unwrap()
        .get("morphemes")
        .expect("should include 'morphemes' field")
        .as_array()
        .expect("'morphemes' should be an array");

    let expected_result: [(&str, &str, &str); 5] = [
        ("これ", "これ", "コレ"),
        ("は", "は", "ハ"),
        ("ペン", "ペン", "ペン"),
        ("です", "です", "デス"),
        ("。", "。", "。"),
    ];

    let mut index = 0;
    for (expected_morpheme, expected_dictionary_form, expected_reading) in expected_result {
        let morpheme_element = morphemes
            .get(index)
            .expect(&format!("index {} should exist", index))
            .as_object()
            .expect("morpheme element should be an object");

        let morpheme = morpheme_element
            .get("morpheme")
            .expect("should include 'morpheme' field")
            .as_str()
            .expect("'morpheme' should be a string");

        let dictionary_form = morpheme_element
            .get("dictionary_form")
            .expect("should include 'dictionary_form' field")
            .as_str()
            .expect("'dictionary_form' should be a string");

        let reading = morpheme_element
            .get("reading")
            .expect("should include 'reading' field")
            .as_str()
            .expect("'reading' should be a string");

        assert_eq!(morpheme, expected_morpheme);
        assert_eq!(dictionary_form, expected_dictionary_form);
        assert_eq!(reading, expected_reading);

        index += 1;
    }
}
