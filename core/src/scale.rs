//! Scales and axis ticks for the charts. Times are in milliseconds since 1970.

use chrono::{DateTime, Datelike, TimeZone, Utc};

use crate::stats::YEAR_MS;

const MONTHS: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];
/// Ticks are placed at midnight in São Paulo, taken as UTC-3: an hour of error does not show on an axis.
const UTC_OFFSET_HOURS: u32 = 3;
const DURATION_STEPS: [f64; 21] = [
    0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0,
    3600.0, 7200.0, 10_800.0, 21_600.0, 43_200.0, 86_400.0,
];
/// Leaves room for the error of dividing by a step that is not a whole number.
const EPSILON: f64 = 1e-9;

/// Maps a domain onto a range, and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    domain: (f64, f64),
    range: (f64, f64),
}

impl Linear {
    pub fn new(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self { domain, range }
    }

    /// Where `value` goes; the middle of the range when the domain is a single value.
    pub fn at(&self, value: f64) -> f64 {
        let ((domain_start, domain_end), (range_start, range_end)) = (self.domain, self.range);
        let domain_span = domain_end - domain_start;
        if domain_span == 0.0 {
            return (range_start + range_end) / 2.0;
        }
        range_start + (value - domain_start) / domain_span * (range_end - range_start)
    }

    /// The value that goes at `position`.
    pub fn invert(&self, position: f64) -> f64 {
        let ((domain_start, domain_end), (range_start, range_end)) = (self.domain, self.range);
        let range_span = range_end - range_start;
        if range_span == 0.0 {
            return domain_start;
        }
        domain_start + (position - range_start) / range_span * (domain_end - domain_start)
    }
}

/// At most `max_ticks` durations between `min` and `max` seconds, at the finest round step that fits.
pub fn duration_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let first = |step: f64| (min / step - EPSILON).ceil();
    let last = |step: f64| (max / step + EPSILON).floor();
    let fits = |step: &f64| last(*step) - first(*step) + 1.0 <= max_ticks as f64;
    let step = DURATION_STEPS
        .iter()
        .copied()
        .find(fits)
        .unwrap_or_else(|| 86_400.0 * ((max - min) / 86_400.0 / max_ticks as f64).ceil());
    let (first, last) = (first(step) as i64, last(step) as i64);
    (first..=last)
        .map(|index| (index as f64 * step * 10.0).round() / 10.0)
        .collect()
}

/// A labelled moment on a time axis.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeTick {
    pub time: f64,
    pub label: String,
}

/// At most `max_ticks` moments between `start` and `end`: the start of years, or of months when the span is short.
pub fn time_ticks(start: f64, end: f64, max_ticks: usize) -> Vec<TimeTick> {
    let yearly = end - start >= 2.0 * YEAR_MS;
    let year_of = |time: f64| {
        DateTime::<Utc>::from_timestamp_millis(time as i64).map_or(1970, |time| time.year())
    };
    let (first_year, last_year) = (i64::from(year_of(start)), i64::from(year_of(end)));
    // Months are counted from year zero, so that steps fall on the same months every year.
    let (first, last) = if yearly {
        (first_year, last_year)
    } else {
        (first_year * 12, last_year * 12 + 11)
    };
    let steps: &[i64] = if yearly {
        &[1, 2, 5, 10, 20, 50, 100]
    } else {
        &[1, 2, 3, 6]
    };

    for &step in steps {
        let mut ticks = Vec::new();
        let mut index = first.div_euclid(step) * step;
        if index < first {
            index += step;
        }
        while index <= last {
            let (year, month) = if yearly {
                (index, 0)
            } else {
                (index.div_euclid(12), index.rem_euclid(12))
            };
            let time = Utc
                .with_ymd_and_hms(year as i32, month as u32 + 1, 1, UTC_OFFSET_HOURS, 0, 0)
                .single()
                .map_or(f64::NAN, |time| time.timestamp_millis() as f64);
            let label = if yearly {
                year.to_string()
            } else {
                format!("{}/{:02}", MONTHS[month as usize], year.rem_euclid(100))
            };
            if time >= start && time <= end {
                ticks.push(TimeTick { time, label });
            }
            index += step;
        }
        if ticks.len() <= max_ticks {
            return ticks;
        }
    }
    Vec::new()
}

/// The index of the value in the sorted `values` that is nearest to `target`.
pub fn nearest_index(values: &[f64], target: f64) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    let low = values
        .partition_point(|value| *value < target)
        .min(values.len() - 1);
    match low.checked_sub(1) {
        Some(previous) if target - values[previous] <= values[low] - target => Some(previous),
        _ => Some(low),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(time: &str) -> f64 {
        time.parse::<DateTime<Utc>>().unwrap().timestamp_millis() as f64
    }

    fn labels(ticks: &[TimeTick]) -> Vec<&str> {
        ticks.iter().map(|tick| tick.label.as_str()).collect()
    }

    #[test]
    fn a_linear_scale_maps_a_domain_onto_a_range_and_back() {
        let scale = Linear::new((0.0, 100.0), (20.0, 520.0));

        assert_eq!(scale.at(0.0), 20.0);
        assert_eq!(scale.at(50.0), 270.0);
        assert_eq!(scale.invert(270.0), 50.0);
    }

    #[test]
    fn a_linear_scale_can_be_inverted_as_the_y_axis_of_a_chart_is() {
        assert_eq!(Linear::new((0.0, 100.0), (300.0, 0.0)).at(25.0), 225.0);
    }

    #[test]
    fn a_scale_over_a_single_value_puts_it_in_the_middle() {
        assert_eq!(Linear::new((10.0, 10.0), (0.0, 100.0)).at(10.0), 50.0);
    }

    #[test]
    fn duration_ticks_fall_on_round_steps() {
        assert_eq!(
            duration_ticks(0.0, 3600.0, 6),
            [0.0, 900.0, 1800.0, 2700.0, 3600.0]
        );
        assert_eq!(
            duration_ticks(1500.0, 1530.0, 6),
            [1500.0, 1510.0, 1520.0, 1530.0]
        );
        assert_eq!(
            duration_ticks(0.0, 14000.0, 6),
            [0.0, 3600.0, 7200.0, 10800.0]
        );
    }

    #[test]
    fn duration_ticks_can_be_fractions_of_a_second() {
        assert_eq!(duration_ticks(1512.3, 1513.9, 5), [1512.5, 1513.0, 1513.5]);
    }

    #[test]
    fn duration_ticks_take_whole_days_beyond_the_round_steps() {
        assert_eq!(
            duration_ticks(0.0, 10.0 * 86_400.0, 3),
            [0.0, 345_600.0, 691_200.0]
        );
    }

    #[test]
    fn time_ticks_fall_on_round_years_when_the_span_is_long() {
        let (start, end) = (
            millis("2006-11-05T12:00:00Z"),
            millis("2026-10-13T12:00:00Z"),
        );

        assert_eq!(
            labels(&time_ticks(start, end, 8)),
            ["2010", "2015", "2020", "2025"]
        );
        assert_eq!(
            labels(&time_ticks(start, end, 12)),
            [
                "2008", "2010", "2012", "2014", "2016", "2018", "2020", "2022", "2024", "2026"
            ]
        );
    }

    #[test]
    fn time_ticks_fall_on_months_when_the_span_is_short() {
        let ticks = time_ticks(
            millis("2025-10-13T12:00:00Z"),
            millis("2026-10-13T12:00:00Z"),
            6,
        );

        assert_eq!(
            labels(&ticks),
            ["nov/25", "jan/26", "mar/26", "mai/26", "jul/26", "set/26"]
        );
    }

    #[test]
    fn time_ticks_are_inside_the_span_in_order() {
        let (start, end) = (
            millis("2006-11-05T12:00:00Z"),
            millis("2026-10-13T12:00:00Z"),
        );

        let times: Vec<f64> = time_ticks(start, end, 8)
            .iter()
            .map(|tick| tick.time)
            .collect();

        assert!(times.iter().all(|time| *time >= start && *time <= end));
        assert!(times.is_sorted());
        assert_eq!(times[0], millis("2010-01-01T03:00:00Z"));
    }

    #[test]
    fn finds_the_value_nearest_to_a_position() {
        let values = [0.0, 10.0, 20.0, 30.0];

        assert_eq!(nearest_index(&values, 14.0), Some(1));
        assert_eq!(nearest_index(&values, 16.0), Some(2));
        assert_eq!(nearest_index(&values, 15.0), Some(1));
        assert_eq!(nearest_index(&values, -5.0), Some(0));
        assert_eq!(nearest_index(&values, 99.0), Some(3));
        assert_eq!(nearest_index(&[], 5.0), None);
    }
}
