use super::*;

// only IdempotencyKey has logic worth testing; the numeric ids are pure wrappers

#[test]
fn accepts_a_uuid_shaped_key() {
    let raw = "b7f3c1a2-4d5e-4f60-9a1b-2c3d4e5f6a7b";
    let key = IdempotencyKey::new(raw.to_string()).expect("a hyphenated uuid is a valid key");
    assert_eq!(key.as_str(), raw);
}

#[test]
fn accepts_the_whole_permitted_charset() {
    let raw = "abcXYZ0189-_";
    let key = IdempotencyKey::new(raw.to_string()).expect("alphanumerics, hyphen and underscore");
    assert_eq!(key.as_str(), raw);
}

#[test]
fn rejects_empty() {
    assert_eq!(
        IdempotencyKey::new(String::new()),
        Err(EngineError::IdempotencyKeyInvalid)
    );
}

#[test]
fn accepts_exactly_the_maximum_length() {
    let raw = "a".repeat(IdempotencyKey::MAX_LEN);
    let key = IdempotencyKey::new(raw.clone()).expect("the cap itself is allowed");
    assert_eq!(key.as_str().len(), IdempotencyKey::MAX_LEN);
    assert_eq!(key.as_str(), raw);
}

#[test]
fn rejects_one_character_over_the_maximum() {
    let raw = "a".repeat(IdempotencyKey::MAX_LEN + 1);
    assert_eq!(
        IdempotencyKey::new(raw),
        Err(EngineError::IdempotencyKeyInvalid)
    );
}

#[test]
fn rejects_characters_outside_the_charset() {
    // a space, a path separator and a percent are the ones most likely to arrive
    // from a careless client, and none of them may reach the dedup map
    for raw in ["has space", "has/slash", "has%25"] {
        assert_eq!(
            IdempotencyKey::new(raw.to_string()),
            Err(EngineError::IdempotencyKeyInvalid),
            "{raw} should have been rejected"
        );
    }
}

#[test]
fn rejects_non_ascii() {
    // "kéy" is 4 bytes but 3 characters; the charset check rejects it before the
    // difference could ever matter to the length cap
    assert_eq!(
        IdempotencyKey::new("kéy".to_string()),
        Err(EngineError::IdempotencyKeyInvalid)
    );
}

#[test]
fn distinct_keys_are_distinct_map_entries() {
    // the property the whole type exists for: commands dedup by key equality
    let mut seen = std::collections::HashSet::new();
    assert!(seen.insert(IdempotencyKey::new("order-1".to_string()).expect("valid key")));
    assert!(seen.insert(IdempotencyKey::new("order-2".to_string()).expect("valid key")));
    assert!(!seen.insert(IdempotencyKey::new("order-1".to_string()).expect("valid key")));
    assert_eq!(seen.len(), 2);
}
