use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use git2::Time;

pub fn timestamp_to_utc(time: Time) -> String {
    // Create a DateTime with the given offset
    let offset = FixedOffset::east_opt(time.offset_minutes() * 60).unwrap();

    // Create UTC datetime from timestamp
    let utc_datetime = DateTime::from_timestamp(time.seconds(), 0).expect("Invalid timestamp");

    // Convert to local time with offset, then back to UTC
    let local_datetime = offset.from_utc_datetime(&utc_datetime.naive_utc());
    let final_utc: DateTime<Utc> = local_datetime.with_timezone(&Utc);

    // Format as string
    final_utc.to_rfc2822()
}
