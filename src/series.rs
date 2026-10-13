use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;

/// A video as it comes from the channel.
#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub published_at: DateTime<Utc>,
    /// Length in seconds; zero for live and upcoming videos.
    pub duration: u64,
}

/// A video plus the channel's average duration right after it was published.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoRecord {
    pub id: String,
    pub title: String,
    pub published_at: DateTime<Utc>,
    /// Length in seconds.
    pub duration: u64,
    /// Average length, in seconds, of every video published up to this one.
    pub mean: f64,
}

/// Builds the channel history: videos in publication order, each with the
/// running average duration up to and including it.
pub fn build_series(videos: &[Video]) -> Vec<VideoRecord> {
    let mut seen = HashSet::new();
    let mut videos: Vec<&Video> = videos
        .iter()
        .filter(|video| video.duration > 0 && seen.insert(video.id.as_str()))
        .collect();
    videos.sort_by(|a, b| (a.published_at, &a.id).cmp(&(b.published_at, &b.id)));

    let mut total_duration = 0u64;
    videos
        .iter()
        .enumerate()
        .map(|(index, video)| {
            total_duration += video.duration;
            let mean = total_duration as f64 / (index + 1) as f64;
            VideoRecord {
                id: video.id.clone(),
                title: video.title.clone(),
                published_at: video.published_at,
                duration: video.duration,
                mean: (mean * 100.0).round() / 100.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(n: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, n, 12, 0, 0).unwrap()
    }

    fn video(id: &str, day_of_month: u32, duration: u64) -> Video {
        Video {
            id: id.to_string(),
            title: format!("Vídeo {id}"),
            published_at: day(day_of_month),
            duration,
        }
    }

    fn ids(records: &[VideoRecord]) -> Vec<&str> {
        records.iter().map(|r| r.id.as_str()).collect()
    }

    fn means(records: &[VideoRecord]) -> Vec<f64> {
        records.iter().map(|r| r.mean).collect()
    }

    #[test]
    fn computes_the_running_mean_in_publication_order() {
        let series = build_series(&[video("a", 1, 100), video("b", 2, 200), video("c", 3, 600)]);

        assert_eq!(ids(&series), ["a", "b", "c"]);
        assert_eq!(means(&series), [100.0, 150.0, 300.0]);
    }

    #[test]
    fn sorts_by_publication_date_before_averaging() {
        let series = build_series(&[video("c", 3, 600), video("a", 1, 100), video("b", 2, 200)]);

        assert_eq!(ids(&series), ["a", "b", "c"]);
        assert_eq!(means(&series), [100.0, 150.0, 300.0]);
    }

    #[test]
    fn videos_without_duration_do_not_count_towards_the_mean() {
        let series = build_series(&[video("a", 1, 100), video("live", 2, 0), video("b", 3, 200)]);

        assert_eq!(ids(&series), ["a", "b"]);
        assert_eq!(means(&series), [100.0, 150.0]);
    }

    #[test]
    fn drops_duplicated_videos() {
        let series = build_series(&[video("a", 1, 100), video("a", 1, 100), video("b", 2, 200)]);

        assert_eq!(ids(&series), ["a", "b"]);
        assert_eq!(means(&series), [100.0, 150.0]);
    }

    #[test]
    fn rounds_the_mean_to_two_decimal_places() {
        let series = build_series(&[video("a", 1, 10), video("b", 2, 10), video("c", 3, 11)]);

        assert_eq!(series[2].mean, 10.33);
    }

    #[test]
    fn carries_the_video_data() {
        let series = build_series(&[video("dQw4w9WgXcQ", 1, 212)]);

        assert_eq!(
            series,
            [VideoRecord {
                id: "dQw4w9WgXcQ".to_string(),
                title: "Vídeo dQw4w9WgXcQ".to_string(),
                published_at: day(1),
                duration: 212,
                mean: 212.0,
            }]
        );
    }
}
