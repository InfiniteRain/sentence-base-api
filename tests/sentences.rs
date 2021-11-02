use common::*;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::{BelongingToDsl, ExpressionMethods};
use rocket::http::Status;
use rocket::serde::json::Value;
use sentence_base::helpers::get_maximum_pending_sentences;
use sentence_base::jwt::TokenType;
use sentence_base::models::sentence::Sentence;
use sentence_base::models::word::Word;
use sentence_base::schema::sentences::columns::is_pending;
use sentence_base::schema::sentences::dsl::sentences;
use serde_json::{json, Map};

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
            "dictionary_form": "",
            "reading": "",
            "sentence": "",
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(
        &json,
        vec![
            "dictionary_form".to_string(),
            "reading".to_string(),
            "sentence".to_string(),
        ],
    );
}

#[test]
fn add_should_result_with_a_word_and_a_sentence_added() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let test_dictionary_form = "猫";
    let test_reading = "ネコ";
    let test_sentence = "これは猫です。";

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "dictionary_form": test_dictionary_form,
            "reading": test_reading,
            "sentence": test_sentence,
        }),
    );
    assert_eq!(response.status(), Status::Ok);

    let word: Word = Word::belonging_to(&user)
        .first(&database_connection)
        .expect("should have at least one word");

    assert_eq!(word.id, 1);
    assert_eq!(word.user_id, user.id);
    assert_eq!(word.dictionary_form, test_dictionary_form);
    assert_eq!(word.reading, test_reading);
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

    let test_dictionary_form = "cat";
    let test_reading = "CAT";
    let test_sentences: [&str; 3] = ["this is a cat.", "the cat is cute.", "the cat is sleeping."];

    for (index, test_sentence) in test_sentences.iter().enumerate() {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "dictionary_form": test_dictionary_form,
                "reading": test_reading,
                "sentence": test_sentence,
            }),
        );
        assert_eq!(response.status(), Status::Ok);

        let sentence_id = (index + 1) as i32;

        let sentence: Sentence = Sentence::belonging_to(&user)
            .find(sentence_id as i32)
            .get_result(&database_connection)
            .expect(&format!("sentence with id {} should exist", sentence_id));

        assert_eq!(sentence.id, sentence_id);
        assert_eq!(sentence.user_id, user.id);
        assert_eq!(sentence.word_id, 1);
        assert_eq!(sentence.sentence, *test_sentence);
        assert_eq!(sentence.is_pending, true);
    }

    let word: Word = Word::belonging_to(&user)
        .first(&database_connection)
        .expect("should have at least one word");

    assert_eq!(word.id, 1);
    assert_eq!(word.user_id, user.id);
    assert_eq!(word.dictionary_form, test_dictionary_form);
    assert_eq!(word.reading, test_reading);
    assert_eq!(word.frequency, 3);
    assert_eq!(word.is_mined, false);
}

#[test]
fn add_should_not_add_more_sentences_over_the_limit() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    std::env::set_var("MAXIMUM_PENDING_SENTENCES", "10");

    for index in 0..get_maximum_pending_sentences() {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "dictionary_form": "cat",
                "reading": "CAT",
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
            "dictionary_form": "cat",
            "reading": "CAT",
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
                "dictionary_form": "cat",
                "reading": "CAT",
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

#[test]
fn get_should_require_auth() {
    let (client, _) = create_client();

    let response = send_get_request(&client, "/sentences");
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn get_should_return_empty_sentences_when_none_are_pending() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_get_request_with_auth(&client, "/sentences", &access_token);
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let data = json.get("data").unwrap().as_object().unwrap();

    assert_word_order(data, vec![]);
}

#[test]
fn get_should_return_pending_sentences_in_the_correct_order() {
    let words: [(&'static str, &'static str); 10] = [
        ("ペン", "ペン"),
        ("魑魅魍魎", "チミモウリョウ"),
        ("勝ち星", "カチボシ"),
        ("魑魅魍魎", "チミモウリョウ"),
        ("猫", "ネコ"),
        ("犬", "イヌ"),
        ("魑魅魍魎", "チミモウリョウ"),
        ("学校", "ガッコウ"),
        ("家", "イエ"),
        ("勝ち星", "カチボシ"),
    ];
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    for (dictionary_form, reading) in words {
        let response = send_post_request_with_json_and_auth(
            &client,
            "/sentences",
            &access_token,
            json!({
                "dictionary_form": dictionary_form,
                "reading": reading,
                "sentence": format!("a sentence with {}", dictionary_form),
            }),
        );
        assert_eq!(response.status(), Status::Ok);
    }

    let response = send_get_request_with_auth(&client, "/sentences", &access_token);
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let data = json.get("data").unwrap().as_object().unwrap();

    assert_word_order(
        &data,
        vec![
            ("魑魅魍魎", "チミモウリョウ"),
            ("魑魅魍魎", "チミモウリョウ"),
            ("魑魅魍魎", "チミモウリョウ"),
            ("勝ち星", "カチボシ"),
            ("勝ち星", "カチボシ"),
            ("家", "イエ"),
            ("学校", "ガッコウ"),
            ("犬", "イヌ"),
            ("猫", "ネコ"),
            ("ペン", "ペン"),
        ],
    );
}

fn assert_word_order(data: &Map<String, Value>, order: Vec<(&str, &str)>) {
    let response_sentences = data
        .get("sentences")
        .expect("should include 'sentences' field")
        .as_array()
        .expect("'sentences' should be an array");

    assert_eq!(order.len(), response_sentences.len());

    for (index, sentence) in response_sentences.iter().enumerate() {
        let word_object = sentence.as_object().expect("words should be objects");

        let response_dictionary_form = word_object
            .get("dictionary_form")
            .expect("should include 'dictionary_form' field")
            .as_str()
            .expect("'dictionary_field' should be a string");

        assert_eq!(response_dictionary_form, order[index].0);

        let response_reading = word_object
            .get("reading")
            .expect("should include 'reading' field")
            .as_str()
            .expect("'reading' should be a string");

        assert_eq!(response_reading, order[index].1);
    }
}

// todo: mined -> unmined if the same word got mined
