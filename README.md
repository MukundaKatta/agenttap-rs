# agenttap

[![crates.io](https://img.shields.io/crates/v/agenttap.svg)](https://crates.io/crates/agenttap)
[![docs.rs](https://docs.rs/agenttap/badge.svg)](https://docs.rs/agenttap)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Wire-level prompt introspection for LLM SDK calls. See exactly what was sent. Credentials redacted by default.

```toml
[dependencies]
agenttap = "0.1"
# Or, with reqwest-middleware integration:
agenttap = { version = "0.1", features = ["reqwest"] }
```

## Why

Five years into the SDK era, "what was actually sent to the model?" remains a hard question. SDK debug logging is verbose, leaks API keys, and reformats payloads. `agenttap` provides redaction primitives plus an optional `reqwest-middleware` impl so you can capture the exact wire payload.

## Quick start (manual)

```rust
use agenttap::{Tap, Redactor};
use serde_json::json;

let tap = Tap::new();

// After your HTTP call, hand the request and response details to the tap:
tap.record(
    "POST",
    "https://api.anthropic.com/v1/messages",
    [
        ("authorization".to_string(), "Bearer sk-ant-secret9876543210xyz".to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ],
    Some(json!({"model": "claude", "messages": [{"role": "user", "content": "hi"}]})),
    200,
    Vec::new(),
    None,
    420,
);

let last = tap.last().unwrap();
println!("{}", last.pretty_request());
println!("auth header now: {}", last.request_headers["authorization"]);
// auth header now: ***REDACTED***
```

## With `reqwest-middleware` (auto-capture)

Enable the `reqwest` feature, then plug `TapMiddleware` into a `reqwest_middleware::ClientBuilder`:

```rust,no_run
# #[cfg(feature = "reqwest")]
# {
use agenttap::{Tap, TapMiddleware};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;

let tap = Tap::new();
let client = ClientBuilder::new(Client::new())
    .with(TapMiddleware::new(tap.clone()))
    .build();

// `client` is now a reqwest_middleware::ClientWithMiddleware that records
// every request/response into `tap`.
# }
```

## Default redaction

- **Headers:** `authorization`, `x-api-key`, `api-key`, `cookie`, `set-cookie`, `anthropic-api-key`, `openai-organization`, `x-amz-security-token`, `x-google-api-key`.
- **Body strings matching:** OpenAI/Anthropic `sk-…`, AWS `AKIA…`, Google `AIza…`, Slack `xox[baprs]-…`.

Custom redactor:

```rust
use agenttap::{Tap, Redactor};

let red = Redactor::default()
    .with_extra_header("x-internal-secret")
    .with_extra_pattern(r"prod-token-\w+")
    .unwrap();
let tap = Tap::new().with_redactor(red);
```

## What it doesn't do

- Not a proxy. Not a server.
- v0.1 captures full bodies in memory only; persistence is your call.
- Streaming request bodies (where `as_bytes()` is `None`) aren't captured for the request side.

## Sibling: Python `agenttap`

Python users with httpx-based SDKs: see [MukundaKatta/agenttap](https://github.com/MukundaKatta/agenttap).

## License

MIT
