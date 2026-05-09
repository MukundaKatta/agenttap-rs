use agenttap::{diff, Redactor, Tap};
use serde_json::json;

fn h(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn redacts_sensitive_headers() {
    let tap = Tap::new();
    let req_h = h(&[
        ("Authorization", "Bearer sk-ant-secret9876543210xyz"),
        ("x-api-key", "abc123"),
        ("Content-Type", "application/json"),
    ]);
    tap.record("POST", "https://api.example.com/x", req_h, None, 200, vec![], None, 5);
    let last = tap.last().unwrap();
    assert_eq!(last.request_headers["authorization"], "***REDACTED***");
    assert_eq!(last.request_headers["x-api-key"], "***REDACTED***");
    assert_eq!(last.request_headers["content-type"], "application/json");
}

#[test]
fn redacts_api_key_patterns_in_body() {
    let tap = Tap::new();
    tap.record(
        "POST",
        "https://api.example.com/x",
        Vec::<(String, String)>::new(),
        Some(json!({"system": "Use this key sk-ant-thiseekrit1234567890 internally"})),
        200,
        vec![],
        None,
        5,
    );
    let last = tap.last().unwrap();
    let body = last.request_body.unwrap();
    let s = body["system"].as_str().unwrap();
    assert!(!s.contains("sk-ant-thiseekrit"), "sk-ant-... should be scrubbed");
    assert!(s.contains("***REDACTED***"));
}

#[test]
fn redactor_none_disables_scrubbing() {
    let tap = Tap::new().with_redactor(Redactor::none());
    let req_h = h(&[("Authorization", "Bearer plain-token")]);
    tap.record(
        "POST",
        "https://api.example.com/x",
        req_h,
        Some(json!({"system": "key sk-ant-thiseekrit1234567890"})),
        200,
        vec![],
        None,
        5,
    );
    let last = tap.last().unwrap();
    assert_eq!(last.request_headers["authorization"], "Bearer plain-token");
    assert!(last.request_body.unwrap()["system"]
        .as_str()
        .unwrap()
        .contains("sk-ant"));
}

#[test]
fn diff_shows_changed_field() {
    let tap = Tap::new();
    tap.record(
        "POST",
        "https://api.example.com/x",
        Vec::<(String, String)>::new(),
        Some(json!({"system": "A", "user": "hi"})),
        200,
        vec![],
        None,
        5,
    );
    tap.record(
        "POST",
        "https://api.example.com/x",
        Vec::<(String, String)>::new(),
        Some(json!({"system": "B", "user": "hi"})),
        200,
        vec![],
        None,
        5,
    );
    let calls = tap.all();
    let d = diff(&calls[0], &calls[1]);
    assert!(d.contains("\"system\": \"A\""), "diff should show -A: {d}");
    assert!(d.contains("\"system\": \"B\""), "diff should show +B: {d}");
}

#[test]
fn history_trims_to_capacity() {
    let tap = Tap::with_capacity(3);
    for i in 0..5 {
        tap.record(
            "POST",
            "https://api.example.com/x",
            Vec::<(String, String)>::new(),
            Some(json!({"i": i})),
            200,
            vec![],
            None,
            1,
        );
    }
    let calls = tap.all();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].request_body.as_ref().unwrap()["i"], 2);
    assert_eq!(calls[2].request_body.as_ref().unwrap()["i"], 4);
}

#[test]
fn extra_header_and_pattern() {
    let red = Redactor::default()
        .with_extra_header("x-secret-thing")
        .with_extra_pattern(r"super-secret-\d+")
        .unwrap();
    let tap = Tap::new().with_redactor(red);
    tap.record(
        "POST",
        "https://api.example.com/x",
        h(&[("X-Secret-Thing", "leaky"), ("normal", "value")]),
        Some(json!({"payload": "context: super-secret-12345 inside"})),
        200,
        vec![],
        None,
        1,
    );
    let last = tap.last().unwrap();
    assert_eq!(last.request_headers["x-secret-thing"], "***REDACTED***");
    assert_eq!(last.request_headers["normal"], "value");
    let s = last.request_body.unwrap()["payload"].as_str().unwrap().to_string();
    assert!(!s.contains("super-secret-12345"));
}

#[test]
fn pretty_request_format() {
    let tap = Tap::new();
    tap.record(
        "POST",
        "https://api.example.com/v1/x",
        Vec::<(String, String)>::new(),
        Some(json!({"k": "v"})),
        200,
        vec![],
        None,
        1,
    );
    let pr = tap.last().unwrap().pretty_request();
    assert!(pr.starts_with("POST https://api.example.com/v1/x"));
    assert!(pr.contains("\"k\": \"v\""));
}

#[test]
fn clones_share_history() {
    let tap = Tap::new();
    let tap2 = tap.clone();
    tap.record(
        "GET",
        "https://api.example.com/x",
        Vec::<(String, String)>::new(),
        None,
        200,
        vec![],
        None,
        1,
    );
    assert_eq!(tap2.all().len(), 1);
}
