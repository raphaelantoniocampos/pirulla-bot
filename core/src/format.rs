//! Text shown to the reader: durations, variations, dates and counts, in Brazilian Portuguese.

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::America::Sao_Paulo;

use crate::data::VideoKind;

pub const MINUS: char = '−';

/// "25min 13s". The fraction of a second is dropped.
pub fn format_duration(seconds: f64) -> String {
    duration(seconds, false)
}

/// "25min 13s 450ms"
pub fn format_duration_ms(seconds: f64) -> String {
    duration(seconds, true)
}

fn duration(seconds: f64, milliseconds: bool) -> String {
    let total = (seconds.abs() * 1000.0).round() as u64;
    let hours = total / 3_600_000;
    let minutes = total % 3_600_000 / 60_000;
    let whole_seconds = total % 60_000 / 1000;

    let text = if hours > 0 {
        format!("{hours}h {minutes:02}min {whole_seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}min {whole_seconds:02}s")
    } else {
        format!("{whole_seconds}s")
    };
    if milliseconds {
        format!("{text} {:03}ms", total % 1000)
    } else {
        text
    }
}

/// The shortest readable form, for axis ticks: "45s", "25min", "1h30".
pub fn format_duration_short(seconds: f64) -> String {
    if seconds == 0.0 {
        return "0".to_string();
    }
    let hours = (seconds / 3600.0).floor() as u64;
    let minutes = (seconds % 3600.0 / 60.0).floor() as u64;
    let rest = (seconds % 60.0 * 10.0).round() / 10.0;

    if hours > 0 {
        return if minutes > 0 {
            format!("{hours}h{minutes:02}")
        } else {
            format!("{hours}h")
        };
    }
    let rest = if rest > 0.0 {
        format!("{}s", rest.to_string().replace('.', ","))
    } else {
        String::new()
    };
    match (minutes > 0, rest.is_empty()) {
        (true, true) => format!("{minutes}min"),
        (true, false) => format!("{minutes}min {rest}"),
        (false, _) => rest,
    }
}

fn decimal(number: f64) -> String {
    format!("{number:.2}").replace('.', ",")
}

fn sign(value: f64, is_zero: bool) -> &'static str {
    if is_zero {
        ""
    } else if value > 0.0 {
        "+"
    } else {
        "−"
    }
}

/// A change in a duration, with its sign: "+0,30s", "−1min 05s".
pub fn format_variation(seconds: f64) -> String {
    let size = (seconds.abs() * 100.0).round() / 100.0;
    let text = if size < 60.0 {
        format!("{}s", decimal(size))
    } else {
        format_duration(size.round())
    };
    format!("{}{text}", sign(seconds, size == 0.0))
}

/// "+0,02%"
pub fn format_percent(percent: f64) -> String {
    let size = percent.abs();
    format!("{}{}%", sign(percent, size < 0.005), decimal(size))
}

/// "31/12/2023", in the time of São Paulo.
pub fn format_date(time: DateTime<Utc>) -> String {
    let local = time.with_timezone(&Sao_Paulo);
    format!("{:02}/{:02}/{}", local.day(), local.month(), local.year())
}

/// "13/10/2026 10:20", in the time of São Paulo.
pub fn format_date_time(time: DateTime<Utc>) -> String {
    let local = time.with_timezone(&Sao_Paulo);
    format!(
        "{} {:02}:{:02}",
        format_date(time),
        local.hour(),
        local.minute()
    )
}

/// "1.523"
pub fn format_count(number: u64) -> String {
    let digits = number.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push('.');
        }
        text.push(digit);
    }
    text
}

/// A total of many durations: "37h 12min", or "26 dias 18h" from two days on.
pub fn format_total(seconds: u64) -> String {
    let hours = seconds / 3600;
    if hours < 48 {
        format!("{hours}h {:02}min", seconds % 3600 / 60)
    } else {
        format!("{} dias {}h", format_count(hours / 24), hours % 24)
    }
}

/// What a video is, when it is not a regular one: "live" or "short".
pub fn format_kind(kind: VideoKind) -> &'static str {
    match kind {
        VideoKind::Video => "",
        VideoKind::Live => "live",
        VideoKind::Short => "short",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(time: &str) -> DateTime<Utc> {
        time.parse().unwrap()
    }

    #[test]
    fn a_duration_shows_only_the_units_it_needs() {
        assert_eq!(format_duration(1513.45), "25min 13s");
        assert_eq!(format_duration(3723.0), "1h 02min 03s");
        assert_eq!(format_duration(3600.0), "1h 00min 00s");
        assert_eq!(format_duration(600.0), "10min 00s");
        assert_eq!(format_duration(45.0), "45s");
        assert_eq!(format_duration(0.0), "0s");
    }

    #[test]
    fn a_duration_drops_the_fraction_of_a_second_to_agree_with_the_form_that_shows_the_milliseconds()
     {
        assert_eq!(format_duration(1472.9), "24min 32s");
        assert_eq!(format_duration_ms(1472.9), "24min 32s 900ms");
        assert_eq!(format_duration(59.6), "59s");
        assert_eq!(format_duration(3599.5), "59min 59s");
    }

    #[test]
    fn a_duration_can_show_its_milliseconds() {
        assert_eq!(format_duration_ms(1513.45), "25min 13s 450ms");
        assert_eq!(format_duration_ms(1513.29), "25min 13s 290ms");
        assert_eq!(format_duration_ms(45.0), "45s 000ms");
    }

    #[test]
    fn the_short_form_of_a_duration_drops_what_is_zero() {
        assert_eq!(format_duration_short(0.0), "0");
        assert_eq!(format_duration_short(30.0), "30s");
        assert_eq!(format_duration_short(60.0), "1min");
        assert_eq!(format_duration_short(90.0), "1min 30s");
        assert_eq!(format_duration_short(1500.0), "25min");
        assert_eq!(format_duration_short(3600.0), "1h");
        assert_eq!(format_duration_short(5400.0), "1h30");
        assert_eq!(format_duration_short(1513.5), "25min 13,5s");
        assert_eq!(format_duration_short(0.5), "0,5s");
    }

    #[test]
    fn a_variation_carries_its_sign() {
        assert_eq!(format_variation(0.3), "+0,30s");
        assert_eq!(format_variation(-0.3), "−0,30s");
        assert_eq!(format_variation(150.0), "+2min 30s");
        assert_eq!(format_variation(-65.2), "−1min 05s");
        assert_eq!(format_variation(59.999), "+1min 00s");
    }

    #[test]
    fn a_variation_too_small_to_show_has_no_sign() {
        assert_eq!(format_variation(0.0), "0,00s");
        assert_eq!(format_variation(-0.001), "0,00s");
    }

    #[test]
    fn a_percentage_carries_its_sign() {
        assert_eq!(format_percent(0.0198), "+0,02%");
        assert_eq!(format_percent(-0.0198), "−0,02%");
        assert_eq!(format_percent(46.666), "+46,67%");
        assert_eq!(format_percent(0.0), "0,00%");
        assert_eq!(format_percent(-0.001), "0,00%");
    }

    #[test]
    fn dates_are_shown_in_the_time_of_sao_paulo() {
        let already_new_year_in_utc = at("2024-01-01T02:30:00Z");

        assert_eq!(format_date(already_new_year_in_utc), "31/12/2023");
        assert_eq!(
            format_date_time(at("2026-10-13T13:20:00Z")),
            "13/10/2026 10:20"
        );
    }

    #[test]
    fn dates_follow_the_daylight_saving_time_sao_paulo_had() {
        // Midnight of the 1st of January of 2015 in São Paulo, which was on summer time (UTC-2) then.
        assert_eq!(
            format_date_time(at("2015-01-01T02:00:00Z")),
            "01/01/2015 00:00"
        );
        assert_eq!(
            format_date_time(at("2015-07-01T03:00:00Z")),
            "01/07/2015 00:00"
        );
    }

    #[test]
    fn counts_use_the_brazilian_thousands_separator() {
        assert_eq!(format_count(1523), "1.523");
        assert_eq!(format_count(42), "42");
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(1_234_567), "1.234.567");
        assert_eq!(format_count(100_000), "100.000");
    }

    #[test]
    fn a_long_total_is_shown_in_days() {
        assert_eq!(format_total(37 * 3600 + 12 * 60), "37h 12min");
        assert_eq!(format_total(26 * 86400 + 18 * 3600 + 1200), "26 dias 18h");
    }

    #[test]
    fn lives_and_shorts_are_named_and_regular_videos_need_no_name() {
        assert_eq!(format_kind(VideoKind::Live), "live");
        assert_eq!(format_kind(VideoKind::Short), "short");
        assert_eq!(format_kind(VideoKind::Video), "");
    }
}
