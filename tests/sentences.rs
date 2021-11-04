use chrono::NaiveDateTime;
use common::*;
use diesel::RunQueryDsl;
use diesel::{BelongingToDsl, ExpressionMethods};
use diesel::{PgConnection, QueryDsl};
use itertools::__std_iter::FromIterator;
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use sentence_base::helpers::get_maximum_pending_sentences;
use sentence_base::jwt::TokenType;
use sentence_base::models::sentence::Sentence;
use sentence_base::models::user::User;
use sentence_base::models::word::Word;
use sentence_base::responses::SuccessResponse;
use sentence_base::routes::sentences::{GetBatchResponse, NewBatchResponse, NewSentenceResponse};
use sentence_base::schema::sentences as schema_sentences;
use sentence_base::schema::sentences::dsl::sentences as dsl_sentences;
use sentence_base::schema::sentences::{
    id as schema_sentences_id, is_pending as schema_sentences_is_pending,
    mining_batch_id as schema_sentences_mining_batch_id,
};
use sentence_base::schema::words as schema_words;
use sentence_base::schema::words::dsl::words as dsl_words;
use sentence_base::schema::words::is_mined as schema_words_is_mined;
use serde_json::{json, Map};
use std::collections::HashSet;

mod common;

const TEST_WORDS: [(&'static str, &'static str); 10] = [
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

#[test]
fn new_should_require_auth() {
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
fn new_should_validate() {
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
fn new_should_result_with_a_word_and_a_sentence_added() {
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
    let deserialized_response: SuccessResponse<NewSentenceResponse> =
        serde_json::from_value(json).expect("should deserialize response");
    let deserialized_data = deserialized_response.get_data();

    assert_eq!(deserialized_data.sentence.sentence_id, sentence.id);
    assert_eq!(deserialized_data.sentence.sentence, sentence.sentence);
    assert_eq!(
        deserialized_data.sentence.dictionary_form,
        word.dictionary_form
    );
    assert_eq!(deserialized_data.sentence.reading, word.reading);
    assert_eq!(deserialized_data.sentence.mining_frequency, word.frequency);
}

#[test]
fn new_should_increase_frequency_on_duplicate() {
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
fn new_should_not_add_more_sentences_over_the_limit() {
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
fn new_should_not_count_non_pending_sentences_towards_the_limit() {
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
        .is_pending_sentence_limit_reached(&database_connection)
        .expect("should resolve whether pending sentence limit was reached");

    assert!(is_sentences_pending_limit_reached);

    diesel::update(dsl_sentences.filter(schema_sentences_is_pending.eq(true)))
        .set(schema_sentences_is_pending.eq(false))
        .execute(&database_connection)
        .expect("query should execute");

    let is_sentences_pending_limit_reached_after_update = user
        .is_pending_sentence_limit_reached(&database_connection)
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
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let add_response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "dictionary_form": "cat",
            "reading": "CAT",
            "sentence": "the cat is sleeping.",
        }),
    );
    assert_eq!(add_response.status(), Status::Ok);

    diesel::update(dsl_sentences.filter(schema_sentences_id.eq(1)))
        .set(schema_sentences_is_pending.eq(false))
        .execute(&database_connection)
        .unwrap();

    let response = send_get_request_with_auth(&client, "/sentences", &access_token);
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let data = json.get("data").unwrap().as_object().unwrap();

    assert_word_order(data, vec![]);
}

#[test]
fn get_should_return_pending_sentences_in_the_correct_order() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    mine_test_words(&client, &access_token);

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

#[test]
fn new_batch_should_require_auth() {
    let (client, _) = create_client();

    let response = send_post_request_with_json(
        &client,
        "/sentences/batches",
        json!({
            "sentences": [1, 2, 3]
        }),
    );
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn new_batch_should_validate() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({
            "sentences": []
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(&json, vec!["sentences".to_string()]);
}

#[test]
fn new_batch_should_not_work_for_non_existent_sentences() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({
            "sentences": [1]
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Invalid Sentences Provided");
}

#[test]
fn new_batch_should_not_work_for_non_owned_sentences() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let new_user = User::register(
        &database_connection,
        "user2".to_string(),
        "user2@domain.com".to_string(),
        "password".to_string(),
    )
    .expect("should register user");

    let new_word = Word::new_or_increase_frequency(&database_connection, &new_user, "cat", "CAT")
        .expect("should add the word");
    let new_sentence = Sentence::new(
        &database_connection,
        &new_user,
        &new_word,
        "the cat is sleeping",
    )
    .expect("should add the sentence");

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({
            "sentences": [new_sentence.id]
        }),
    );
    assert_eq!(response.status(), Status::UnprocessableEntity);
    let json = response_to_json(response);
    assert_fail(&json, "Invalid Sentences Provided");
}

#[test]
fn new_batch_should_work() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);
    let sentence_ids = mine_test_words(&client, &access_token);

    let response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({ "sentences": sentence_ids }),
    );
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);
    let deserialized_response: SuccessResponse<NewBatchResponse> =
        serde_json::from_value(json).expect("should deserialize response");

    let sentence_batch: Vec<Sentence> = schema_sentences::table
        .filter(schema_sentences_mining_batch_id.eq(deserialized_response.get_data().batch_id))
        .filter(schema_sentences_is_pending.eq(false))
        .get_results(&database_connection)
        .expect("should execute find sentence batch query");

    let words: Vec<Word> = schema_words::table
        .filter(schema_words_is_mined.eq(true))
        .get_results(&database_connection)
        .expect("should execute find words query");

    let test_words_set: HashSet<(&str, &str)> = HashSet::from_iter(TEST_WORDS.iter().cloned());

    assert_eq!(sentence_batch.len(), TEST_WORDS.len());
    assert_eq!(words.len(), test_words_set.len());
}

#[test]
fn new_batch_rejects_when_submitting_the_same_batch_twice() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);
    let sentence_ids = mine_test_words(&client, &access_token);
    new_batch_from_words(&client, &access_token, &sentence_ids);

    let second_response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({ "sentences": sentence_ids }),
    );
    assert_eq!(second_response.status(), Status::UnprocessableEntity);
    let json = response_to_json(second_response);
    assert_fail(&json, "Invalid Sentences Provided");
}

#[test]
fn add_should_set_is_mined_to_false_when_mined_again() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);
    let sentence_ids = mine_test_words(&client, &access_token);

    let (_, first_word_query) = get_mined_from_id(&database_connection, sentence_ids[0]);
    assert_eq!(first_word_query.is_mined, false);

    new_batch_from_words(&client, &access_token, &sentence_ids);

    let (_, second_word_query) = get_mined_from_id(&database_connection, sentence_ids[0]);
    assert_eq!(second_word_query.is_mined, true);

    let mine_response = send_post_request_with_json_and_auth(
        &client,
        "/sentences",
        &access_token,
        json!({
            "dictionary_form": TEST_WORDS[0].0,
            "reading": TEST_WORDS[0].1,
            "sentence": format!("some sentence with {}", TEST_WORDS[0].0),
        }),
    );
    assert_eq!(mine_response.status(), Status::Ok);

    let (_, third_word_query) = get_mined_from_id(&database_connection, sentence_ids[0]);
    assert_eq!(third_word_query.is_mined, false);
}

#[test]
fn get_batch_should_require_auth() {
    let (client, _) = create_client();

    let response = send_get_request(&client, "/sentences/batches/1");
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn get_batch_should_fail_on_non_existent_batch() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let response = send_get_request_with_auth(&client, "/sentences/batches/1", &access_token);
    assert_eq!(response.status(), Status::NotFound);
    let json = response_to_json(response);
    assert_fail(&json, "Batch Not Found");
}

#[test]
fn get_batch_should_not_work_for_non_owned_batches() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let new_user = User::register(
        &database_connection,
        "user2".to_string(),
        "user2@domain.com".to_string(),
        "password".to_string(),
    )
    .expect("should register user");
    let new_user_access_token = generate_jwt_token_for_user(&new_user, TokenType::Access);

    let sentence_ids = mine_test_words(&client, &access_token);
    new_batch_from_words(&client, &access_token, &sentence_ids);

    let user_get_batch_response =
        send_get_request_with_auth(&client, "/sentences/batches/1", &access_token);
    assert_eq!(user_get_batch_response.status(), Status::Ok);

    let new_user_get_batch_response =
        send_get_request_with_auth(&client, "/sentences/batches/1", &new_user_access_token);
    assert_eq!(new_user_get_batch_response.status(), Status::NotFound);
    let json = response_to_json(new_user_get_batch_response);
    assert_fail(&json, "Batch Not Found");
}

#[test]
fn get_batch_should_work() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);
    let sentence_ids = mine_test_words(&client, &access_token);
    new_batch_from_words(&client, &access_token, &sentence_ids);

    let user_get_batch_response =
        send_get_request_with_auth(&client, "/sentences/batches/1", &access_token);
    assert_eq!(user_get_batch_response.status(), Status::Ok);
    let json = response_to_json(user_get_batch_response);
    assert_success(&json);

    let deserialized_response: SuccessResponse<GetBatchResponse> =
        serde_json::from_value(json).expect("should deserialize response");
    let deserialized_data = deserialized_response.get_data();

    assert_eq!(deserialized_data.sentences.len(), TEST_WORDS.len());

    for sentence in &deserialized_data.sentences {
        let (sentence_entry, word_entry) =
            get_mined_from_id(&database_connection, sentence.sentence_id);

        assert_eq!(sentence.sentence, sentence_entry.sentence);
        assert_eq!(sentence.dictionary_form, word_entry.dictionary_form);
        assert_eq!(sentence.reading, word_entry.reading);
        assert_eq!(sentence.mining_frequency, word_entry.frequency);
    }
}

#[test]
fn get_all_batches_should_validate() {
    let (client, _) = create_client();

    let response = send_get_request(&client, "/sentences/batches");
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn get_mining_batches_should_work() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);

    let first_get_batches_response =
        send_get_request_with_auth(&client, "/sentences/batches", &access_token);
    let json = response_to_json(first_get_batches_response);
    assert_success(&json);
    let first_batch = get_all_batches_from_json(&json);

    assert_eq!(first_batch.len(), 0);

    let second_batch_sentence_ids = mine_test_words(&client, &access_token);
    new_batch_from_words(&client, &access_token, &second_batch_sentence_ids);

    let second_get_batches_response =
        send_get_request_with_auth(&client, "/sentences/batches", &access_token);
    let json = response_to_json(second_get_batches_response);
    let second_batch = get_all_batches_from_json(&json);

    assert_eq!(second_batch.len(), 1);
    assert_eq!(second_batch[0].id, 1);

    let third_batch_sentence_ids = mine_test_words(&client, &access_token);
    new_batch_from_words(&client, &access_token, &third_batch_sentence_ids);

    let third_get_batches_response =
        send_get_request_with_auth(&client, "/sentences/batches", &access_token);
    let json = response_to_json(third_get_batches_response);
    let third_batch = get_all_batches_from_json(&json);

    assert_eq!(third_batch.len(), 2);
    assert_eq!(third_batch[0].id, 2);
    assert_eq!(third_batch[1].id, 1);
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GetAllBatchesResponse {
    pub batches: Vec<MiningBatchEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MiningBatchEntry {
    pub id: i32,
    pub created_at: NaiveDateTime,
}

fn get_mined_from_id(database_connection: &PgConnection, sentence_id: i32) -> (Sentence, Word) {
    schema_sentences::table
        .filter(schema_sentences_id.eq(sentence_id))
        .inner_join(dsl_words)
        .first::<(Sentence, Word)>(database_connection)
        .expect("should execute the find sentence query")
}

fn new_batch_from_words(client: &Client, access_token: &String, sentence_ids: &Vec<i32>) {
    let new_batch_response = send_post_request_with_json_and_auth(
        &client,
        "/sentences/batches",
        &access_token,
        json!({ "sentences": sentence_ids }),
    );
    assert_eq!(
        new_batch_response.status(),
        Status::Ok,
        "{:?}",
        sentence_ids
    );
}

fn mine_test_words(client: &Client, access_token: &String) -> Vec<i32> {
    let mut sentence_ids: Vec<i32> = vec![];

    for (dictionary_form, reading) in TEST_WORDS {
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
        let json = response_to_json(response);
        let deserialized_response: SuccessResponse<NewSentenceResponse> =
            serde_json::from_value(json).expect("should deserialize response");

        sentence_ids.push(deserialized_response.get_data().sentence.sentence_id);
    }

    sentence_ids
}

fn get_all_batches_from_json(json: &Value) -> Vec<MiningBatchEntry> {
    let deserialized_response: SuccessResponse<GetAllBatchesResponse> =
        serde_json::from_value(json.clone()).expect("should deserialize response");
    deserialized_response.get_data().batches.clone()
}
