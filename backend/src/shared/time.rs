use chrono::{DateTime, SecondsFormat, Utc};

/// ミリ秒精度・Z 終端の ISO 8601 文字列（JS の `Date#toISOString` と同形式）。
/// DB でも同じ形式で保存するため、文字列比較が時刻順と一致する。
pub fn to_iso(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn from_iso(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_default()
}
