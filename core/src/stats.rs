//! What is computed from the videos: the average, and the totals by period.

use chrono::{DateTime, Datelike, Utc};
use chrono_tz::America::Sao_Paulo;

use crate::data::{Video, VideoKind};

pub const YEAR_MS: f64 = 365.25 * 86_400_000.0;
const MONTHS: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];

/// Which videos count in the average besides the regular ones, which always do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kinds {
    pub lives: bool,
    pub shorts: bool,
}

/// What counts in the average when the website opens, and in the posts on X.
pub const DEFAULT_KINDS: Kinds = Kinds {
    lives: false,
    shorts: true,
};

impl Kinds {
    pub fn count(self, kind: VideoKind) -> bool {
        match kind {
            VideoKind::Video => true,
            VideoKind::Live => self.lives,
            VideoKind::Short => self.shorts,
        }
    }
}

impl AsRef<Video> for Video {
    fn as_ref(&self) -> &Video {
        self
    }
}

/// The videos that count.
pub fn of_kinds(videos: &[Video], kinds: Kinds) -> Vec<Video> {
    videos
        .iter()
        .filter(|video| kinds.count(video.kind))
        .cloned()
        .collect()
}

/// A video with the rate at the time it came out.
#[derive(Debug, Clone, PartialEq)]
pub struct RatedVideo {
    pub video: Video,
    /// The average duration, in seconds, of the videos up to this one, rounded to hundredths.
    pub mean: f64,
    /// How much this video moved the average, in seconds.
    pub variation: f64,
    pub variation_percent: f64,
}

impl AsRef<Video> for RatedVideo {
    fn as_ref(&self) -> &Video {
        &self.video
    }
}

/// Gives each video the rate at the time. The videos must be in order of publication.
pub fn with_rate(videos: &[Video]) -> Vec<RatedVideo> {
    let mut total = 0;
    let mut previous_mean: Option<f64> = None;
    videos
        .iter()
        .enumerate()
        .map(|(index, video)| {
            total += video.duration;
            let mean = (total as f64 / (index + 1) as f64 * 100.0).round() / 100.0;
            let variation = previous_mean.map_or(0.0, |previous| mean - previous);
            let variation_percent = match previous_mean {
                Some(previous) if previous != 0.0 => variation / previous * 100.0,
                _ => 0.0,
            };
            previous_mean = Some(mean);
            RatedVideo {
                video: video.clone(),
                mean,
                variation,
                variation_percent,
            }
        })
        .collect()
}

/// The milliseconds since 1970, as the charts measure time.
pub fn millis(time: DateTime<Utc>) -> f64 {
    time.timestamp_millis() as f64
}

/// The videos published in the `years` before `now`; all of them when `years` is `None`.
pub fn last_years<T: AsRef<Video> + Clone>(
    videos: &[T],
    years: Option<u32>,
    now: DateTime<Utc>,
) -> Vec<T> {
    let Some(years) = years else {
        return videos.to_vec();
    };
    let since = millis(now) - f64::from(years) * YEAR_MS;
    videos
        .iter()
        .filter(|video| millis(video.as_ref().published_at) >= since)
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Year,
    Month,
}

/// The totals of a year or of a month.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodGroup {
    /// "2024", or "2024-03".
    pub key: String,
    /// "2024", or "mar/24".
    pub label: String,
    pub count: usize,
    pub total: u64,
    /// `None` when no video came out in the period.
    pub mean: Option<f64>,
}

/// Totals for each year or month, in the time of São Paulo, including the periods without videos.
pub fn group_by_period<T: AsRef<Video>>(videos: &[T], period: Period) -> Vec<PeriodGroup> {
    let (Some(first), Some(last)) = (videos.first(), videos.last()) else {
        return Vec::new();
    };
    let first = period_index(first.as_ref().published_at, period);
    let last = period_index(last.as_ref().published_at, period);
    let mut groups: Vec<PeriodGroup> = (first..=last)
        .map(|index| empty_group(index, period))
        .collect();

    for video in videos {
        let video = video.as_ref();
        let group = &mut groups[(period_index(video.published_at, period) - first) as usize];
        group.count += 1;
        group.total += video.duration;
    }
    for group in &mut groups {
        group.mean = (group.count > 0).then(|| group.total as f64 / group.count as f64);
    }
    groups
}

/// The year, or the number of months since year zero, that `time` falls in.
fn period_index(time: DateTime<Utc>, period: Period) -> i64 {
    let local = time.with_timezone(&Sao_Paulo);
    let year = i64::from(local.year());
    match period {
        Period::Year => year,
        Period::Month => year * 12 + i64::from(local.month0()),
    }
}

fn empty_group(index: i64, period: Period) -> PeriodGroup {
    let (key, label) = match period {
        Period::Year => (index.to_string(), index.to_string()),
        Period::Month => {
            let (year, month) = (index.div_euclid(12), index.rem_euclid(12) as usize);
            (
                format!("{year}-{:02}", month + 1),
                format!("{}/{:02}", MONTHS[month], year.rem_euclid(100)),
            )
        }
    };
    PeriodGroup {
        key,
        label,
        count: 0,
        total: 0,
        mean: None,
    }
}

/// How many videos there are, how long they are together and which is the longest.
#[derive(Debug, PartialEq)]
pub struct Summary<'a, T> {
    pub count: usize,
    pub total: u64,
    /// The first of the longest, or `None` when there are no videos.
    pub longest: Option<&'a T>,
}

pub fn summarize<T: AsRef<Video>>(videos: &[T]) -> Summary<'_, T> {
    let mut longest: Option<&T> = None;
    for video in videos {
        if longest.is_none_or(|longest| video.as_ref().duration > longest.as_ref().duration) {
            longest = Some(video);
        }
    }
    Summary {
        count: videos.len(),
        total: videos.iter().map(|video| video.as_ref().duration).sum(),
        longest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(id: &str, published_at: &str, duration: u64, kind: VideoKind) -> Video {
        Video {
            id: id.to_string(),
            title: id.to_uppercase(),
            published_at: published_at.parse().unwrap(),
            duration,
            kind,
        }
    }

    fn data() -> Vec<Video> {
        vec![
            video("a", "2023-12-31T12:00:00Z", 100, VideoKind::Video),
            // Already 2024 in UTC, but still the last night of 2023 in São Paulo.
            video("b", "2024-01-01T02:30:00Z", 200, VideoKind::Live),
            video("c", "2024-03-10T12:00:00Z", 600, VideoKind::Short),
        ]
    }

    fn ids<T: AsRef<Video>>(videos: &[T]) -> Vec<&str> {
        videos
            .iter()
            .map(|video| video.as_ref().id.as_str())
            .collect()
    }

    fn at(time: &str) -> DateTime<Utc> {
        time.parse().unwrap()
    }

    #[test]
    fn regular_videos_always_count_and_lives_and_shorts_count_when_asked_for() {
        let videos = data();
        let kinds = |lives, shorts| Kinds { lives, shorts };

        assert_eq!(ids(&of_kinds(&videos, kinds(false, false))), ["a"]);
        assert_eq!(ids(&of_kinds(&videos, kinds(true, false))), ["a", "b"]);
        assert_eq!(ids(&of_kinds(&videos, kinds(false, true))), ["a", "c"]);
        assert_eq!(ids(&of_kinds(&videos, kinds(true, true))), ["a", "b", "c"]);
    }

    #[test]
    fn by_default_the_lives_are_left_out_and_the_shorts_count() {
        assert_eq!(
            DEFAULT_KINDS,
            Kinds {
                lives: false,
                shorts: true
            }
        );
    }

    #[test]
    fn the_rate_is_the_average_duration_up_to_each_video_with_the_variation_it_caused() {
        let rated = with_rate(&data());

        let column = |field: fn(&RatedVideo) -> f64| rated.iter().map(field).collect::<Vec<_>>();
        assert_eq!(column(|video| video.mean), [100.0, 150.0, 300.0]);
        assert_eq!(column(|video| video.variation), [0.0, 50.0, 150.0]);
        assert_eq!(column(|video| video.variation_percent), [0.0, 50.0, 100.0]);
    }

    #[test]
    fn the_rate_counts_only_the_videos_it_is_given() {
        let rated = with_rate(&of_kinds(&data(), DEFAULT_KINDS));

        assert_eq!(ids(&rated), ["a", "c"]);
        assert_eq!(rated[1].mean, 350.0);
        assert_eq!(rated[1].variation, 250.0);
    }

    #[test]
    fn the_rate_is_rounded_to_hundredths_of_a_second() {
        let videos: Vec<Video> = [10, 10, 11]
            .iter()
            .enumerate()
            .map(|(index, duration)| {
                video(
                    &index.to_string(),
                    &format!("2024-01-0{}T12:00:00Z", index + 1),
                    *duration,
                    VideoKind::Video,
                )
            })
            .collect();

        assert_eq!(with_rate(&videos).last().unwrap().mean, 10.33);
    }

    #[test]
    fn a_range_keeps_the_videos_of_the_last_years() {
        let videos = data();

        assert_eq!(
            ids(&last_years(&videos, Some(1), at("2024-06-01T00:00:00Z"))),
            ["a", "b", "c"]
        );
        assert_eq!(
            ids(&last_years(&videos, Some(1), at("2025-02-01T00:00:00Z"))),
            ["c"]
        );
        assert_eq!(
            ids(&last_years(&videos, None, at("2030-01-01T00:00:00Z"))),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn a_range_does_not_change_the_variation_of_a_video() {
        let rated = with_rate(&data());

        let selected = last_years(&rated, Some(1), at("2025-02-01T00:00:00Z"));

        assert_eq!(selected[0].variation, 150.0);
    }

    #[test]
    fn groups_the_videos_by_the_year_they_were_published_in_sao_paulo() {
        let years = group_by_period(&data(), Period::Year);

        assert_eq!(
            years,
            [
                PeriodGroup {
                    key: "2023".to_string(),
                    label: "2023".to_string(),
                    count: 2,
                    total: 300,
                    mean: Some(150.0)
                },
                PeriodGroup {
                    key: "2024".to_string(),
                    label: "2024".to_string(),
                    count: 1,
                    total: 600,
                    mean: Some(600.0)
                },
            ]
        );
    }

    #[test]
    fn groups_by_month_keeping_the_months_without_videos() {
        let months = group_by_period(&data(), Period::Month);

        let summary: Vec<(&str, &str, usize, Option<f64>)> = months
            .iter()
            .map(|group| {
                (
                    group.key.as_str(),
                    group.label.as_str(),
                    group.count,
                    group.mean,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("2023-12", "dez/23", 2, Some(150.0)),
                ("2024-01", "jan/24", 0, None),
                ("2024-02", "fev/24", 0, None),
                ("2024-03", "mar/24", 1, Some(600.0)),
            ]
        );
    }

    #[test]
    fn summarizes_the_videos() {
        let videos = data();

        assert_eq!(
            summarize(&videos),
            Summary {
                count: 3,
                total: 900,
                longest: Some(&videos[2])
            }
        );
    }

    #[test]
    fn the_longest_is_the_first_of_the_longest() {
        let videos = vec![
            video("a", "2024-01-01T12:00:00Z", 100, VideoKind::Video),
            video("b", "2024-01-02T12:00:00Z", 100, VideoKind::Video),
        ];

        assert_eq!(summarize(&videos).longest.unwrap().id, "a");
    }

    #[test]
    fn copes_with_no_videos_at_all() {
        let none: Vec<Video> = Vec::new();

        assert!(with_rate(&none).is_empty());
        assert!(group_by_period(&none, Period::Year).is_empty());
        assert_eq!(
            summarize(&none),
            Summary {
                count: 0,
                total: 0,
                longest: None
            }
        );
    }
}
