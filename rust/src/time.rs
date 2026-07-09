use chrono::DateTime;

const GPS_UNIX_OFFSET_SECONDS_BASE: i64 = 315964800;
const GPS_UNIX_OFFSET_SECONDS_LEAP: i64 = 18;

pub fn tdf_time_to_unix(tdf_time: i64) -> (i64, u32) {
    let unix_seconds =
        (tdf_time >> 16) + GPS_UNIX_OFFSET_SECONDS_BASE - GPS_UNIX_OFFSET_SECONDS_LEAP;
    let unix_nano = (1_000_000_000 * (tdf_time & 0xFFFF)) / 65536;

    (unix_seconds, unix_nano as u32)
}

pub fn tdf_time_to_unix_micros(tdf_time: i64) -> i64 {
    let (unix_seconds, unix_nano) = tdf_time_to_unix(tdf_time);
    (unix_seconds * 1_000_000) + (unix_nano as i64 / 1_000)
}

pub fn tdf_time_to_datetime(tdf_time: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    let (unix_seconds, unix_nano) = tdf_time_to_unix(tdf_time);
    DateTime::from_timestamp(unix_seconds, unix_nano)
}
