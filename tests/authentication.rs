use bcrypt::verify;
use common::*;
use jwt::{SignWithKey, VerifyWithKey};
use rocket::http::Status;
use sentence_base::jwt::{get_access_token_expiry_time, get_jwt_secret_hmac, AccessClaims};
use sentence_base::models::user::User;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

mod common;

const TEST_USERNAME: &'static str = "test";
const TEST_EMAIL: &'static str = "example@domain.com";
const TEST_PASSWORD: &'static str = "password";

#[test]
fn register_should_validate() {
    let (client, _) = create_client();
    let response = send_post_request_with_json(
        &client,
        "/auth/register",
        json!({
            "username": "",
            "email": "",
            "password": ""
        }),
    );

    assert_eq!(response.status(), Status::UnprocessableEntity);

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
        "/auth/register",
        json!({
            "username": TEST_USERNAME,
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(response.status(), Status::Ok);

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
        "/auth/register",
        json!({
            "username": TEST_USERNAME,
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(registration_response.status(), Status::Ok);

    let username_duplicate_response = send_post_request_with_json(
        &client,
        "/auth/register",
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
        "/auth/register",
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

#[test]
fn login_should_validate() {
    let (client, _) = create_client();
    let response = send_post_request_with_json(
        &client,
        "/auth/login",
        json!({
            "email": "",
            "password": ""
        }),
    );

    assert_eq!(response.status(), Status::UnprocessableEntity);

    let json = response_to_json(response);

    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(&json, vec!["email".to_string(), "password".to_string()]);
}

#[test]
fn login_should_reject_on_wrong_creds() {
    let (client, _, _) = create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let cred_combinations: [(&str, &str); 3] = [
        ("wrong@domain.com", "wrong"),
        (TEST_EMAIL, "wrong"),
        ("wrong@domain.com", TEST_PASSWORD),
    ];

    for (username, password) in cred_combinations {
        let response = send_post_request_with_json(
            &client,
            "/auth/login",
            json!({
                "email": username,
                "password": password
            }),
        );

        assert_eq!(response.status(), Status::Unauthorized);
        let json = response_to_json(response);
        assert_fail(&json, "Invalid Credentials");
    }
}

#[test]
fn login_should_return_a_jwt() {
    let (client, _, _) = create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let response = send_post_request_with_json(
        &client,
        "/auth/login",
        json!({
            "email": TEST_EMAIL,
            "password": TEST_PASSWORD
        }),
    );

    assert_eq!(response.status(), Status::Ok);

    let json = response_to_json(response);

    assert_success(&json);

    let jwt_token = json
        .get("data")
        .expect("should include 'data' field")
        .as_object()
        .expect("'data' should be an object")
        .get("token")
        .expect("should include 'token' field")
        .as_str()
        .expect("'token' should be a string");

    let jwt_secret_hmac = get_jwt_secret_hmac();
    let jwt_expiry_time = get_access_token_expiry_time();
    let claims: AccessClaims = jwt_token
        .verify_with_key(&jwt_secret_hmac)
        .expect("key should be verified");
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    assert!(u64_diff(current_time, claims.iat) <= 10);
    assert!(u64_diff(current_time + jwt_expiry_time, claims.exp) <= 10);
}

#[test]
fn me_should_reject_no_token() {
    let (client, _) = create_client();

    let response = send_get_request(&client, "/auth/me");
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "No Token Provided");
}

#[test]
fn me_should_reject_malformed_token() {
    let (client, _) = create_client();

    let token = "wrong token".to_string();
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Malformed Token Provided");
}

#[test]
fn me_should_reject_future_iat_token() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(AccessClaims {
        iat: current_timestamp + 10,
        exp: current_timestamp + 3610,
        sub: 0,
        gen: 0,
    });
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with IAT in the Future Provided");
}

#[test]
fn me_should_reject_expired_token() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(AccessClaims {
        iat: current_timestamp,
        exp: current_timestamp - 3600,
        sub: 0,
        gen: 0,
    });
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Expired Token Provided");
}

#[test]
fn me_should_reject_invalid_subject() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(AccessClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: 0,
        gen: 0,
    });
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with Invalid Subject Provided");
}

#[test]
fn me_should_resolve_with_proper_token() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let token = generate_jwt_token_for_user(&user);
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let data = json.get("data").unwrap().as_object().unwrap();

    let username = data
        .get("username")
        .expect("should include 'username' field")
        .as_str()
        .expect("'username' should be a string");

    assert_eq!(username, user.username);

    let email = data
        .get("email")
        .expect("should include 'email' field")
        .as_str()
        .expect("'email' should be a string");

    assert_eq!(email, user.email);

    let id = data
        .get("id")
        .expect("should include 'id' field")
        .as_u64()
        .expect("'id' should be an integer");

    assert_eq!(id, user.id as u64);
}

#[test]
fn should_respect_token_generation() {
    let (client, user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let token = generate_jwt_token_for_user(&user);

    let first_response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(first_response.status(), Status::Ok);
    assert_eq!(user.increment_token_generation(&database_connection), Ok(1));

    let second_response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(second_response.status(), Status::Unauthorized);
    let second_response_json = response_to_json(second_response);
    assert_fail(&second_response_json, "Revoked Token Provided");
}

fn generate_jwt_token_for_user(user: &User) -> String {
    let current_timestamp = get_current_timestamp();
    generate_jwt_token(AccessClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: user.id,
        gen: 0,
    })
}

fn generate_jwt_token(claims: AccessClaims) -> String {
    claims
        .sign_with_key(&get_jwt_secret_hmac())
        .expect("token should be signed")
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn u64_diff(a: u64, b: u64) -> u64 {
    if a < b {
        b - a
    } else {
        a - b
    }
}
