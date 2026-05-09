use crate::redact::Redactor;
use parking_lot::Mutex;
use serde_json::Value;
use similar::TextDiff;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;

/// One captured request/response pair, redacted.
#[derive(Debug, Clone)]
pub struct TappedCall {
    /// When this was recorded.
    pub timestamp: SystemTime,
    /// HTTP method.
    pub method: String,
    /// Full request URL.
    pub url: String,
    /// Request headers (lower-cased keys, redacted values).
    pub request_headers: BTreeMap<String, String>,
    /// Request body, parsed as JSON if possible.
    pub request_body: Option<Value>,
    /// HTTP response status.
    pub response_status: u16,
    /// Response headers (lower-cased keys, redacted values).
    pub response_headers: BTreeMap<String, String>,
    /// Response body, parsed as JSON if possible.
    pub response_body: Option<Value>,
    /// Wall-clock latency in milliseconds.
    pub elapsed_ms: u64,
}

impl TappedCall {
    /// A pretty-printed `METHOD URL\n\n<body>` rendering.
    pub fn pretty_request(&self) -> String {
        let body = self
            .request_body
            .as_ref()
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
            .unwrap_or_default();
        format!("{} {}\n\n{}", self.method, self.url, body)
    }
}

/// Records LLM SDK HTTP traffic with redaction.
///
/// `Tap` is `Clone` (cheap; it shares an inner store via `Arc<Mutex<...>>`).
#[derive(Clone)]
pub struct Tap {
    redactor: Arc<Redactor>,
    history_size: usize,
    history: Arc<Mutex<Vec<TappedCall>>>,
}

impl Tap {
    /// Construct a new tap with the default redactor (history 1024).
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Construct with a custom history capacity.
    pub fn with_capacity(history_size: usize) -> Self {
        Self {
            redactor: Arc::new(Redactor::default()),
            history_size,
            history: Arc::new(Mutex::new(Vec::with_capacity(history_size.min(64)))),
        }
    }

    /// Override the redactor.
    pub fn with_redactor(mut self, r: Redactor) -> Self {
        self.redactor = Arc::new(r);
        self
    }

    /// Access the active redactor.
    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// Most recent call, if any.
    pub fn last(&self) -> Option<TappedCall> {
        self.history.lock().last().cloned()
    }

    /// All recorded calls, oldest first.
    pub fn all(&self) -> Vec<TappedCall> {
        self.history.lock().clone()
    }

    /// Drop all recorded history.
    pub fn reset(&self) {
        self.history.lock().clear();
    }

    /// Record a request/response pair.
    ///
    /// `request_body` and `response_body` can be `None` if absent. Strings
    /// containing valid JSON will be parsed; other strings are stored as
    /// `Value::String`.
    #[allow(clippy::too_many_arguments)]
    pub fn record<HR, HS>(
        &self,
        method: impl Into<String>,
        url: impl Into<String>,
        request_headers: HR,
        request_body: Option<Value>,
        response_status: u16,
        response_headers: HS,
        response_body: Option<Value>,
        elapsed_ms: u64,
    ) -> TappedCall
    where
        HR: IntoIterator<Item = (String, String)>,
        HS: IntoIterator<Item = (String, String)>,
    {
        let call = TappedCall {
            timestamp: SystemTime::now(),
            method: method.into(),
            url: url.into(),
            request_headers: self.redactor.headers(request_headers),
            request_body: request_body.map(|b| self.redactor.body(b)),
            response_status,
            response_headers: self.redactor.headers(response_headers),
            response_body: response_body.map(|b| self.redactor.body(b)),
            elapsed_ms,
        };
        let mut history = self.history.lock();
        history.push(call.clone());
        let excess = history.len().saturating_sub(self.history_size);
        if excess > 0 {
            history.drain(0..excess);
        }
        call
    }
}

impl Default for Tap {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified diff of two captured request bodies. Useful for "why did this
/// work yesterday and not today?"
pub fn diff(a: &TappedCall, b: &TappedCall) -> String {
    let pretty = |c: &TappedCall| -> String {
        c.request_body
            .as_ref()
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
            .unwrap_or_default()
    };
    let a_text = pretty(a);
    let b_text = pretty(b);
    TextDiff::from_lines(&a_text, &b_text)
        .unified_diff()
        .header("a", "b")
        .to_string()
}
