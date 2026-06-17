use chrono::{DateTime, Datelike, FixedOffset, Months};

use crate::exports::wasco_dev::datetime::datetime::{Guest, OffsetSize, Timestamp};

wit_bindgen::generate!({ generate_all });

impl std::fmt::Display for OffsetSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size_str = match self {
            OffsetSize::Seconds => "second",
            OffsetSize::Minutes => "minute",
            OffsetSize::Hours => "hour",
            OffsetSize::Days => "day",
            OffsetSize::Weeks => "week",
            OffsetSize::Months => "month",
            OffsetSize::Years => "year",
        };
        write!(f, "{size_str}")
    }
}

fn pluralize(str: String, count: i32) -> String {
    if count.abs() != 1 { str + "s" } else { str }
}

/**
TimeDelta only supports offset sizes lower than months.
This is because weeks and below can be calculated to a specific amount of nanoseconds,
while months do not always contain the same amount of nanoseconds (february contains less than january, for example).
Therefore, this is handled separately by the TimeOffset.
*/
enum TimeOffset {
    TimeDelta(chrono::TimeDelta),
    Months(i32),
}

impl TimeOffset {
    fn new(offset_count: i32, offset_size: OffsetSize) -> Self {
        match offset_size {
            OffsetSize::Seconds => Self::TimeDelta(chrono::TimeDelta::seconds(offset_count as i64)),
            OffsetSize::Minutes => Self::TimeDelta(chrono::TimeDelta::minutes(offset_count as i64)),
            OffsetSize::Hours => Self::TimeDelta(chrono::TimeDelta::hours(offset_count as i64)),
            OffsetSize::Days => Self::TimeDelta(chrono::TimeDelta::days(offset_count as i64)),
            OffsetSize::Weeks => Self::TimeDelta(chrono::TimeDelta::weeks(offset_count as i64)),
            OffsetSize::Months => Self::Months(offset_count),
            OffsetSize::Years => Self::Months(offset_count * 12),
        }
    }

    fn add_months(
        timestamp: chrono::DateTime<FixedOffset>,
        offset_count: i32,
    ) -> Result<chrono::DateTime<FixedOffset>, String> {
        if offset_count < 0 {
            timestamp
                .checked_sub_months(Months::new(offset_count.unsigned_abs()))
                .ok_or_else(|| String::from("Could not offset the datetime"))
        } else {
            timestamp
                .checked_add_months(Months::new(offset_count as u32))
                .ok_or_else(|| String::from("Could not offset the datetime"))
        }
    }

    fn offset_time(
        &self,
        timestamp: chrono::DateTime<FixedOffset>,
    ) -> Result<chrono::DateTime<FixedOffset>, String> {
        match self {
            Self::TimeDelta(time_delta) => timestamp
                .checked_add_signed(*time_delta)
                .ok_or_else(|| String::from("Could not offset the datetime")),
            Self::Months(months) => Self::add_months(timestamp, *months),
        }
    }

    fn offset_time_in_business_days(
        &self,
        timestamp: chrono::DateTime<FixedOffset>,
        offset_size: OffsetSize,
    ) -> Result<DateTime<FixedOffset>, String> {
        if let TimeOffset::TimeDelta(time_delta) = self
            && offset_size < OffsetSize::Weeks
        {
            // If you're only counting business days every week is 5 days instead of 7, so we add 2 days for every 5 in the offset we started with.
            let amount_of_days = time_delta.num_days();
            let extra_days = amount_of_days / 5 * 2;
            let offset_timestamp = timestamp
                .checked_add_signed(*time_delta + chrono::TimeDelta::days(extra_days))
                .ok_or_else(|| String::from("Could not offset the datetime"))?;

            let offset_timestamp_weekday = offset_timestamp.weekday().num_days_from_monday();

            // If the day of the week comes before the original day of the week, that means we passed a weekend we didn't yet account for.
            // If the day of the week ends up on saturday or sunday (5 or 6) we are currently in a weekend we didn't yet account for.
            if offset_timestamp_weekday < timestamp.weekday().num_days_from_monday()
                || offset_timestamp_weekday > 4
            {
                offset_timestamp
                    .checked_add_days(chrono::Days::new(2))
                    .ok_or_else(|| String::from("Could not offset the datetime"))
            } else {
                Ok(offset_timestamp)
            }
        } else {
            self.offset_time(timestamp)
        }
    }
}

struct Component;

impl Guest for Component {
    fn now() -> Timestamp {
        chrono::Utc::now().to_rfc3339()
    }

    fn change_timezone(timestamp: Timestamp, timezone: String) -> Result<Timestamp, String> {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|_| format!("The timestamp \"{timestamp}\" is not correctly formatted"))?;
        let timezone: FixedOffset = timezone
            .parse()
            .map_err(|_| format!("The timezone \"{timezone}\" is not correctly formatted"))?;

        Ok(timestamp.with_timezone(&timezone).to_rfc3339())
    }

    fn offset_datetime(
        timestamp: Timestamp,
        offset_count: i32,
        offset_size: OffsetSize,
    ) -> Result<Timestamp, String> {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|_| format!("The timestamp \"{timestamp}\" is not correctly formatted"))?;

        let time_offset = TimeOffset::new(offset_count, offset_size);

        match time_offset.offset_time(timestamp) {
            Ok(offset_timestamp) => Ok(offset_timestamp.to_rfc3339()),
            Err(error_message) => Err(format!(
                "{error_message} by {offset_count} {}",
                pluralize(offset_size.to_string(), offset_count)
            )),
        }
    }

    fn offset_datetime_in_business_days(
        timestamp: Timestamp,
        offset_count: i32,
        offset_size: OffsetSize,
    ) -> Result<Timestamp, String> {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|_| format!("The timestamp \"{timestamp}\" is not correctly formatted"))?;

        let time_offset = TimeOffset::new(offset_count, offset_size);

        match time_offset.offset_time_in_business_days(timestamp, offset_size) {
            Ok(offset_timestamp) => Ok(offset_timestamp.to_rfc3339()),
            Err(error_message) => Err(format!(
                "{error_message} by {offset_count} {}",
                pluralize(offset_size.to_string(), offset_count)
            )),
        }
    }
}

export! {Component}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;

    #[test]
    fn change_timezone_with_invalid_timestamp_test() {
        assert_eq!(
            Component::change_timezone(String::from("invalid"), String::from("Z"))
                .unwrap_err()
                .as_str(),
            "The timestamp \"invalid\" is not correctly formatted"
        );
    }

    #[test]
    fn change_timezone_with_invalid_timezone_test() {
        assert_eq!(
            Component::change_timezone(
                String::from("1970-01-02T00:00:00+00:00"),
                String::from("invalid")
            )
            .unwrap_err()
            .as_str(),
            "The timezone \"invalid\" is not correctly formatted"
        );
    }

    #[test]
    fn change_timezone_validity_test() {
        let now = chrono::Utc::now().fixed_offset();
        let now_in_another_timezone = chrono::DateTime::parse_from_rfc3339(
            &Component::change_timezone(now.to_rfc3339(), String::from("+01:00")).unwrap(),
        )
        .unwrap();

        // The timezone is 3600 seconds offset from UTC.
        assert_eq!(now_in_another_timezone.timezone().local_minus_utc(), 3600);

        // The time is still the same when converted back to UTC.
        assert_eq!(now.naive_utc(), now_in_another_timezone.naive_utc());
    }

    #[test]
    fn offset_datetime_large_timedelta_validity_test() {
        // Offset the UNIX EPOCH by 3 days, 3 hours, 3 minutes and 3 seconds exactly
        let offset_datetime = DateTime::parse_from_rfc3339(
            &Component::offset_datetime(
                String::from("1970-01-01T00:00:00+00:00"),
                270183,
                OffsetSize::Seconds,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(offset_datetime.day(), 4);
        assert_eq!(offset_datetime.hour(), 3);
        assert_eq!(offset_datetime.minute(), 3);
        assert_eq!(offset_datetime.second(), 3);
    }

    #[test]
    fn offset_datetime_in_business_days_weekend_skip_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-02T00:00:00+00:00"),
                1,
                OffsetSize::Days
            )
            .unwrap()
            .as_str(),
            "1970-01-05T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_datetime_in_business_days_weekend_skip_with_hour_offset_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-02T06:00:00+00:00"),
                18,
                OffsetSize::Hours
            )
            .unwrap()
            .as_str(),
            "1970-01-05T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_datetime_in_business_days_multiple_weekend_skip_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-02T00:00:00+00:00"),
                14,
                OffsetSize::Days
            )
            .unwrap()
            .as_str(),
            "1970-01-22T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_datetime_in_business_days_starting_in_weekend_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-03T00:00:00+00:00"),
                1,
                OffsetSize::Days
            )
            .unwrap()
            .as_str(),
            "1970-01-06T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_datetime_in_business_days_by_weeks_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-01T00:00:00+00:00"),
                2,
                OffsetSize::Weeks
            )
            .unwrap()
            .as_str(),
            "1970-01-15T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_datetime_in_business_days_by_months_test() {
        assert_eq!(
            Component::offset_datetime_in_business_days(
                String::from("1970-01-01T00:00:00+00:00"),
                2,
                OffsetSize::Months
            )
            .unwrap()
            .as_str(),
            "1970-03-01T00:00:00+00:00"
        );
    }

    #[test]
    fn offset_minimum_datatime_errors_test() {
        assert_eq!(
            TimeOffset::new(-1, OffsetSize::Seconds)
                .offset_time(DateTime::<FixedOffset>::MIN_UTC.fixed_offset())
                .unwrap_err()
                .as_str(),
            "Could not offset the datetime"
        )
    }

    #[test]
    fn offset_to_minimum_datatime_works_test() {
        assert_eq!(
            TimeOffset::new(-1, OffsetSize::Seconds)
                .offset_time(
                    DateTime::<FixedOffset>::MIN_UTC.fixed_offset() + chrono::TimeDelta::seconds(1)
                )
                .unwrap()
                .to_rfc3339()
                .as_str(),
            "-262143-01-01T00:00:00+00:00"
        )
    }

    #[test]
    fn offset_to_maximum_datatime_works_test() {
        assert_eq!(
            TimeOffset::new(1, OffsetSize::Seconds)
                .offset_time(
                    DateTime::<FixedOffset>::MAX_UTC.fixed_offset() - chrono::TimeDelta::seconds(1)
                )
                .unwrap()
                .to_rfc3339()
                .as_str(),
            "+262142-12-31T23:59:59.999999999+00:00"
        )
    }

    #[test]
    fn offset_maximum_datatime_errors_test() {
        assert_eq!(
            TimeOffset::new(1, OffsetSize::Seconds)
                .offset_time(DateTime::<FixedOffset>::MAX_UTC.fixed_offset())
                .unwrap_err()
                .as_str(),
            "Could not offset the datetime"
        )
    }

    /// Proptests have to be run as unit tests, because integration tests on cdylib crates aren't able to directly interact with the crate.
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn change_timezone(tz in "[+-]([01][0-9]|2[0-3]):([0-5][0-9])", dt in "(((2000|2400|2800|((19|2[0-9])(0[48]|[2468][048]|[13579][26])))-02-29)|(((19|2[0-9])[0-9]{2})-02-(0[1-9]|1[0-9]|2[0-8]))|(((19|2[0-9])[0-9]{2})-(0[13578]|10|12)-(0[1-9]|[12][0-9]|3[01]))|(((19|2[0-9])[0-9]{2})-(0[469]|11)-(0[1-9]|[12][0-9]|30)))T([01][0-9]|[2][0-3]):[0-5][0-9]:[0-5][0-9]([+-]([01][0-9]|2[0-3]):([0-5][0-9])|Z)") {
                prop_assert!(Component::change_timezone(dt, tz).is_ok());
            }

            #[test]
            fn offset_datetime(oc in -1000000..1000000, os in 0..6, dt in "(((2000|2400|2800|((19|2[0-9])(0[48]|[2468][048]|[13579][26])))-02-29)|(((19|2[0-9])[0-9]{2})-02-(0[1-9]|1[0-9]|2[0-8]))|(((19|2[0-9])[0-9]{2})-(0[13578]|10|12)-(0[1-9]|[12][0-9]|3[01]))|(((19|2[0-9])[0-9]{2})-(0[469]|11)-(0[1-9]|[12][0-9]|30)))T([01][0-9]|[2][0-3]):[0-5][0-9]:[0-5][0-9]([+-]([01][0-9]|2[0-3]):([0-5][0-9])|Z)") {
                let os = match os {
                    0 => OffsetSize::Days,
                    1 => OffsetSize::Hours,
                    2 => OffsetSize::Minutes,
                    3 => OffsetSize::Seconds,
                    4 => OffsetSize::Weeks,
                    5 => OffsetSize::Months,
                    _ => OffsetSize::Years,
                };
                prop_assert!(Component::offset_datetime(dt, oc, os).is_ok());
            }

            #[test]
            fn offset_datetime_in_business_days(oc in -1000000..1000000, os in 0..6, dt in "(((2000|2400|2800|((19|2[0-9])(0[48]|[2468][048]|[13579][26])))-02-29)|(((19|2[0-9])[0-9]{2})-02-(0[1-9]|1[0-9]|2[0-8]))|(((19|2[0-9])[0-9]{2})-(0[13578]|10|12)-(0[1-9]|[12][0-9]|3[01]))|(((19|2[0-9])[0-9]{2})-(0[469]|11)-(0[1-9]|[12][0-9]|30)))T([01][0-9]|[2][0-3]):[0-5][0-9]:[0-5][0-9]([+-]([01][0-9]|2[0-3]):([0-5][0-9])|Z)") {
                let os = match os {
                    0 => OffsetSize::Days,
                    1 => OffsetSize::Hours,
                    2 => OffsetSize::Minutes,
                    3 => OffsetSize::Seconds,
                    4 => OffsetSize::Weeks,
                    5 => OffsetSize::Months,
                    _ => OffsetSize::Years,
                };
                prop_assert!(Component::offset_datetime_in_business_days(dt, oc, os).is_ok());
            }
        }
    }
}
