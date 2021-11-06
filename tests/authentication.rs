use bcrypt::verify;
use common::*;
use jwt::VerifyWithKey;
use rocket::http::Status;
use rocket::local::blocking::{Client, LocalResponse};
use sentence_base::helpers::{get_access_token_expiry_time, get_refresh_token_expiry_time};
use sentence_base::jwt::{get_current_timestamp, get_jwt_secret_hmac, TokenClaims, TokenType};
use sentence_base::models::user::User;
use serde_json::json;

mod common;

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
fn register_should_fail_on_duplicate_data_in_different_case() {
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
            "username": TEST_USERNAME.to_uppercase(),
            "email": "different@domain.com",
            "password": TEST_PASSWORD.to_uppercase()
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
            "email": TEST_EMAIL.to_uppercase(),
            "password": TEST_PASSWORD.to_uppercase()
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

    let data = json.get("data").unwrap().as_object().unwrap();
    let access_token = data
        .get("access_token")
        .expect("should include 'access_token' field")
        .as_str()
        .expect("'access_token' should be a string");
    let refresh_token = data
        .get("refresh_token")
        .expect("should include 'refresh_token' field")
        .as_str()
        .expect("'refresh_token' should be a string");

    assert_jwt_token(access_token, TokenType::Access);
    assert_jwt_token(refresh_token, TokenType::Refresh);
}

#[test]
fn login_should_work_with_creds_in_different_case() {
    let (client, _, _) = create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let response = send_post_request_with_json(
        &client,
        "/auth/login",
        json!({
            "email": TEST_EMAIL.to_uppercase(),
            "password": TEST_PASSWORD
        }),
    );
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);
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
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp + 10,
        exp: current_timestamp + 3610,
        sub: 0,
        gen: 0,
        typ: TokenType::Access,
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
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp - 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Access,
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
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Access,
    });
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with Invalid Subject Provided");
}

#[test]
fn me_should_reject_invalid_type() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Refresh,
    });
    let response = send_get_request_with_auth(&client, "/auth/me", &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with Invalid Type Provided");
}

#[test]
fn me_should_resolve_with_proper_token() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let token = generate_jwt_token_for_user(&user, TokenType::Access);
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
fn refresh_should_validate() {
    let (client, _) = create_client();
    let access_token = "".to_string();
    let response = send_refresh_request(&client, &access_token);

    assert_eq!(response.status(), Status::UnprocessableEntity);

    let json = response_to_json(response);

    assert_fail(&json, "Validation Error");
    assert_fail_reasons_validation_fields(&json, vec!["refresh_token".to_string()]);
}

#[test]
fn refresh_should_reject_malformed_token() {
    let (client, _) = create_client();

    let access_token = "wrong token".to_string();
    let response = send_refresh_request(&client, &access_token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Malformed Token Provided");
}

#[test]
fn refresh_should_reject_future_iat_token() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp + 10,
        exp: current_timestamp + 3610,
        sub: 0,
        gen: 0,
        typ: TokenType::Refresh,
    });
    let response = send_refresh_request(&client, &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with IAT in the Future Provided");
}

#[test]
fn refresh_should_reject_expired_token() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp - 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Refresh,
    });
    let response = send_refresh_request(&client, &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Expired Token Provided");
}

#[test]
fn refresh_should_reject_invalid_subject() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Refresh,
    });
    let response = send_refresh_request(&client, &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with Invalid Subject Provided");
}

#[test]
fn refresh_should_reject_invalid_type() {
    let (client, _) = create_client();

    let current_timestamp = get_current_timestamp();
    let token = generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: 0,
        gen: 0,
        typ: TokenType::Access,
    });
    let response = send_refresh_request(&client, &token);
    assert_eq!(response.status(), Status::Unauthorized);
    let json = response_to_json(response);
    assert_fail(&json, "Token with Invalid Type Provided");
}

#[test]
fn refresh_should_resolve_with_proper_token() {
    let (client, user, _) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);

    let token = generate_jwt_token_for_user(&user, TokenType::Refresh);
    let response = send_refresh_request(&client, &token);
    assert_eq!(response.status(), Status::Ok);
    let json = response_to_json(response);
    assert_success(&json);

    let data = json.get("data").unwrap().as_object().unwrap();
    let access_token = data
        .get("access_token")
        .expect("should include 'access_token' field")
        .as_str()
        .expect("'access_token' should be a string");
    let refresh_token = data
        .get("refresh_token")
        .expect("should include 'refresh_token' field")
        .as_str()
        .expect("'refresh_token' should be a string");

    assert_jwt_token(access_token, TokenType::Access);
    assert_jwt_token(refresh_token, TokenType::Refresh);
}

#[test]
fn should_respect_token_generation() {
    let (client, mut user, database_connection) =
        create_client_and_register_user(TEST_USERNAME, TEST_EMAIL, TEST_PASSWORD);
    let access_token = generate_jwt_token_for_user(&user, TokenType::Access);
    let refresh_token = generate_jwt_token_for_user(&user, TokenType::Refresh);

    let first_me_response = send_get_request_with_auth(&client, "/auth/me", &access_token);
    assert_eq!(first_me_response.status(), Status::Ok);
    let first_refresh_response = send_refresh_request(&client, &refresh_token);
    assert_eq!(first_refresh_response.status(), Status::Ok);

    assert_eq!(user.increment_token_generation(&database_connection), Ok(1));

    let second_me_response = send_get_request_with_auth(&client, "/auth/me", &access_token);
    assert_eq!(second_me_response.status(), Status::Unauthorized);
    let second_refresh_response = send_refresh_request(&client, &refresh_token);
    assert_eq!(second_refresh_response.status(), Status::Unauthorized);

    let second_me_response_json = response_to_json(second_me_response);
    assert_fail(&second_me_response_json, "Revoked Token Provided");
    let second_refresh_response_json = response_to_json(second_refresh_response);
    assert_fail(&second_refresh_response_json, "Revoked Token Provided");
}

fn send_refresh_request<'a>(client: &'a Client, token: &'a String) -> LocalResponse<'a> {
    send_post_request_with_json(
        &client,
        "/auth/refresh",
        json!({
            "refresh_token": token,
        }),
    )
}

fn assert_jwt_token(token: &str, token_type: TokenType) {
    let jwt_secret_hmac = get_jwt_secret_hmac();
    let claims: TokenClaims = token
        .verify_with_key(&jwt_secret_hmac)
        .expect("key should be verified");
    let current_time = get_current_timestamp();
    let expiry_time = match token_type {
        TokenType::Access => get_access_token_expiry_time(),
        TokenType::Refresh => get_refresh_token_expiry_time(),
    };

    assert!(
        u64_diff(current_time, claims.iat) <= 10,
        "expected iat: {}, claimed: {}",
        current_time,
        claims.iat,
    );
    assert!(
        u64_diff(current_time + expiry_time, claims.exp) <= 10,
        "expected expiry time: {}, claimed: {}",
        current_time + expiry_time,
        claims.exp,
    );
}

fn u64_diff(a: u64, b: u64) -> u64 {
    if a < b {
        b - a
    } else {
        a - b
    }
}
