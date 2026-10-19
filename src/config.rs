use anyhow::{Context, Result, bail};

use crate::oauth::Credentials;

/// The environment variables with the keys of the X app and of the account it posts as.
const X_KEYS: [&str; 4] = [
    "TWITTER_API_KEY",
    "TWITTER_API_SECRET",
    "ACCESS_TOKEN",
    "ACCESS_SECRET_TOKEN",
];

/// The value of the variable `name`, unless it is missing or blank.
fn value(lookup: &impl Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The settings of the generator, read from environment variables.
pub struct Config {
    pub developer_key: String,
    pub channel_id: String,
    /// Replaces the address of the YouTube API, to run against a fake one.
    pub youtube_api_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the settings using `lookup` to get the value of each variable.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| value(&lookup, name);
        match (get("DEVELOPER_KEY"), get("CHANNEL_ID")) {
            (Some(developer_key), Some(channel_id)) => Ok(Self {
                developer_key,
                channel_id,
                youtube_api_url: get("YOUTUBE_API_URL"),
            }),
            (developer_key, channel_id) => {
                let missing: Vec<&str> = [
                    developer_key.is_none().then_some("DEVELOPER_KEY"),
                    channel_id.is_none().then_some("CHANNEL_ID"),
                ]
                .into_iter()
                .flatten()
                .collect();
                bail!("missing environment variables: {}", missing.join(", "))
            }
        }
    }
}

/// The settings of `announce post`, read from environment variables.
pub struct PostConfig {
    /// The texts to post, from the JSON list in `POSTS`.
    pub posts: Vec<String>,
    /// The keys to post with, or `None` to only show the texts.
    pub x_credentials: Option<Credentials>,
    /// Replaces the address of the X API, to run against a fake one.
    pub x_api_url: Option<String>,
}

impl PostConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the settings using `lookup` to get the value of each variable.
    ///
    /// The keys of X are all set or none is: setting only some of them is an error.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let posts = match value(&lookup, "POSTS") {
            Some(posts) => {
                serde_json::from_str(&posts).context("POSTS must be a JSON list of texts")?
            }
            None => Vec::new(),
        };
        let keys = X_KEYS.map(|name| value(&lookup, name));
        let missing: Vec<&str> = X_KEYS
            .iter()
            .zip(&keys)
            .filter(|(_, key)| key.is_none())
            .map(|(name, _)| *name)
            .collect();
        let x_credentials = match keys {
            [
                Some(consumer_key),
                Some(consumer_secret),
                Some(token),
                Some(token_secret),
            ] => Some(Credentials {
                consumer_key,
                consumer_secret,
                token,
                token_secret,
            }),
            _ if missing.len() == X_KEYS.len() => None,
            _ => bail!("missing environment variables: {}", missing.join(", ")),
        };
        Ok(Self {
            posts,
            x_credentials,
            x_api_url: value(&lookup, "X_API_URL"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<Config> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|name| vars.get(name).cloned())
    }

    fn rejection(vars: &[(&str, &str)]) -> String {
        config(vars)
            .err()
            .expect("the settings should be rejected")
            .to_string()
    }

    #[test]
    fn reads_the_key_and_the_channel() {
        let config =
            config(&[("DEVELOPER_KEY", " dev-key "), ("CHANNEL_ID", "UCchannel")]).unwrap();

        assert_eq!(config.developer_key, "dev-key");
        assert_eq!(config.channel_id, "UCchannel");
        assert_eq!(config.youtube_api_url, None);
    }

    #[test]
    fn names_every_missing_variable() {
        assert_eq!(
            rejection(&[]),
            "missing environment variables: DEVELOPER_KEY, CHANNEL_ID"
        );
        assert_eq!(
            rejection(&[("DEVELOPER_KEY", "dev-key"), ("CHANNEL_ID", "  ")]),
            "missing environment variables: CHANNEL_ID"
        );
    }

    #[test]
    fn talks_to_the_real_api_unless_told_otherwise() {
        let config = config(&[
            ("DEVELOPER_KEY", "dev-key"),
            ("CHANNEL_ID", "UCchannel"),
            ("YOUTUBE_API_URL", "http://localhost:1"),
        ])
        .unwrap();

        assert_eq!(
            config.youtube_api_url.as_deref(),
            Some("http://localhost:1")
        );
    }

    fn post_config(vars: &[(&str, &str)]) -> Result<PostConfig> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        PostConfig::from_lookup(|name| vars.get(name).cloned())
    }

    const X: [(&str, &str); 4] = [
        ("TWITTER_API_KEY", " api-key "),
        ("TWITTER_API_SECRET", "api-secret"),
        ("ACCESS_TOKEN", "token"),
        ("ACCESS_SECRET_TOKEN", "token-secret\n"),
    ];

    #[test]
    fn reads_the_texts_to_post_and_the_keys_of_the_old_bot() {
        let mut vars = X.to_vec();
        vars.push(("POSTS", r#"["primeiro", "segundo\n🚨"]"#));

        let config = post_config(&vars).unwrap();

        assert_eq!(config.posts, ["primeiro", "segundo\n🚨"]);
        let credentials = config.x_credentials.unwrap();
        assert_eq!(credentials.consumer_key, "api-key");
        assert_eq!(credentials.consumer_secret, "api-secret");
        assert_eq!(credentials.token, "token");
        assert_eq!(credentials.token_secret, "token-secret");
        assert_eq!(config.x_api_url, None);
    }

    #[test]
    fn without_texts_there_is_nothing_to_post() {
        assert!(post_config(&[]).unwrap().posts.is_empty());
        assert!(post_config(&[("POSTS", "[]")]).unwrap().posts.is_empty());
        assert!(post_config(&[("POSTS", " ")]).unwrap().posts.is_empty());
    }

    #[test]
    fn rejects_texts_that_are_not_a_json_list_of_strings() {
        for posts in ["not json", r#""one""#, "[1]"] {
            let error = post_config(&[("POSTS", posts)]).err().expect(posts);

            assert_eq!(error.to_string(), "POSTS must be a JSON list of texts");
        }
    }

    #[test]
    fn without_any_key_of_x_the_texts_are_only_shown() {
        assert!(post_config(&[]).unwrap().x_credentials.is_none());
        // How GitHub passes a secret that does not exist.
        let blank: Vec<(&str, &str)> = X.iter().map(|(name, _)| (*name, "")).collect();
        assert!(post_config(&blank).unwrap().x_credentials.is_none());
    }

    #[test]
    fn names_the_keys_of_x_that_are_missing_when_only_some_are_set() {
        let error = post_config(&[X[0], X[2], ("ACCESS_SECRET_TOKEN", " ")])
            .err()
            .expect("the settings should be rejected");

        assert_eq!(
            error.to_string(),
            "missing environment variables: TWITTER_API_SECRET, ACCESS_SECRET_TOKEN"
        );
    }

    #[test]
    fn posts_to_the_real_x_unless_told_otherwise() {
        let config = post_config(&[("X_API_URL", "http://localhost:1")]).unwrap();

        assert_eq!(config.x_api_url.as_deref(), Some("http://localhost:1"));
    }
}
