//! When an event was emitted, and how it displays.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// When an event was emitted: an instant, displayed in UTC in ISO 8601.
///
/// It displays to the nanosecond, without trailing zeros, for example
/// `2023-11-14T22:13:20.123Z`. A year outside 0 to 9999 is written as an ISO 8601 expanded year,
/// with its sign and at least five digits, for example `+10000-01-01T00:00:00Z`.
///
/// # Examples
///
/// ```
/// use std::time::{Duration, SystemTime, UNIX_EPOCH};
///
/// use itinera::event::Timestamp;
///
/// let timestamp = Timestamp::from(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123));
/// assert_eq!(timestamp.to_string(), "2023-11-14T22:13:20.123Z");
/// let instant: SystemTime = timestamp.into();
/// assert_eq!(instant, UNIX_EPOCH + Duration::from_millis(1_700_000_000_123));
/// ```
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::From, derive_more::Into,
)]
pub struct Timestamp {
    instant: SystemTime,
}
const SECONDS_PER_DAY: i128 = 86_400;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (seconds, nanos) = seconds_since_epoch(self.instant);
        Utc { seconds, nanos }.fmt(f)
    }
}
struct Utc {
    seconds: i128,
    nanos: u32,
}
impl fmt::Display for Utc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { seconds, nanos } = *self;
        let (year, month, day) = civil_date(seconds.div_euclid(SECONDS_PER_DAY));
        if (0..=9999).contains(&year) {
            write!(f, "{year:04}")?;
        } else {
            write!(f, "{year:+06}")?;
        }
        let of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        write!(
            f,
            "-{month:02}-{day:02}T{:02}:{:02}:{:02}",
            of_day / 3600,
            of_day % 3600 / 60,
            of_day % 60
        )?;
        if nanos > 0 {
            let (mut fraction, mut digits) = (nanos, 9);
            while fraction % 10 == 0 {
                fraction /= 10;
                digits -= 1;
            }
            write!(f, ".{fraction:0digits$}")?;
        }
        f.write_str("Z")
    }
}
fn seconds_since_epoch(timestamp: SystemTime) -> (i128, u32) {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(after) => (i128::from(after.as_secs()), after.subsec_nanos()),
        Err(before) => {
            let before = before.duration();
            let seconds = -i128::from(before.as_secs());
            match before.subsec_nanos() {
                0 => (seconds, 0),
                nanos => (seconds - 1, NANOS_PER_SECOND - nanos),
            }
        }
    }
}
// Howard Hinnant's days-to-civil algorithm, for the proleptic Gregorian calendar.
fn civil_date(days_since_epoch: i128) -> (i128, i128, i128) {
    let days = days_since_epoch + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i128::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rstest::rstest;

    use super::*;

    fn at(seconds: i128, nanos: u32) -> String {
        Utc { seconds, nanos }.to_string()
    }

    #[rstest]
    #[case::the_epoch(0, 0, "1970-01-01T00:00:00Z")]
    #[case::the_day_after_a_leap_day_in_a_century(951_868_800, 0, "2000-03-01T00:00:00Z")]
    #[case::a_leap_day(1_709_208_000, 0, "2024-02-29T12:00:00Z")]
    #[case::milliseconds(1_700_000_000, 123_000_000, "2023-11-14T22:13:20.123Z")]
    #[case::nanoseconds(1_700_000_000, 5, "2023-11-14T22:13:20.000000005Z")]
    #[case::the_last_second_of_year_9999(253_402_300_799, 0, "9999-12-31T23:59:59Z")]
    #[case::the_first_second_of_year_0(-62_167_219_200, 0, "0000-01-01T00:00:00Z")]
    #[case::a_day_before_the_epoch(-86_400, 0, "1969-12-31T00:00:00Z")]
    fn a_timestamp_displays_in_utc_in_iso_8601(
        #[case] seconds: i128,
        #[case] nanos: u32,
        #[case] written: &str,
    ) {
        assert_eq!(at(seconds, nanos), written);
    }

    #[test]
    fn a_timestamp_just_before_1970_borrows_from_the_previous_second() {
        let just_before = UNIX_EPOCH - Duration::from_nanos(100);
        assert_eq!(seconds_since_epoch(just_before), (-1, 999_999_900));
        assert_eq!(
            Timestamp::from(just_before).to_string(),
            "1969-12-31T23:59:59.9999999Z"
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::from_secs(1)).to_string(),
            "1969-12-31T23:59:59Z"
        );
    }

    #[rstest]
    #[case::year_10000(253_402_300_800, "+10000-01-01T00:00:00Z")]
    #[case::year_minus_1(-62_167_219_201, "-00001-12-31T23:59:59Z")]
    fn a_year_outside_0_to_9999_is_written_as_an_expanded_year(
        #[case] seconds: i128,
        #[case] written: &str,
    ) {
        assert_eq!(at(seconds, 0), written);
    }
}
