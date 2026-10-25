use chrono::{Duration, NaiveDate, TimeZone};
use chrono_tz::Tz;

pub fn parse(day: &str) -> Result<NaiveDate, String> {
    let date =
        NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| "Use a valid YYYY-MM-DD date")?;
    if date.format("%Y-%m-%d").to_string() != day {
        return Err("Use a valid YYYY-MM-DD date".into());
    }
    Ok(date)
}

/// Resolve both civil boundaries independently: DST days are not always 24h.
pub fn bounds(day: &str, zone: &str) -> Result<(i64, i64), String> {
    let date = parse(day)?;
    let tz: Tz = zone.parse().map_err(|_| "Unknown IANA time zone")?;
    let midnight = |date: NaiveDate| {
        (0..180)
            .find_map(|minute| {
                tz.from_local_datetime(&(date.and_hms_opt(0, 0, 0)? + Duration::minutes(minute)))
                    .earliest()
            })
            .map(|time| time.timestamp_millis())
            .ok_or_else(|| "This civil day does not exist in the selected time zone".to_string())
    };
    Ok((
        midnight(date)?,
        midnight(date.succ_opt().ok_or("Date out of range")?)?,
    ))
}
