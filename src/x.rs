use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use serde_json::{Value, json};

use crate::announce::Publisher;
use crate::http;
use crate::oauth::{Credentials, authorization_header};

const API_URL: &str = "https://api.x.com";

/// Posts on X through the v2 API, as the account of its credentials.
pub struct XClient {
    http: Client,
    base_url: String,
    credentials: Credentials,
}

impl XClient {
    pub fn new(credentials: Credentials) -> Result<Self> {
        Ok(Self {
            http: http::client()?,
            base_url: API_URL.to_string(),
            credentials,
        })
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn authorization(&self, url: &str) -> String {
        let nonce: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(32)
            .collect();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        authorization_header(&self.credentials, "POST", url, &[], &nonce, timestamp)
    }
}

impl Publisher for XClient {
    /// Posts `text` and answers the address of the post, unless X left its id out.
    fn publish(&self, text: &str) -> Result<Option<String>> {
        let url = format!("{}/2/tweets", self.base_url);
        // The body is JSON, so it is not part of the signature.
        let request = self
            .http
            .post(&url)
            .header(AUTHORIZATION, self.authorization(&url))
            .json(&json!({"text": text}));
        let created: Value = http::send(request, "X post")?;
        Ok(created
            .pointer("/data/id")
            .and_then(Value::as_str)
            .map(|id| format!("https://x.com/i/status/{id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server, ServerGuard};

    const SIGNED: &str = r#"^OAuth oauth_consumer_key="consumer-key", oauth_nonce="[A-Za-z0-9]{32}", oauth_signature="[^"]+", oauth_signature_method="HMAC-SHA1", oauth_timestamp="\d+", oauth_token="access-token", oauth_version="1.0"$"#;
    const SECRETS: [&str; 4] = [
        "consumer-key",
        "consumer-secret",
        "access-token",
        "access-secret",
    ];

    fn client(server: &ServerGuard) -> XClient {
        let credentials = Credentials {
            consumer_key: "consumer-key".to_string(),
            consumer_secret: "consumer-secret".to_string(),
            token: "access-token".to_string(),
            token_secret: "access-secret".to_string(),
        };
        XClient::new(credentials)
            .unwrap()
            .with_base_url(server.url())
    }

    fn refusal(server: &ServerGuard) -> String {
        format!("{:#}", client(server).publish("texto").unwrap_err())
    }

    #[test]
    fn posts_the_text_signed_by_the_account_and_answers_its_address() {
        let mut server = Server::new();
        let post = server
            .mock("POST", "/2/tweets")
            .match_header("authorization", Matcher::Regex(SIGNED.to_string()))
            .match_header("content-type", "application/json")
            .match_body(Matcher::Json(
                json!({"text": "🚨 VÍDEO NOVO 🚨\n🎥 Título"}),
            ))
            .with_status(201)
            .with_body(
                json!({"data": {"id": "1846987139428634858", "text": "🚨 VÍDEO NOVO 🚨"}})
                    .to_string(),
            )
            .expect(1)
            .create();

        let address = client(&server)
            .publish("🚨 VÍDEO NOVO 🚨\n🎥 Título")
            .unwrap();

        post.assert();
        assert_eq!(
            address.as_deref(),
            Some("https://x.com/i/status/1846987139428634858")
        );
    }

    #[test]
    fn a_post_x_took_without_saying_its_id_is_not_an_error() {
        let mut server = Server::new();
        server
            .mock("POST", "/2/tweets")
            .with_status(201)
            .with_body(json!({"data": {}}).to_string())
            .create();

        assert_eq!(client(&server).publish("texto").unwrap(), None);
    }

    #[test]
    fn says_why_x_refused_a_post() {
        let mut server = Server::new();
        server
            .mock("POST", "/2/tweets")
            .with_status(402)
            .with_body(json!({"title": "CreditsDepleted", "detail": "Your enrolled account does not have any credits to fulfill this request.", "status": 402}).to_string())
            .create();

        let error = refusal(&server);

        assert!(error.contains("402"), "{error}");
        assert!(
            error.contains("CreditsDepleted: Your enrolled account does not have any credits"),
            "{error}"
        );
    }

    #[test]
    fn says_when_x_takes_a_post_for_a_duplicate() {
        let mut server = Server::new();
        server
            .mock("POST", "/2/tweets")
            .with_status(403)
            .with_body(json!({"title": "Forbidden", "detail": "You are not allowed to create a Tweet with duplicate content.", "status": 403}).to_string())
            .create();

        let error = refusal(&server);

        assert!(error.contains("403"), "{error}");
        assert!(error.contains("duplicate content"), "{error}");
    }

    #[test]
    fn the_keys_never_show_up_in_errors() {
        let mut server = Server::new();
        server
            .mock("POST", "/2/tweets")
            .with_status(401)
            .with_body(
                json!({"errors": [{"message": "Could not authenticate you", "code": 32}]})
                    .to_string(),
            )
            .create();

        let error = refusal(&server);

        assert!(error.contains("Could not authenticate you"), "{error}");
        for secret in SECRETS {
            assert!(!error.contains(secret), "{error}");
        }
    }

    #[test]
    fn says_when_x_cannot_be_reached() {
        let client = client(&Server::new()).with_base_url("http://127.0.0.1:1");

        let error = format!("{:#}", client.publish("texto").unwrap_err());

        assert!(error.starts_with("X post failed"), "{error}");
        for secret in SECRETS {
            assert!(!error.contains(secret), "{error}");
        }
    }
}
