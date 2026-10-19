//! The post announcing a video, posted on X by the bot or by hand from the website.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::format::{MINUS, format_duration, format_duration_ms, format_percent, format_variation};
use crate::stats::RatedVideo;

/// The most a post can weigh, as X counts it.
pub const MAX_TWEET_LENGTH: usize = 280;

/// What X counts for a link, whatever its real length.
const LINK_LENGTH: usize = 23;
const ELLIPSIS: char = '…';

/// What `encodeURIComponent` leaves as it is.
const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// The length of `text` as X counts it: every link counts as 23 characters, and
/// the characters outside the Latin ranges, such as emoji, count as two.
///
/// X also turns bare domains such as `g1.com` into links. Words that look like
/// one count as themselves plus a link, so that the result is never too short.
pub fn weighted_length(text: &str) -> usize {
    words_and_spaces(text)
        .map(|word| {
            if word.starts_with("https://") || word.starts_with("http://") {
                LINK_LENGTH
            } else if looks_like_domain(word) {
                word.chars().map(character_weight).sum::<usize>() + LINK_LENGTH
            } else {
                word.chars().map(character_weight).sum()
            }
        })
        .sum()
}

/// The words of `text` and the runs of spaces between them, in order.
fn words_and_spaces(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let end = rest
            .find(|character: char| character.is_whitespace() != first.is_whitespace())
            .unwrap_or(rest.len());
        let (piece, after) = rest.split_at(end);
        rest = after;
        Some(piece)
    })
}

fn character_weight(character: char) -> usize {
    match u32::from(character) {
        0..=0x10FF | 0x2000..=0x200D | 0x2010..=0x201F | 0x2032..=0x2037 => 1,
        _ => 2,
    }
}

/// Whether `word` has a dot between a letter or digit and a letter, as in `g1.com`.
fn looks_like_domain(word: &str) -> bool {
    let characters: Vec<char> = word.chars().collect();
    characters
        .windows(3)
        .any(|window| window[0].is_alphanumeric() && window[1] == '.' && window[2].is_alphabetic())
}

/// Cuts `text` at the longest length that weighs at most `max_length`, ending it with an ellipsis.
fn truncate(text: &str, max_length: usize) -> String {
    if weighted_length(text) <= max_length {
        return text.to_string();
    }
    text.char_indices()
        .rev()
        .filter(|(end, _)| *end > 0)
        .map(|(end, _)| format!("{}{ELLIPSIS}", text[..end].trim_end()))
        .find(|cut| weighted_length(cut) <= max_length)
        .unwrap_or_default()
}

/// Writes the post announcing `video`, with a link to the website at `site_url`.
/// The title is cut when the post would not fit in [`MAX_TWEET_LENGTH`].
pub fn compose_tweet(video: &RatedVideo, site_url: &str) -> String {
    let variation = format_variation(video.variation);
    let trend = if variation.starts_with(MINUS) {
        "📉"
    } else {
        "📈"
    };
    let write = |title: &str| {
        [
            "🚨 VÍDEO NOVO 🚨".to_string(),
            format!("🎥 {title}"),
            format!(
                "🕒 Duração: {}",
                format_duration(video.video.duration as f64)
            ),
            format!("🔗 {}", video.video.url()),
            String::new(),
            format!("⏳ 1 Pirulla: {}", format_duration_ms(video.mean)),
            format!(
                "📊 Variação: {trend} {variation} ({})",
                format_percent(video.variation_percent)
            ),
            format!("🌐 Gráficos: {site_url}"),
        ]
        .join("\n")
    };

    let room = MAX_TWEET_LENGTH.saturating_sub(weighted_length(&write("")));
    write(&truncate(&video.video.title, room))
}

/// The address that opens X with `text` ready to be posted.
pub fn intent_url(text: &str) -> String {
    format!(
        "https://x.com/intent/post?text={}",
        utf8_percent_encode(text, URI_COMPONENT)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Video, VideoKind};

    const VIDEO_ID: &str = "dQw4w9WgXcQ";
    const VIDEO_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
    const SITE_URL: &str = "https://example.github.io/pirulla-bot/";

    fn video(title: &str) -> RatedVideo {
        RatedVideo {
            video: Video {
                id: VIDEO_ID.to_string(),
                title: title.to_string(),
                published_at: "2024-01-01T12:00:00Z".parse().unwrap(),
                duration: 1800,
                kind: VideoKind::Video,
            },
            mean: 1513.75,
            variation: 0.3,
            variation_percent: 0.0198,
        }
    }

    fn line(tweet: &str, index: usize) -> &str {
        tweet.lines().nth(index).unwrap()
    }

    #[test]
    fn announces_a_video_that_raised_the_average() {
        assert_eq!(
            compose_tweet(&video("O que é ciência?"), SITE_URL),
            [
                "🚨 VÍDEO NOVO 🚨",
                "🎥 O que é ciência?",
                "🕒 Duração: 30min 00s",
                "🔗 https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                "",
                "⏳ 1 Pirulla: 25min 13s 750ms",
                "📊 Variação: 📈 +0,30s (+0,02%)",
                "🌐 Gráficos: https://example.github.io/pirulla-bot/",
            ]
            .join("\n")
        );
    }

    #[test]
    fn announces_a_video_that_lowered_the_average() {
        let mut video = video("O que é ciência?");
        video.variation = -2.48;
        video.variation_percent = -0.0862;

        let tweet = compose_tweet(&video, SITE_URL);

        assert_eq!(line(&tweet, 6), "📊 Variação: 📉 −2,48s (−0,09%)");
    }

    #[test]
    fn an_unchanged_average_is_not_shown_as_a_fall() {
        let mut video = video("O que é ciência?");
        video.variation = -0.001;
        video.variation_percent = -0.00007;

        let tweet = compose_tweet(&video, SITE_URL);

        assert_eq!(line(&tweet, 6), "📊 Variação: 📈 0,00s (0,00%)");
    }

    #[test]
    fn counts_length_the_way_x_does() {
        assert_eq!(weighted_length("abc"), 3);
        assert_eq!(weighted_length("Variação"), 8);
        assert_eq!(weighted_length("🚨"), 2);
        assert_eq!(weighted_length("…"), 2);
        assert_eq!(weighted_length("−2,48s"), 7);
        assert_eq!(weighted_length("a\n\nb"), 4);
        assert_eq!(weighted_length(VIDEO_URL), 23);
        assert_eq!(
            weighted_length(&format!("🔗 {VIDEO_URL}\nfim")),
            2 + 1 + 23 + 1 + 3
        );
        assert_eq!(weighted_length(""), 0);
    }

    #[test]
    fn a_word_that_x_may_turn_into_a_link_counts_as_the_word_plus_a_link() {
        assert_eq!(
            weighted_length("leia em g1.com hoje"),
            4 + 1 + 2 + 1 + (6 + 23) + 1 + 4
        );
        assert_eq!(weighted_length("subiu 0.02% hoje"), 16);
        assert_eq!(weighted_length("fim. Outra frase"), 16);
    }

    #[test]
    fn the_longest_title_youtube_allows_fits_whole_in_a_typical_announcement() {
        let title = "A".repeat(100);

        let tweet = compose_tweet(&video(&title), SITE_URL);

        assert!(tweet.contains(&title));
        assert!(weighted_length(&tweet) <= MAX_TWEET_LENGTH);
    }

    #[test]
    fn a_title_that_cannot_fit_is_cut_with_an_ellipsis_and_the_links_are_kept() {
        let tweet = compose_tweet(&video(&"🧬".repeat(100)), SITE_URL);
        let title = line(&tweet, 1);

        assert!(weighted_length(&tweet) <= MAX_TWEET_LENGTH);
        assert!(
            title.starts_with("🎥 🧬🧬") && title.ends_with("🧬…"),
            "{title}"
        );
        assert!(tweet.contains(VIDEO_URL) && tweet.contains(SITE_URL));
        assert!(
            weighted_length(&tweet) > MAX_TWEET_LENGTH - 4,
            "the title should use the room there is"
        );
    }

    #[test]
    fn never_exceeds_the_limit_whatever_the_title_and_the_numbers() {
        let titles = [
            "A".repeat(100),
            "🧬".repeat(100),
            format!("{} g1.com", "A".repeat(93)),
            format!("Veja em pirulla.com.br e em g1.com {}", "é".repeat(65)),
            "a.b ".repeat(25),
            "漢字".repeat(50),
        ];
        for title in titles {
            let mut worst = video(&title);
            worst.video.duration = 86_400 + 23 * 3600 + 59 * 60 + 59;
            worst.mean = 9999.99;
            worst.variation = -5999.99;
            worst.variation_percent = -37.5;

            let tweet = compose_tweet(&worst, SITE_URL);

            assert!(weighted_length(&tweet) <= MAX_TWEET_LENGTH, "{title}");
            assert!(tweet.contains(VIDEO_URL) && tweet.contains(SITE_URL));
        }
    }

    #[test]
    fn the_link_to_post_on_x_carries_the_whole_text() {
        assert_eq!(
            intent_url("linha 1\nlinha 2 & 100% #pirulla 🚨"),
            "https://x.com/intent/post?text=linha%201%0Alinha%202%20%26%20100%25%20%23pirulla%20%F0%9F%9A%A8"
        );
        assert_eq!(
            intent_url("(a) b-c_d.e!~*'"),
            "https://x.com/intent/post?text=(a)%20b-c_d.e!~*'"
        );
    }
}
