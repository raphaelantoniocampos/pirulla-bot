use std::collections::HashSet;

pub use pirulla_core::data::{Video, VideoKind};

/// The videos the website shows, in order of publication: the ones that have a
/// duration, each of them once.
pub fn timeline(videos: &[Video]) -> Vec<Video> {
    let mut seen = HashSet::new();
    let mut timeline: Vec<Video> = videos
        .iter()
        .filter(|video| video.duration > 0 && seen.insert(video.id.as_str()))
        .cloned()
        .collect();
    timeline.sort_by(|a, b| (a.published_at, &a.id).cmp(&(b.published_at, &b.id)));
    timeline
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    fn day(n: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, n, 12, 0, 0).unwrap()
    }

    fn video(id: &str, day_of_month: u32, duration: u64) -> Video {
        Video {
            id: id.to_string(),
            title: format!("Vídeo {id}"),
            published_at: day(day_of_month),
            duration,
            kind: VideoKind::Video,
        }
    }

    fn ids(videos: &[Video]) -> Vec<&str> {
        videos.iter().map(|video| video.id.as_str()).collect()
    }

    #[test]
    fn puts_the_videos_in_order_of_publication() {
        let videos = timeline(&[video("c", 3, 600), video("a", 1, 100), video("b", 2, 200)]);

        assert_eq!(ids(&videos), ["a", "b", "c"]);
    }

    #[test]
    fn videos_published_at_the_same_time_keep_a_stable_order() {
        let videos = timeline(&[video("b", 1, 200), video("a", 1, 100)]);

        assert_eq!(ids(&videos), ["a", "b"]);
    }

    #[test]
    fn leaves_out_the_videos_without_a_duration() {
        let videos = timeline(&[
            video("a", 1, 100),
            video("upcoming", 2, 0),
            video("b", 3, 200),
        ]);

        assert_eq!(ids(&videos), ["a", "b"]);
    }

    #[test]
    fn drops_duplicated_videos() {
        let videos = timeline(&[video("a", 1, 100), video("a", 1, 100), video("b", 2, 200)]);

        assert_eq!(ids(&videos), ["a", "b"]);
    }

    #[test]
    fn keeps_what_each_video_is() {
        let mut live = video("live", 2, 7200);
        live.kind = VideoKind::Live;

        let videos = timeline(&[live.clone(), video("a", 1, 100)]);

        assert_eq!(videos, [video("a", 1, 100), live]);
    }
}
