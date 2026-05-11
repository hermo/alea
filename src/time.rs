/// Parse ISO 8601 timestamp (with timezone offset or Z) to unix epoch seconds.
pub fn parse_iso8601(s: &str) -> Result<u64, String> {
    let err =
        || format!("invalid timestamp: {s} (expected ISO 8601, e.g. 2026-07-22T12:00:00+03:00)");

    let (datetime_str, offset_secs) = if let Some(stripped) = s.strip_suffix('Z') {
        (stripped, 0i64)
    } else if s.len() >= 6
        && (s.as_bytes()[s.len() - 6] == b'+' || s.as_bytes()[s.len() - 6] == b'-')
    {
        let (dt, tz) = s.split_at(s.len() - 6);
        let sign: i64 = if tz.starts_with('-') { -1 } else { 1 };
        let h: i64 = tz[1..3].parse().map_err(|_| err())?;
        let m: i64 = tz[4..6].parse().map_err(|_| err())?;
        (dt, sign * (h * 3600 + m * 60))
    } else {
        return Err(err());
    };

    let parts: Vec<&str> = datetime_str.split('T').collect();
    if parts.len() != 2 {
        return Err(err());
    }
    let date_parts: Vec<u64> = parts[0]
        .split('-')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let time_parts: Vec<u64> = parts[1]
        .split(':')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Err(err());
    }

    let (year, month, day) = (date_parts[0] as i64, date_parts[1], date_parts[2]);
    let (hour, min, sec) = (time_parts[0], time_parts[1], time_parts[2]);

    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    for d in &month_days(is_leap(year))[..month as usize - 1] {
        days += *d as i64;
    }
    days += (day as i64) - 1;

    let epoch =
        days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64) - offset_secs;
    Ok(epoch as u64)
}

pub fn epoch_to_iso(epoch: i64) -> String {
    let days = epoch / 86400;
    let rem = epoch % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let mut y = 1970i64;
    let mut d = days;
    loop {
        let yd = if is_leap(y) { 366 } else { 365 };
        if d < yd {
            break;
        }
        d -= yd;
        y += 1;
    }

    let mdays = month_days(is_leap(y));
    let mut mo = 0usize;
    while mo < 12 && d >= mdays[mo] as i64 {
        d -= mdays[mo] as i64;
        mo += 1;
    }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", mo + 1, d + 1)
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn month_days(leap: bool) -> [u8; 12] {
    [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ]
}
