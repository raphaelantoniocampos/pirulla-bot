use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What YouTube lists a video as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoKind {
    /// A regular upload.
    #[default]
    Video,
    /// A broadcast that went out live.
    Live,
    Short,
}

/// A video of the channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub title: String,
    pub published_at: DateTime<Utc>,
    /// Length in seconds; zero for lives that are still running and for upcoming videos.
    pub duration: u64,
    /// Data files written before the kinds were told apart have none: every video was a regular one.
    #[serde(default)]
    pub kind: VideoKind,
}

impl Video {
    pub fn url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.id)
    }
}

/// Everything the website shows: the contents of its data file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteData {
    pub generated_at: DateTime<Utc>,
    pub channel_id: String,
    /// In order of publication.
    pub videos: Vec<Video>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_data_file() {
        let data: SiteData = serde_json::from_str(
            r#"{
"generatedAt":"2026-10-13T19:24:58Z",
"channelId":"UCchannel",
"videos":[
{"id":"a","title":"Vídeo \"a\"","publishedAt":"2024-01-01T12:00:00Z","duration":100,"kind":"live"}
]
}"#,
        )
        .unwrap();

        assert_eq!(data.channel_id, "UCchannel");
        assert_eq!(data.videos[0].title, "Vídeo \"a\"");
        assert_eq!(data.videos[0].duration, 100);
        assert_eq!(data.videos[0].kind, VideoKind::Live);
        assert_eq!(data.videos[0].url(), "https://www.youtube.com/watch?v=a");
    }

    #[test]
    fn a_video_without_a_kind_is_a_regular_video() {
        let video: Video = serde_json::from_str(
            r#"{"id":"a","title":"A","publishedAt":"2023-12-31T12:00:00Z","duration":100}"#,
        )
        .unwrap();

        assert_eq!(video.kind, VideoKind::Video);
    }

    #[test]
    fn a_video_is_written_with_its_kind_in_lower_case() {
        let video = Video {
            id: "s".to_string(),
            title: "Vídeo s".to_string(),
            published_at: "2024-01-01T12:00:00Z".parse().unwrap(),
            duration: 30,
            kind: VideoKind::Short,
        };

        assert_eq!(
            serde_json::to_string(&video).unwrap(),
            r#"{"id":"s","title":"Vídeo s","publishedAt":"2024-01-01T12:00:00Z","duration":30,"kind":"short"}"#
        );
    }
}
