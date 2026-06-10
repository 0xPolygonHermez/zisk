//! Bearer-token authentication for the coordinator HTTP scrape surface.

use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthFailure {
    Missing,
    Malformed,
    Mismatch,
}

static EXPECTED_TOKEN: OnceLock<Option<String>> = OnceLock::new();

pub fn set_expected_token(token: Option<String>) {
    let normalized = token.filter(|token| !token.is_empty());
    let _ = EXPECTED_TOKEN.set(normalized);
}

pub fn auth_enabled() -> bool {
    matches!(EXPECTED_TOKEN.get(), Some(Some(_)))
}

pub fn authorize(request_head: &str) -> Result<(), AuthFailure> {
    authorize_with(request_head, EXPECTED_TOKEN.get().and_then(|token| token.as_deref()))
}

pub(crate) fn authorize_with(
    request_head: &str,
    expected: Option<&str>,
) -> Result<(), AuthFailure> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let header = extract_authorization_header(request_head).ok_or(AuthFailure::Missing)?;
    let token = header.strip_prefix("Bearer ").ok_or(AuthFailure::Malformed)?;
    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(AuthFailure::Mismatch)
    }
}

fn extract_authorization_header(request_head: &str) -> Option<&str> {
    for line in request_head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("Authorization") {
            return Some(value.trim());
        }
    }
    None
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    let max_len = a.len().max(b.len());
    for i in 0..max_len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(av ^ bv);
    }
    diff == 0
}

pub fn unauthorized_response(reason: &str) -> String {
    format!(
        "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Bearer\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        reason.len(),
        reason
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(headers: &[(&str, &str)]) -> String {
        let mut request = String::from("GET /metrics HTTP/1.1\r\n");
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        request
    }

    #[test]
    fn disabled_when_expected_is_none() {
        assert!(authorize_with(&req(&[]), None).is_ok());
    }

    #[test]
    fn missing_header_is_missing() {
        assert_eq!(authorize_with(&req(&[]), Some("secret")), Err(AuthFailure::Missing));
    }

    #[test]
    fn wrong_scheme_is_malformed() {
        assert_eq!(
            authorize_with(&req(&[("Authorization", "Basic Zm9v")]), Some("secret")),
            Err(AuthFailure::Malformed)
        );
    }

    #[test]
    fn wrong_token_is_mismatch() {
        assert_eq!(
            authorize_with(&req(&[("Authorization", "Bearer wrong")]), Some("secret")),
            Err(AuthFailure::Mismatch)
        );
    }

    #[test]
    fn correct_token_ok() {
        assert!(authorize_with(&req(&[("Authorization", "Bearer secret")]), Some("secret")).is_ok());
    }

    #[test]
    fn header_name_case_insensitive() {
        assert!(authorize_with(&req(&[("authorization", "Bearer secret")]), Some("secret")).is_ok());
    }

    #[test]
    fn unauthorized_response_has_www_authenticate() {
        let response = unauthorized_response("missing bearer token");
        assert!(response.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        assert!(response.contains("WWW-Authenticate: Bearer"));
        assert!(response.ends_with("missing bearer token"));
    }
}
