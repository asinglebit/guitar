use chrono::{DateTime, FixedOffset, TimeZone, Utc};

pub fn git2_timestamp_to_utc(time: git2::Time) -> String {
    timestamp_to_utc(time.seconds(), time.offset_minutes() * 60)
}

pub fn gix_timestamp_to_utc(time: gix::date::Time) -> String {
    timestamp_to_utc(time.seconds, time.offset)
}

pub fn gix_timestamp_to_utc_date_time(time: gix::date::Time) -> String {
    timestamp_to_utc_date_time(time.seconds, time.offset)
}

fn timestamp_to_utc(seconds: i64, offset_seconds: i32) -> String {
    // Git stores the author's offset separately from epoch seconds.
    let offset = FixedOffset::east_opt(offset_seconds).unwrap();

    // Start with the raw epoch time, then apply the stored offset.
    let utc_datetime = DateTime::from_timestamp(seconds, 0).expect("Invalid timestamp");

    // Normalize to UTC so inspector timestamps use one stable display timezone.
    let local_datetime = offset.from_utc_datetime(&utc_datetime.naive_utc());
    let final_utc: DateTime<Utc> = local_datetime.with_timezone(&Utc);

    final_utc.to_rfc2822()
}

fn timestamp_to_utc_date_time(seconds: i64, offset_seconds: i32) -> String {
    let offset = FixedOffset::east_opt(offset_seconds).unwrap();
    let utc_datetime = DateTime::from_timestamp(seconds, 0).expect("Invalid timestamp");
    let local_datetime = offset.from_utc_datetime(&utc_datetime.naive_utc());
    let final_utc: DateTime<Utc> = local_datetime.with_timezone(&Utc);

    final_utc.format("%Y-%m-%d %H:%M").to_string()
}
