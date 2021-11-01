use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_query;
use jwt::SignWithKey;
use rocket::http::{ContentType, Header};
use rocket::local::blocking::{Client, LocalResponse};
use rocket::serde::json::Value;
use sentence_base;
use sentence_base::jwt::{get_current_timestamp, get_jwt_secret_hmac, TokenClaims, TokenType};
use sentence_base::models::user::User;
use std::sync::atomic::{AtomicUsize, Ordering};

static DATABASE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub const DATABASE_TEST_PORT: i32 = 5430;
pub const DATABASE_USERNAME: &'static str = "rocket";
pub const DATABASE_PASSWORD: &'static str = "rocket";

pub const TEST_USERNAME: &'static str = "test";
pub const TEST_EMAIL: &'static str = "example@domain.com";
pub const TEST_PASSWORD: &'static str = "password";

pub fn prepare_new_database() -> String {
    let database_name = format!("test_db_{}", DATABASE_COUNT.fetch_add(1, Ordering::Relaxed));

    let create_database_queries: [String; 2] = [
        format!("DROP DATABASE IF EXISTS {};", database_name),
        format!(
            "CREATE DATABASE {} OWNER {};",
            database_name, DATABASE_USERNAME
        ),
    ];

    let global_pg_url = format!(
        "postgres://{}:{}@localhost:{}/",
        DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_TEST_PORT
    );
    let global_pg_connection =
        PgConnection::establish(&global_pg_url).expect("database connection should be established");

    for query in create_database_queries {
        sql_query(&query)
            .execute(&global_pg_connection)
            .expect(&(format!("{}", &query)));
    }

    let local_database_url = format!(
        "postgres://{}:{}@localhost:{}/{}",
        DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_TEST_PORT, database_name
    );
    let local_database_connection = create_database_connection(&local_database_url);
    diesel_migrations::run_pending_migrations(&local_database_connection)
        .expect("migration should run");

    local_database_url
}

pub fn create_client() -> (Client, String) {
    let database_url = prepare_new_database();
    let rocket = sentence_base::rocket(&database_url);

    (
        Client::tracked(rocket).expect("client should launch"),
        database_url,
    )
}

pub fn create_client_and_register_user(
    username: &str,
    email: &str,
    password: &str,
) -> (Client, User, PgConnection) {
    let database_url = prepare_new_database();
    let rocket = sentence_base::rocket(&database_url);
    let database_connection = create_database_connection(&database_url);
    let user = User::register(
        &database_connection,
        username.to_string(),
        email.to_string(),
        password.to_string(),
    )
    .expect("should register");

    (
        Client::tracked(rocket).expect("client should launch"),
        user,
        database_connection,
    )
}

pub fn create_database_connection(connection_url: &String) -> PgConnection {
    PgConnection::establish(&connection_url).expect("database connection should be established")
}

pub fn send_post_request_with_json<'a>(
    client: &'a Client,
    url: &'a str,
    json: Value,
) -> LocalResponse<'a> {
    client
        .post(url)
        .header(ContentType::JSON)
        .body(json.to_string())
        .dispatch()
}

pub fn send_post_request_with_json_and_auth<'a>(
    client: &'a Client,
    url: &'a str,
    token: &'a str,
    json: Value,
) -> LocalResponse<'a> {
    client
        .post(url)
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", &token)))
        .body(json.to_string())
        .dispatch()
}

pub fn send_get_request<'a>(client: &'a Client, url: &'a str) -> LocalResponse<'a> {
    client.get(url).dispatch()
}

pub fn send_get_request_with_auth<'a>(
    client: &'a Client,
    url: &'a str,
    token: &String,
) -> LocalResponse<'a> {
    client
        .get(url)
        .header(Header::new("Authorization", format!("Bearer {}", &token)))
        .dispatch()
}

pub fn response_to_json(response: LocalResponse) -> Value {
    response.into_json::<Value>().expect("body must be json")
}

pub fn assert_fail(json: &Value, message: &str) {
    let response_status = json
        .get("status")
        .expect("should include 'status' field")
        .as_str();

    assert_eq!(response_status, Some("fail"));

    let response_message = json
        .get("message")
        .expect("should include 'message' field")
        .as_str();

    assert_eq!(response_message, Some(message));
}

fn collect_fail_reasons(json: &Value) -> Vec<String> {
    json.get("reasons")
        .expect("should include 'reasons' field")
        .as_array()
        .expect("'reasons' should be an array")
        .iter()
        .map(|reason| {
            reason
                .as_str()
                .expect("all elements in 'reasons' should be strings")
                .to_string()
        })
        .collect::<Vec<String>>()
}

pub fn assert_fail_reasons(json: &Value, reasons: Vec<String>) {
    let reasons_len = reasons.len();
    let response_reasons = collect_fail_reasons(&json);
    let response_reasons_len = response_reasons.len();

    assert_eq!(
        response_reasons_len, reasons_len,
        "response should have exactly {} reasons; received amount of response reasons: {}",
        reasons_len, response_reasons_len
    );

    for reason in &reasons {
        assert!(
            response_reasons.contains(&reason),
            "response reasons should contain '{}'; received response reasons: {:?}",
            reason,
            response_reasons
        );
    }
}

pub fn assert_fail_reasons_validation_fields(json: &Value, fields: Vec<String>) {
    let fields_len = fields.len();
    let response_reasons = collect_fail_reasons(&json);
    let response_reasons_len = response_reasons.len();

    assert_eq!(
        response_reasons_len, fields_len,
        "response should have exactly {} reasons; received amount of response reasons: {}",
        fields_len, response_reasons_len
    );

    'fields: for field in &fields {
        for response_reason in &response_reasons {
            if response_reason.starts_with(&(format!(r#"field "{}"#, field))) {
                continue 'fields;
            }
        }

        panic!(
            "response reasons should contain '{}' error; received response reasons: {:?}",
            field, response_reasons
        );
    }
}

pub fn assert_success(json: &Value) {
    let response_status = json
        .get("status")
        .expect("should include 'status' field")
        .as_str();

    assert_eq!(response_status, Some("success"));
    assert!(json.get("data").is_some());
}

pub fn generate_jwt_token_for_user(user: &User, token_type: TokenType) -> String {
    let current_timestamp = get_current_timestamp();
    generate_jwt_token(TokenClaims {
        iat: current_timestamp,
        exp: current_timestamp + 3600,
        sub: user.id,
        gen: 0,
        typ: token_type,
    })
}

pub fn generate_jwt_token(claims: TokenClaims) -> String {
    claims
        .sign_with_key(&get_jwt_secret_hmac())
        .expect("token should be signed")
}
