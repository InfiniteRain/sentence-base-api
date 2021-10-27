use bcrypt::verify;
use common::*;
use rocket::http::Status;
use sentence_base::models::user::User;
use serde_json::json;

mod common;

const TEST_USERNAME: &'static str = "test";
const TEST_EMAIL: &'static str = "example@domain.com";
const TEST_PASSWORD: &'static str = "password";

#[test]
fn register_should_validate() {
    let (client, _) = create_client();
    let response = send_post_request_with_json(
        &client,
        "/authenticate/register",
        json!({
            "username": "",
            "email": "",
            "password": ""
        }),
    );

    assert_eq!(response.status().code, 422);

    let json = response_to_json(response);

    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(
        &json,
        vec![
            "username".to_string(),
            "email".to_string(),
            "password".to_string(),
        ],
    );
}

#[test]
fn register_should_add_new_user() {
    let (client, database_url) = create_client();
    let response = send_post_request_with_json(
        &client,
        "/authenticate/register",
        json!({
            "username": TEST_USERNAME,
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(response.status().code, 200);

    let json = response_to_json(response);

    assert_success(&json);

    let data = json.get("data").unwrap();

    let response_username = data
        .get("username")
        .expect("should include 'username' field");

    assert_eq!(response_username, TEST_USERNAME);

    let response_email = data.get("email").expect("should include 'email' field");

    assert_eq!(response_email, TEST_EMAIL);

    let user_id = data
        .get("id")
        .expect("should include 'id' field")
        .as_u64()
        .expect("'id' should be an integer");

    let database_connection = create_database_connection(&database_url);
    let user = User::find_by_id(&database_connection, user_id as i32)
        .expect("the user should be queryable");

    verify(TEST_PASSWORD, &user.hash).expect("password should be hashed");
}

#[test]
fn register_should_fail_on_duplicate_data() {
    let (client, _) = create_client();
    let registration_response = send_post_request_with_json(
        &client,
        "/authenticate/register",
        json!({
            "username": TEST_USERNAME,
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(registration_response.status(), Status::Ok);

    let username_duplicate_response = send_post_request_with_json(
        &client,
        "/authenticate/register",
        json!({
            "username": TEST_USERNAME,
            "email": "different@domain.com",
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(username_duplicate_response.status(), Status::Conflict);
    let username_duplicate_json = response_to_json(username_duplicate_response);
    assert_fail(&username_duplicate_json, "Validation Error");
    assert_fail_reasons(
        &username_duplicate_json,
        vec!["duplicate username".to_string()],
    );

    let email_duplicate_response = send_post_request_with_json(
        &client,
        "/authenticate/register",
        json!({
            "username": "different_test",
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(email_duplicate_response.status(), Status::Conflict);
    let email_duplicate_json = response_to_json(email_duplicate_response);
    assert_fail(&email_duplicate_json, "Validation Error");
    assert_fail_reasons(&email_duplicate_json, vec!["duplicate email".to_string()]);
}
