use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ERROR_BODY_CHARS: usize = 500;

pub(crate) fn client() -> Result<Client> {
    Client::builder()
        .timeout(TIMEOUT)
        .build()
        .context("could not set up the HTTP client")
}

/// Sends `request` and decodes the JSON it answers with.
///
/// Errors describe `what` failed and why, but never include the request url,
/// since it may carry credentials.
pub(crate) fn send<T: DeserializeOwned>(request: RequestBuilder, what: &str) -> Result<T> {
    decode(succeeded(respond(request, what)?, what)?, what)
}

/// Like [`send`], but asking for something that does not exist (a 404) answers
/// `None` instead of failing.
pub(crate) fn send_optional<T: DeserializeOwned>(
    request: RequestBuilder,
    what: &str,
) -> Result<Option<T>> {
    let response = respond(request, what)?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    decode(succeeded(response, what)?, what).map(Some)
}

fn respond(request: RequestBuilder, what: &str) -> Result<Response> {
    request
        .send()
        .map_err(reqwest::Error::without_url)
        .with_context(|| format!("{what} failed"))
}

fn succeeded(response: Response, what: &str) -> Result<Response> {
    let status = response.status();
    if !status.is_success() {
        let reason = error_reason(&response.text().unwrap_or_default());
        bail!("{what} failed with status {status}: {reason}");
    }
    Ok(response)
}

fn decode<T: DeserializeOwned>(response: Response, what: &str) -> Result<T> {
    response
        .json()
        .map_err(reqwest::Error::without_url)
        .with_context(|| format!("{what} returned an unexpected response"))
}

/// The reason an API gave for refusing a request, taken from its response body.
fn error_reason(body: &str) -> String {
    let json: Value = serde_json::from_str(body).unwrap_or_default();
    let text = |pointer: &str| json.pointer(pointer).and_then(Value::as_str);

    // Google, X problem details and X error lists, in this order.
    if let Some(message) = text("/error/message") {
        message.to_string()
    } else if let Some(detail) = text("/detail") {
        match text("/title") {
            Some(title) => format!("{title}: {detail}"),
            None => detail.to_string(),
        }
    } else if let Some(message) = text("/errors/0/message") {
        message.to_string()
    } else {
        let one_line = body.split_whitespace().collect::<Vec<_>>().join(" ");
        one_line.chars().take(MAX_ERROR_BODY_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_a_google_error_with_its_message() {
        let body = r#"{"error": {"code": 400, "message": "API key not valid.", "errors": [{"reason": "badRequest"}]}}"#;

        assert_eq!(error_reason(body), "API key not valid.");
    }

    #[test]
    fn explains_an_x_problem_with_its_title_and_detail() {
        let body = r#"{"title": "Forbidden", "detail": "You are not allowed to create a Tweet with duplicate content.", "status": 403}"#;

        assert_eq!(
            error_reason(body),
            "Forbidden: You are not allowed to create a Tweet with duplicate content."
        );
    }

    #[test]
    fn explains_an_x_error_list_with_its_first_message() {
        let body = r#"{"errors": [{"message": "Could not authenticate you", "code": 32}]}"#;

        assert_eq!(error_reason(body), "Could not authenticate you");
    }

    #[test]
    fn shows_any_other_body_on_one_line_and_cut_short() {
        assert_eq!(error_reason("  Unauthorized\n"), "Unauthorized");
        assert_eq!(
            error_reason("{\n  \"unexpected\": true\n}"),
            "{ \"unexpected\": true }"
        );
        assert_eq!(
            error_reason(&"x".repeat(2000)).chars().count(),
            MAX_ERROR_BODY_CHARS
        );
        assert_eq!(error_reason(""), "");
    }
}
