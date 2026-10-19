//! Which new videos to announce on X, and the posts that announce them.

use chrono::{DateTime, Duration, Utc};

use crate::data::SiteData;
use crate::stats::{DEFAULT_KINDS, of_kinds, with_rate};
use crate::tweet::compose_tweet;

/// A video published longer ago than this is not announced, even when it is new
/// to the data: it is old news, or a video that came back to the channel.
pub const MAX_AGE: Duration = Duration::days(7);

/// The posts announcing the videos of `current` that `previous` did not have, in
/// order of publication. Only the videos that count in the average the website
/// opens with are announced, each one with the average right after it, in the
/// same words as the "Gerar tweet" button of the website.
pub fn announcements(
    previous: &SiteData,
    current: &SiteData,
    site_url: &str,
    now: DateTime<Utc>,
) -> Vec<String> {
    let is_new = |id: &str| !previous.videos.iter().any(|video| video.id == id);
    with_rate(&of_kinds(&current.videos, DEFAULT_KINDS))
        .into_iter()
        .filter(|rated| is_new(&rated.video.id) && now - rated.video.published_at <= MAX_AGE)
        .map(|mut rated| {
            rated.video.title = without_mentions(&rated.video.title);
            compose_tweet(&rated, site_url)
        })
        .collect()
}

/// Puts an invisible space after every "@" that X would take for a mention,
/// since automated posts must not mention anyone.
pub fn without_mentions(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        result.push(character);
        let names_someone = characters
            .peek()
            .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_');
        if (character == '@' || character == '＠') && names_someone {
            result.push('\u{200B}');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Video, VideoKind};

    const SITE_URL: &str = "https://example.github.io/pirulla-bot/";

    fn now() -> DateTime<Utc> {
        "2026-10-16T12:00:00Z".parse().unwrap()
    }

    fn video(id: &str, hours_ago: i64, duration: u64, kind: VideoKind) -> Video {
        Video {
            id: id.to_string(),
            title: format!("Vídeo {id}"),
            published_at: now() - Duration::hours(hours_ago),
            duration,
            kind,
        }
    }

    fn data(videos: Vec<Video>) -> SiteData {
        SiteData {
            generated_at: now(),
            channel_id: "UCchannel".to_string(),
            videos,
        }
    }

    fn known() -> Vec<Video> {
        vec![
            video("a", 500, 100, VideoKind::Video),
            video("b", 400, 300, VideoKind::Video),
        ]
    }

    fn with(extra: Vec<Video>) -> SiteData {
        data(known().into_iter().chain(extra).collect())
    }

    fn announce(previous: &SiteData, current: &SiteData) -> Vec<String> {
        announcements(previous, current, SITE_URL, now())
    }

    #[test]
    fn announces_a_new_video_with_the_average_right_after_it_as_the_button_of_the_website_does() {
        let posts = announce(
            &with(vec![]),
            &with(vec![video("c", 2, 800, VideoKind::Video)]),
        );

        assert_eq!(
            posts,
            [[
                "🚨 VÍDEO NOVO 🚨",
                "🎥 Vídeo c",
                "🕒 Duração: 13min 20s",
                "🔗 https://www.youtube.com/watch?v=c",
                "",
                "⏳ 1 Pirulla: 6min 40s 000ms",
                "📊 Variação: 📈 +3min 20s (+100,00%)",
                "🌐 Gráficos: https://example.github.io/pirulla-bot/",
            ]
            .join("\n")]
        );
    }

    #[test]
    fn announces_the_regular_videos_and_the_shorts_and_not_the_lives() {
        let current = with(vec![
            video("l", 5, 7200, VideoKind::Live),
            video("s", 4, 60, VideoKind::Short),
            video("v", 3, 140, VideoKind::Video),
        ]);

        let posts = announce(&with(vec![]), &current);

        assert_eq!(posts.len(), 2);
        assert!(posts[0].contains("\n🎥 Vídeo s\n"), "{}", posts[0]);
        assert!(posts[1].contains("\n🎥 Vídeo v\n"), "{}", posts[1]);
        // (100 + 300 + 60 + 140) / 4: the live is not in the average either.
        assert!(
            posts[1].contains("\n⏳ 1 Pirulla: 2min 30s 000ms\n"),
            "{}",
            posts[1]
        );
    }

    #[test]
    fn a_video_the_previous_data_already_had_is_not_announced_again_even_when_its_kind_changed() {
        let previous = with(vec![video("c", 2, 800, VideoKind::Live)]);

        let posts = announce(&previous, &with(vec![video("c", 2, 800, VideoKind::Video)]));

        assert!(posts.is_empty());
    }

    #[test]
    fn nothing_is_announced_when_no_video_is_new() {
        assert!(announce(&with(vec![]), &with(vec![])).is_empty());
        assert!(
            announce(
                &with(vec![video("c", 2, 800, VideoKind::Video)]),
                &with(vec![])
            )
            .is_empty()
        );
    }

    #[test]
    fn a_video_published_too_long_ago_is_not_announced_even_when_it_is_new_to_the_data() {
        let limit = MAX_AGE.num_hours();

        let on_the_limit = with(vec![video("c", limit, 800, VideoKind::Video)]);
        let past_the_limit = with(vec![video("c", limit + 1, 800, VideoKind::Video)]);

        assert_eq!(announce(&with(vec![]), &on_the_limit).len(), 1);
        assert!(announce(&with(vec![]), &past_the_limit).is_empty());
        assert_eq!(MAX_AGE, Duration::days(7));
    }

    #[test]
    fn a_mention_in_a_title_does_not_reach_anyone() {
        let mut guest = video("c", 2, 800, VideoKind::Video);
        guest.title = "Papo com @the_mingau".to_string();

        let posts = announce(&with(vec![]), &with(vec![guest]));

        assert!(
            posts[0].contains("\n🎥 Papo com @\u{200B}the_mingau\n"),
            "{}",
            posts[0]
        );
    }

    #[test]
    fn breaks_every_mention_without_changing_how_the_text_looks() {
        assert_eq!(
            without_mentions("@the_mingau e ＠Outro_1"),
            "@\u{200B}the_mingau e ＠\u{200B}Outro_1"
        );
        assert_eq!(without_mentions("@ sozinho, 100@%"), "@ sozinho, 100@%");
        assert_eq!(without_mentions("Sem menção"), "Sem menção");
    }
}
