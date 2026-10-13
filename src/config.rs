use anyhow::{Result, bail};

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
        let get = |name: &str| {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
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
}
