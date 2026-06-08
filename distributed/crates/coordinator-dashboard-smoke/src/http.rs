//! HTTP helpers shared across live smoke probes.

use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde_json::Value;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Build a short-lived blocking HTTP client for probe requests.
pub fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|error| anyhow!("failed to build HTTP client: {error}"))
}

/// GET JSON and include the upstream URL in errors.
pub fn request_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
) -> Result<T> {
    let response = client
        .get(url)
        .headers(headers.clone())
        .send()
        .map_err(|error| anyhow!("{url} failed: {error}"))?;
    decode_json_response(url, response)
}

pub fn request_json_with_auth<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    auth: &BasicAuth,
) -> Result<T> {
    let response =
        auth.apply(client.get(url)).send().map_err(|error| anyhow!("{url} failed: {error}"))?;
    decode_json_response(url, response)
}

fn decode_json_response<T: DeserializeOwned>(url: &str, response: Response) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        let body =
            response.text().unwrap_or_else(|error| format!("<could not read body: {error}>"));
        return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
    }

    response.json::<T>().map_err(|error| anyhow!("{url} returned non-JSON body: {error}"))
}

pub fn request_value(client: &Client, url: &str, headers: &HeaderMap) -> Result<Value> {
    request_json::<Value>(client, url, headers)
}

pub fn request_value_with_auth(client: &Client, url: &str, auth: &BasicAuth) -> Result<Value> {
    request_json_with_auth::<Value>(client, url, auth)
}

#[derive(Debug, Clone)]
pub struct BasicAuth {
    user: String,
    password: String,
}

impl BasicAuth {
    pub fn new(user: &str, password: &str) -> Self {
        Self { user: user.to_owned(), password: password.to_owned() }
    }

    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        request.basic_auth(&self.user, Some(&self.password))
    }
}

pub fn coordinator_headers(scrape_token: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    if let Some(token) = scrape_token {
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|error| anyhow!("failed to encode coordinator auth header: {error}"))?;
        headers.insert(AUTHORIZATION, value);
    }
    Ok(headers)
}

#[allow(dead_code)]
pub fn insert_accept_json(headers: &mut HeaderMap) {
    static ACCEPT: HeaderName = reqwest::header::ACCEPT;
    headers.insert(ACCEPT.clone(), HeaderValue::from_static("application/json"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafana_auth_keeps_user_and_secret_separate() {
        let auth_value = ["grafana", "test", "value"].join("-");
        let auth = BasicAuth::new("user", &auth_value);
        assert_eq!(auth.user, "user");
        assert_eq!(auth.password, auth_value);
    }

    #[test]
    fn coordinator_headers_emits_bearer_only_when_token_provided() {
        let none = coordinator_headers(None).unwrap();
        assert!(none.get(AUTHORIZATION).is_none());

        let with = coordinator_headers(Some("abc.def")).unwrap();
        assert_eq!(with.get(AUTHORIZATION).unwrap().to_str().unwrap(), "Bearer abc.def");
    }
}
