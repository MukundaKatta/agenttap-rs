use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Default lower-cased header names whose values are scrubbed.
pub const DEFAULT_SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "api-key",
    "x-amz-security-token",
    "x-google-api-key",
    "openai-organization",
    "anthropic-api-key",
    "cookie",
    "set-cookie",
];

/// Default regex patterns for credential-like strings inside bodies.
pub const DEFAULT_VALUE_PATTERNS: &[&str] = &[
    r"sk-[A-Za-z0-9_-]{20,}",
    r"sk-ant-[A-Za-z0-9_-]{20,}",
    r"AKIA[0-9A-Z]{16}",
    r"AIza[0-9A-Za-z_-]{35}",
    r"xox[baprs]-[A-Za-z0-9-]{10,}",
];

fn compiled_default_patterns() -> &'static [Regex] {
    static PATS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATS.get_or_init(|| {
        DEFAULT_VALUE_PATTERNS
            .iter()
            .map(|p| Regex::new(p).expect("default pattern compiles"))
            .collect()
    })
}

/// Scrubs sensitive headers and credential-shaped strings.
#[derive(Clone)]
pub struct Redactor {
    sensitive_headers: HashSet<String>,
    value_patterns: Vec<Regex>,
    placeholder: String,
}

impl Default for Redactor {
    /// The default redactor: scrubs known sensitive headers and key shapes.
    fn default() -> Self {
        Self {
            sensitive_headers: DEFAULT_SENSITIVE_HEADERS
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
            value_patterns: compiled_default_patterns().to_vec(),
            placeholder: "***REDACTED***".to_string(),
        }
    }
}

impl Redactor {
    /// A no-op redactor: nothing is scrubbed. Use only for local debugging.
    pub fn none() -> Self {
        Self {
            sensitive_headers: HashSet::new(),
            value_patterns: Vec::new(),
            placeholder: "***REDACTED***".to_string(),
        }
    }

    /// Override the placeholder string.
    pub fn with_placeholder(mut self, ph: impl Into<String>) -> Self {
        self.placeholder = ph.into();
        self
    }

    /// Add an extra header name to the sensitive-headers set (case-insensitive).
    pub fn with_extra_header(mut self, h: &str) -> Self {
        self.sensitive_headers.insert(h.to_lowercase());
        self
    }

    /// Add an extra credential-shape pattern.
    pub fn with_extra_pattern(mut self, p: &str) -> Result<Self, regex::Error> {
        self.value_patterns.push(Regex::new(p)?);
        Ok(self)
    }

    /// Apply scrubbing to a header map (lowercases keys for lookup).
    pub fn headers<I, K, V>(&self, headers: I) -> std::collections::BTreeMap<String, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        headers
            .into_iter()
            .map(|(k, v)| {
                let k_lower = k.as_ref().to_lowercase();
                let v = if self.sensitive_headers.contains(&k_lower) {
                    self.placeholder.clone()
                } else {
                    self.scrub_value(v.as_ref()).into_owned()
                };
                (k_lower, v)
            })
            .collect()
    }

    /// Apply scrubbing to a JSON body in place.
    pub fn body(&self, body: serde_json::Value) -> serde_json::Value {
        match body {
            serde_json::Value::Object(map) => {
                let scrubbed = map.into_iter().map(|(k, v)| (k, self.body(v))).collect();
                serde_json::Value::Object(scrubbed)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(|v| self.body(v)).collect())
            }
            serde_json::Value::String(s) => {
                serde_json::Value::String(self.scrub_value(&s).into_owned())
            }
            other => other,
        }
    }

    fn scrub_value<'a>(&self, s: &'a str) -> std::borrow::Cow<'a, str> {
        let mut out: std::borrow::Cow<'a, str> = std::borrow::Cow::Borrowed(s);
        for pat in &self.value_patterns {
            if pat.is_match(&out) {
                out =
                    std::borrow::Cow::Owned(pat.replace_all(&out, &*self.placeholder).into_owned());
            }
        }
        out
    }
}
