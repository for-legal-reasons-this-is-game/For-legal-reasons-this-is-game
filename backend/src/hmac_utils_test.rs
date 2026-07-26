use super::*;
use axum::http::Method;

const KEY: &[u8] = b"0123456789abcdef0123456789abcdef";
const OTHER_KEY: &[u8] = b"fedcba9876543210fedcba9876543210";

#[test]
fn construct_signature_is_deterministic() {
    let a = construct_signature(Method::POST, "/api/v1/users", 1_700_000_000, r#"{"x":1}"#, KEY);
    let b = construct_signature(Method::POST, "/api/v1/users", 1_700_000_000, r#"{"x":1}"#, KEY);
    assert_eq!(a, b);
}

#[test]
fn construct_signature_returns_hex_sha256() {
    let sig = construct_signature(Method::GET, "/health", 0, "", KEY);
    assert_eq!(sig.len(), 64);
    assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn construct_signature_changes_with_each_component() {
    let base = construct_signature(Method::POST, "/api/v1/users", 1_700_000_000, "body", KEY);

    assert_ne!(
        base,
        construct_signature(Method::GET, "/api/v1/users", 1_700_000_000, "body", KEY)
    );
    assert_ne!(
        base,
        construct_signature(Method::POST, "/api/v1/accounts", 1_700_000_000, "body", KEY)
    );
    assert_ne!(
        base,
        construct_signature(Method::POST, "/api/v1/users", 1_700_000_001, "body", KEY)
    );
    assert_ne!(
        base,
        construct_signature(Method::POST, "/api/v1/users", 1_700_000_000, "other", KEY)
    );
    assert_ne!(
        base,
        construct_signature(Method::POST, "/api/v1/users", 1_700_000_000, "body", OTHER_KEY)
    );
}

#[test]
fn construct_signature_known_vector() {
    // HMAC-SHA256("POST\n/api/v1/users\n1700000000\n{\"x\":1}", KEY)
    let sig = construct_signature(
        Method::POST,
        "/api/v1/users",
        1_700_000_000,
        r#"{"x":1}"#,
        KEY,
    );
    let mut mac = Hmac::<Sha256>::new_from_slice(KEY).unwrap();
    mac.update(b"POST\n/api/v1/users\n1700000000\n{\"x\":1}");
    let expected = hex::encode(mac.finalize().into_bytes());
    assert_eq!(sig, expected);
}

#[test]
fn verify_message_accepts_valid_signature() {
    let message = b"hello world";
    let mut mac = Hmac::<Sha256>::new_from_slice(KEY).unwrap();
    mac.update(message);
    let signature = hex::encode(mac.finalize().into_bytes());

    assert_eq!(verify_message(KEY, message, &signature), Ok(true));
}

#[test]
fn verify_message_rejects_wrong_signature() {
    let message = b"hello world";
    let bad_sig = "00".repeat(32);

    assert_eq!(verify_message(KEY, message, &bad_sig), Ok(false));
}

#[test]
fn verify_message_rejects_wrong_key() {
    let message = b"hello world";
    let mut mac = Hmac::<Sha256>::new_from_slice(KEY).unwrap();
    mac.update(message);
    let signature = hex::encode(mac.finalize().into_bytes());

    assert_eq!(verify_message(OTHER_KEY, message, &signature), Ok(false));
}

#[test]
fn verify_message_rejects_invalid_hex() {
    assert!(verify_message(KEY, b"msg", "not-hex").is_err());
    assert!(verify_message(KEY, b"msg", "abc").is_err()); // odd length
}

#[test]
fn verify_request_round_trip() {
    let method = Method::PUT;
    let path = "/api/v1/accounts/42";
    let timestamp = 1_712_345_678;
    let body = r#"{"balance":100}"#;

    let signature = construct_signature(method.clone(), path, timestamp, body, KEY);
    assert!(verify_request(
        method,
        path,
        timestamp,
        body,
        KEY,
        &signature
    ));
}

#[test]
fn verify_request_rejects_tampered_fields() {
    let signature = construct_signature(
        Method::POST,
        "/api/v1/users",
        1_700_000_000,
        "body",
        KEY,
    );

    assert!(!verify_request(
        Method::GET,
        "/api/v1/users",
        1_700_000_000,
        "body",
        KEY,
        &signature
    ));
    assert!(!verify_request(
        Method::POST,
        "/api/v1/other",
        1_700_000_000,
        "body",
        KEY,
        &signature
    ));
    assert!(!verify_request(
        Method::POST,
        "/api/v1/users",
        1_700_000_001,
        "body",
        KEY,
        &signature
    ));
    assert!(!verify_request(
        Method::POST,
        "/api/v1/users",
        1_700_000_000,
        "tampered",
        KEY,
        &signature
    ));
    assert!(!verify_request(
        Method::POST,
        "/api/v1/users",
        1_700_000_000,
        "body",
        OTHER_KEY,
        &signature
    ));
}

#[test]
fn verify_request_rejects_invalid_hex_signature() {
    assert!(!verify_request(
        Method::GET,
        "/",
        0,
        "",
        KEY,
        "not-a-valid-hex-signature"
    ));
}

#[test]
fn verify_message_matches_construct_signature_payload() {
    let method = Method::DELETE;
    let path = "/api/v1/instruments/BTC";
    let timestamp = 99u64;
    let body = "";
    let payload = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
    let signature = construct_signature(method, path, timestamp, body, KEY);

    assert_eq!(
        verify_message(KEY, payload.as_bytes(), &signature),
        Ok(true)
    );
}
