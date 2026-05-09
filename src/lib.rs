//! Wire-level prompt introspection for LLM SDK calls.
//!
//! `agenttap` records the exact request/response pairs flying out to your
//! LLM provider, with `Authorization`, `x-api-key`, and known credential
//! patterns scrubbed by default. v0.1 provides the data types and the
//! redactor; with the `reqwest` feature it also installs as a
//! [`reqwest_middleware::Middleware`].
//!
//! # Quick start (manual recording)
//!
//! ```
//! use agenttap::{Tap, Redactor};
//! use serde_json::json;
//!
//! let tap = Tap::new();
//!
//! // Record manually after each call:
//! let req_headers: Vec<(String, String)> = vec![
//!     ("authorization".into(), "Bearer sk-ant-thiseekrit1234567890".into()),
//!     ("content-type".into(), "application/json".into()),
//! ];
//! let resp_headers: Vec<(String, String)> = Vec::new();
//! tap.record(
//!     "POST",
//!     "https://api.anthropic.com/v1/messages",
//!     req_headers,
//!     Some(json!({"model": "claude", "messages": [{"role": "user", "content": "hi"}]})),
//!     200,
//!     resp_headers,
//!     None,
//!     420,
//! );
//!
//! let last = tap.last().unwrap();
//! assert_eq!(last.request_headers["authorization"], "***REDACTED***");
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod redact;
mod tap;

pub use crate::redact::{Redactor, DEFAULT_SENSITIVE_HEADERS, DEFAULT_VALUE_PATTERNS};
pub use crate::tap::{diff, Tap, TappedCall};

#[cfg(feature = "reqwest")]
mod reqwest_middleware_impl;
#[cfg(feature = "reqwest")]
pub use crate::reqwest_middleware_impl::TapMiddleware;
