use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use log::info;
use pirulla_core::announcements::announcements;
use pirulla_core::data::SiteData;

/// More posts than this at once means something went wrong, so none is posted.
pub const MAX_POSTS: usize = 3;

/// Where the announcements go.
pub trait Publisher {
    /// Publishes `text` and answers the address of the post, when it is known.
    fn publish(&self, text: &str) -> Result<Option<String>>;
}

/// Logs the announcements instead of posting them.
pub struct DryRun;

impl Publisher for DryRun {
    fn publish(&self, text: &str) -> Result<Option<String>> {
        info!("The keys of X are not set, so not posting:\n{text}");
        Ok(None)
    }
}

/// The posts announcing the videos that the data file at `current` has and the
/// one at `previous` did not.
pub fn compose(
    previous: &Path,
    current: &Path,
    site_url: &str,
    now: DateTime<Utc>,
) -> Result<Vec<String>> {
    let read = |path: &Path| -> Result<SiteData> {
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    };
    let previous =
        read(previous).with_context(|| format!("could not read {}", previous.display()))?;
    let current = read(current).with_context(|| format!("could not read {}", current.display()))?;
    Ok(announcements(&previous, &current, site_url, now))
}

/// Publishes `posts` one after the other, and stops at the first that fails.
pub fn announce(posts: &[String], publisher: &impl Publisher) -> Result<()> {
    if posts.len() > MAX_POSTS {
        bail!(
            "refusing to post {} texts at once, more than the {MAX_POSTS} a run should ever find; post them by hand if they are right",
            posts.len()
        );
    }
    for (index, text) in posts.iter().enumerate() {
        let address = publisher
            .publish(text)
            .with_context(|| format!("could not post {} of {}", index + 1, posts.len()))?;
        if let Some(address) = address {
            info!("Posted {address}:\n{text}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::cell::RefCell;

    /// Keeps what it is given, and refuses the texts in `refused`.
    #[derive(Default)]
    struct FakeX {
        posted: RefCell<Vec<String>>,
        refused: Vec<&'static str>,
    }

    impl Publisher for FakeX {
        fn publish(&self, text: &str) -> Result<Option<String>> {
            if self.refused.contains(&text) {
                return Err(anyhow!("X refused it"));
            }
            self.posted.borrow_mut().push(text.to_string());
            Ok(Some(format!(
                "https://x.com/i/status/{}",
                self.posted.borrow().len()
            )))
        }
    }

    fn texts(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| text.to_string()).collect()
    }

    #[test]
    fn publishes_every_text_in_order() {
        let x = FakeX::default();

        announce(&texts(&["primeiro", "segundo"]), &x).unwrap();

        assert_eq!(*x.posted.borrow(), ["primeiro", "segundo"]);
    }

    #[test]
    fn stops_at_the_first_text_that_fails() {
        let x = FakeX {
            refused: vec!["segundo"],
            ..FakeX::default()
        };

        let error = announce(&texts(&["primeiro", "segundo", "terceiro"]), &x).unwrap_err();

        assert_eq!(*x.posted.borrow(), ["primeiro"]);
        assert_eq!(format!("{error:#}"), "could not post 2 of 3: X refused it");
    }

    #[test]
    fn refuses_more_texts_at_once_than_a_run_should_ever_find() {
        let x = FakeX::default();
        let too_many = vec!["texto".to_string(); MAX_POSTS + 1];

        let error = announce(&too_many, &x).unwrap_err();

        assert!(
            error.to_string().starts_with("refusing to post 4 texts"),
            "{error}"
        );
        assert!(x.posted.borrow().is_empty());
        announce(&too_many[1..], &x).unwrap();
    }

    fn data_file(dir: &tempfile::TempDir, name: &str, ids: &[&str]) -> std::path::PathBuf {
        let videos: Vec<String> = ids
            .iter()
            .map(|id| format!(r#"{{"id":"{id}","title":"Vídeo {id}","publishedAt":"2026-10-16T10:00:00Z","duration":600,"kind":"video"}}"#))
            .collect();
        let path = dir.path().join(name);
        std::fs::write(
            &path,
            format!(
                r#"{{"generatedAt":"2026-10-16T12:00:00Z","channelId":"UCchannel","videos":[{}]}}"#,
                videos.join(",")
            ),
        )
        .unwrap();
        path
    }

    #[test]
    fn composes_the_posts_for_the_videos_new_to_the_data_file() {
        let dir = tempfile::tempdir().unwrap();
        let previous = data_file(&dir, "previous.json", &["a"]);
        let current = data_file(&dir, "current.json", &["a", "b"]);
        let now = "2026-10-16T12:00:00Z".parse().unwrap();

        let posts = compose(
            &previous,
            &current,
            "https://example.github.io/pirulla-bot/",
            now,
        )
        .unwrap();

        assert_eq!(posts.len(), 1);
        assert!(posts[0].contains("\n🎥 Vídeo b\n"), "{}", posts[0]);
    }

    #[test]
    fn says_which_data_file_it_could_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let current = data_file(&dir, "current.json", &["a"]);
        let broken = dir.path().join("broken.json");
        std::fs::write(&broken, "{").unwrap();
        let now = Utc::now();

        let missing = compose(&dir.path().join("missing.json"), &current, "", now).unwrap_err();
        let broken = compose(&current, &broken, "", now).unwrap_err();

        assert!(
            format!("{missing:#}").starts_with("could not read "),
            "{missing:#}"
        );
        assert!(
            format!("{missing:#}").contains("missing.json"),
            "{missing:#}"
        );
        assert!(format!("{broken:#}").contains("broken.json"), "{broken:#}");
    }

    #[test]
    fn has_nothing_to_do_without_texts() {
        let x = FakeX::default();

        announce(&[], &x).unwrap();

        assert!(x.posted.borrow().is_empty());
    }
}
