use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sha1::Sha1;

/// Everything but the unreserved characters of RFC 3986 gets percent-encoded.
const ENCODED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// The keys of an X app and of the account it posts as.
#[derive(Clone, PartialEq)]
pub struct Credentials {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub token: String,
    pub token_secret: String,
}

/// Keeps the keys out of logs and panic messages.
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials(<redacted>)")
    }
}

/// Builds the `Authorization` header of an OAuth 1.0a request signed with
/// HMAC-SHA1.
///
/// `params` are the query string and form-encoded body parameters of the
/// request. JSON and multipart bodies are not part of the signature.
pub fn authorization_header(
    credentials: &Credentials,
    method: &str,
    url: &str,
    params: &[(&str, &str)],
    nonce: &str,
    timestamp: u64,
) -> String {
    let timestamp = timestamp.to_string();
    let mut oauth_params = vec![
        ("oauth_consumer_key", credentials.consumer_key.as_str()),
        ("oauth_nonce", nonce),
        ("oauth_signature_method", "HMAC-SHA1"),
        ("oauth_timestamp", timestamp.as_str()),
        ("oauth_token", credentials.token.as_str()),
        ("oauth_version", "1.0"),
    ];

    let mut signed_params: Vec<(String, String)> = oauth_params
        .iter()
        .chain(params)
        .map(|(name, value)| (encode(name), encode(value)))
        .collect();
    signed_params.sort();
    let signed_params: Vec<String> = signed_params
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    let base = format!(
        "{}&{}&{}",
        method.to_uppercase(),
        encode(url),
        encode(&signed_params.join("&"))
    );
    let key = format!(
        "{}&{}",
        encode(&credentials.consumer_secret),
        encode(&credentials.token_secret)
    );
    let mut mac =
        Hmac::<Sha1>::new_from_slice(key.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(base.as_bytes());
    let signature = STANDARD.encode(mac.finalize().into_bytes());

    oauth_params.push(("oauth_signature", &signature));
    oauth_params.sort();
    let fields: Vec<String> = oauth_params
        .iter()
        .map(|(name, value)| format!("{name}=\"{}\"", encode(value)))
        .collect();
    format!("OAuth {}", fields.join(", "))
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, ENCODED).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(
        consumer_key: &str,
        consumer_secret: &str,
        token: &str,
        token_secret: &str,
    ) -> Credentials {
        Credentials {
            consumer_key: consumer_key.to_string(),
            consumer_secret: consumer_secret.to_string(),
            token: token.to_string(),
            token_secret: token_secret.to_string(),
        }
    }

    /// The example from X's "Creating a signature" guide.
    #[test]
    fn signs_the_example_from_the_x_documentation() {
        let credentials = credentials(
            "xvz1evFS4wEEPTGEFPHBog",
            "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
            "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
            "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE",
        );

        let header = authorization_header(
            &credentials,
            "POST",
            "https://api.twitter.com/1.1/statuses/update.json",
            &[
                (
                    "status",
                    "Hello Ladies + Gentlemen, a signed OAuth request!",
                ),
                ("include_entities", "true"),
            ],
            "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg",
            1318622958,
        );

        assert_eq!(
            header,
            "OAuth oauth_consumer_key=\"xvz1evFS4wEEPTGEFPHBog\", \
             oauth_nonce=\"kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg\", \
             oauth_signature=\"hCtSmYh%2BiHYCEqBWrE7C7hYmtUk%3D\", \
             oauth_signature_method=\"HMAC-SHA1\", \
             oauth_timestamp=\"1318622958\", \
             oauth_token=\"370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb\", \
             oauth_version=\"1.0\""
        );
    }

    /// The example from appendix A of the OAuth Core 1.0 specification.
    #[test]
    fn signs_the_example_from_the_oauth_specification() {
        let credentials = credentials(
            "dpf43f3p2l4k3l03",
            "kd94hf93k423kf44",
            "nnch734d00sl2jdk",
            "pfkkdhi9sl3r4s00",
        );

        let header = authorization_header(
            &credentials,
            "GET",
            "http://photos.example.net/photos",
            &[("file", "vacation.jpg"), ("size", "original")],
            "kllo9940pd9333jh",
            1191242096,
        );

        assert!(
            header.contains("oauth_signature=\"tR3%2BTy81lMeYAr%2FFid0kMTYa%2FWM%3D\""),
            "{header}"
        );
    }

    #[test]
    fn request_parameters_are_signed_but_not_sent_in_the_header() {
        let credentials = credentials("key", "secret", "token", "token-secret");

        let plain = authorization_header(
            &credentials,
            "POST",
            "https://api.x.com/2/tweets",
            &[],
            "nonce",
            1,
        );
        let with_param = authorization_header(
            &credentials,
            "POST",
            "https://api.x.com/2/tweets",
            &[("a", "b")],
            "nonce",
            1,
        );

        assert_ne!(plain, with_param);
        assert!(!with_param.contains("a=\"b\""));
    }
}
