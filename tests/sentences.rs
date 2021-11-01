use common::*;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::{BelongingToDsl, ExpressionMethods};
use rocket::http::Status;
use sentence_base::helpers::get_maximum_pending_sentences;
use sentence_base::jwt::TokenType;
use sentence_base::models::sentence::Sentence;
use sentence_base::models::word::Word;
use sentence_base::schema::sentences::columns::is_pending;
use sentence_base::schema::sentences::dsl::sentences;
use serde_json::json;

mod common;

#[test]
fn add_should_require_auth() {
    let (client, _) = create_client();

    let response = send_post_request_with_json(
        &client,
        "/sentences",
        json!({
            "word": "some word",
            "sentence": "some sentence",
        }),
    );
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn add_should_validate() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "word": "",
            "sentence": "",
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(&json, vec!["word".to_string(), "sentence".to_string()]);
}

#[test]
fn add_should_result_with_a_word_and_a_sentence_added() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let test_word = "猫";
    let test_sentence = "これは猫です。";

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "word": test_word,
            "sentence": test_sentence,
        }),
    );
    assert_eq!(response.status(), Status::Ok);

    let word: Word = Word::belonging_to(&user)
        .first(&database_connection)
        .expect("should have at least one word");

    assert_eq!(word.id, 1);
    assert_eq!(word.user_id, user.id);
    assert_eq!(word.word, test_word);
    assert_eq!(word.frequency, 1);
    assert_eq!(word.is_mined, false);

    let sentence: Sentence = Sentence::belonging_to(&word)
        .first(&database_connection)
        .expect("should have at least one sentence");

    assert_eq!(sentence.id, 1);
    assert_eq!(sentence.user_id, word.user_id);
    assert_eq!(sentence.word_id, word.id);
    assert_eq!(sentence.sentence, test_sentence);
    assert_eq!(sentence.is_pending, true);

    let json = response_to_json(response);
    assert_success(&json);
}

#[test]
fn add_should_increase_frequency_on_duplicate() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let test_word = "cat";
    let test_sentences: [&str; 3] = ["this is a cat.", "the cat is cute.", "the cat is sleeping."];

    let mut index = 1;
    for test_sentence in test_sentences {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "word": test_word,
                "sentence": test_sentence,
            }),
        );
        assert_eq!(response.status(), Status::Ok);

        let sentence: Sentence = Sentence::belonging_to(&user)
            .find(index)
            .get_result(&database_connection)
            .expect(&format!("sentence with id {} should exist", index));

        assert_eq!(sentence.id, index);
        assert_eq!(sentence.user_id, user.id);
        assert_eq!(sentence.word_id, 1);
        assert_eq!(sentence.sentence, test_sentence);
        assert_eq!(sentence.is_pending, true);

        index += 1;
    }

    let word: Word = Word::belonging_to(&user)
        .first(&database_connection)
        .expect("should have at least one word");

    assert_eq!(word.id, 1);
    assert_eq!(word.user_id, user.id);
    assert_eq!(word.word, test_word);
    assert_eq!(word.frequency, 3);
    assert_eq!(word.is_mined, false);
}

#[test]
fn add_should_not_add_more_sentences_over_the_limit() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    std::env::set_var("MAXIMUM_PENDING_SENTENCES", "10");

    for index in 0..get_maximum_pending_sentences() {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "word": "cat",
                "sentence": format!("a cat number {} has appeared", index),
            }),
        );
        assert_eq!(response.status(), Status::Ok);
    }

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "word": "cat",
            "sentence": "the final cat has appeared",
        }),
    );
    assert_eq!(response.status(), Status::TooManyRequests);
    let json = response_to_json(response);
    assert_fail(&json, "Pending Sentences Limit Reached")
}

#[test]
fn add_should_not_count_non_pending_sentences_towards_the_limit() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    std::env::set_var("MAXIMUM_PENDING_SENTENCES", "10");

    for index in 0..get_maximum_pending_sentences() {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "word": "cat",
                "sentence": format!("a cat number {} has appeared", index),
            }),
        );
        assert_eq!(response.status(), Status::Ok);
    }

    let is_sentences_pending_limit_reached = user
        .pending_sentence_limit_reached(&database_connection)
        .expect("should resolve whether pending sentence limit was reached");

    assert!(is_sentences_pending_limit_reached);

    diesel::update(sentences.filter(is_pending.eq(true)))
        .set(is_pending.eq(false))
        .execute(&database_connection)
        .expect("query should execute");

    let is_sentences_pending_limit_reached_after_update = user
        .pending_sentence_limit_reached(&database_connection)
        .expect("should resolve whether pending sentence limit was reached");

    assert!(!is_sentences_pending_limit_reached_after_update)
}

// todo: mined -> unmined if the same word got mined
