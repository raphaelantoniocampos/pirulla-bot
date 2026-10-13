/// Parses an ISO 8601 duration as returned by the YouTube API
/// (`PT1H2M3S`, `P1DT2H3M4S`, `P0D`) into seconds.
pub fn parse_iso8601_duration(input: &str) -> Option<u64> {
    let rest = input.strip_prefix('P')?;
    let (date, time) = rest.split_once('T').unwrap_or((rest, ""));
    if rest.is_empty() || rest.ends_with('T') {
        return None;
    }
    parse_components(date, &DATE_UNITS)?.checked_add(parse_components(time, &TIME_UNITS)?)
}

const DATE_UNITS: [(char, u64); 2] = [('W', 604_800), ('D', 86_400)];
const TIME_UNITS: [(char, u64); 3] = [('H', 3_600), ('M', 60), ('S', 1)];

/// Sums a sequence of `<number><unit>` pairs, whose units must follow the order of `units`.
fn parse_components(part: &str, units: &[(char, u64)]) -> Option<u64> {
    let mut units = units.iter();
    let mut total = 0u64;
    let mut number: Option<u64> = None;
    for c in part.chars() {
        if let Some(digit) = c.to_digit(10) {
            number = Some(
                number
                    .unwrap_or(0)
                    .checked_mul(10)?
                    .checked_add(digit.into())?,
            );
        } else {
            let (_, seconds) = units.find(|(unit, _)| *unit == c)?;
            total = total.checked_add(number.take()?.checked_mul(*seconds)?)?;
        }
    }
    number.is_none().then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hours_minutes_and_seconds() {
        assert_eq!(parse_iso8601_duration("PT1H2M3S"), Some(3723));
        assert_eq!(parse_iso8601_duration("PT15M33S"), Some(933));
        assert_eq!(parse_iso8601_duration("PT45S"), Some(45));
        assert_eq!(parse_iso8601_duration("PT2H"), Some(7200));
        assert_eq!(parse_iso8601_duration("PT1M"), Some(60));
    }

    #[test]
    fn parses_durations_longer_than_a_day() {
        assert_eq!(parse_iso8601_duration("P1DT2H3M4S"), Some(93784));
        assert_eq!(parse_iso8601_duration("P2D"), Some(172800));
        assert_eq!(parse_iso8601_duration("P1W"), Some(604800));
    }

    #[test]
    fn parses_the_zero_duration_of_live_and_upcoming_videos() {
        assert_eq!(parse_iso8601_duration("P0D"), Some(0));
    }

    #[test]
    fn rejects_malformed_durations() {
        for input in [
            "",
            "abc",
            "P",
            "PT",
            "PT5X",
            "PT1H2M3Sjunk",
            "1H2M",
            "PTS",
            "P1H",
        ] {
            assert_eq!(parse_iso8601_duration(input), None, "input: {input:?}");
        }
    }
}
