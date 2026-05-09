//! `reqwest-middleware` integration. Enabled with the `reqwest` feature.

use crate::Tap;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{Body, Request, Response};
use reqwest_middleware::{Middleware, Next};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::Instant;

/// Middleware that captures every request/response into a [`Tap`].
#[derive(Clone)]
pub struct TapMiddleware {
    tap: Tap,
}

impl TapMiddleware {
    /// Wrap a `Tap` as middleware.
    pub fn new(tap: Tap) -> Self {
        Self { tap }
    }
}

#[async_trait]
impl Middleware for TapMiddleware {
    async fn handle(
        &self,
        req: Request,
        ext: &mut http::Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // Snapshot request side. Cloning the body if present requires reading.
        let method = req.method().as_str().to_string();
        let url = req.url().to_string();
        let req_headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or("<non-utf8>").to_string(),
                )
            })
            .collect();
        let req_body_value: Option<Value> = match req.body() {
            Some(b) => match b.as_bytes() {
                Some(bytes) => parse_json_or_string(bytes),
                None => None, // streaming bodies not captured
            },
            None => None,
        };

        let t0 = Instant::now();
        let response = next.run(req, ext).await?;

        // Consume response body so we can record it; rebuild a Response.
        let status = response.status();
        let resp_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or("<non-utf8>").to_string(),
                )
            })
            .collect();
        // Save status + headers for rebuild
        let saved_status = status;
        let mut http_resp_builder = http::Response::builder().status(status);
        if let Some(b) = http_resp_builder.headers_mut() {
            for (k, v) in response.headers().iter() {
                b.insert(k.clone(), v.clone());
            }
        }

        let body_bytes: Bytes = response.bytes().await.map_err(reqwest_middleware::Error::from)?;
        let resp_body_value = parse_json_or_string(&body_bytes);
        let elapsed_ms = t0.elapsed().as_millis() as u64;

        self.tap.record(
            method,
            url,
            req_headers,
            req_body_value,
            saved_status.as_u16(),
            resp_headers,
            resp_body_value,
            elapsed_ms,
        );

        let new_resp = http_resp_builder
            .body(Body::from(body_bytes))
            .map_err(|e| reqwest_middleware::Error::Middleware(anyhow::anyhow!(e)))?;
        Ok(Response::from(new_resp))
    }
}

fn parse_json_or_string(bytes: &[u8]) -> Option<Value> {
    if bytes.is_empty() {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    Some(match serde_json::from_str::<Value>(text) {
        Ok(v) => v,
        Err(_) => Value::String(text.to_string()),
    })
}

// Hidden helper for stable BTreeMap keys when needed.
#[allow(dead_code)]
fn to_btreemap(v: Vec<(String, String)>) -> BTreeMap<String, String> {
    v.into_iter().collect()
}
