//! Local time from the system zone file (TZif, RFC 8536) without extra crates
//!
//! Used for `curdate`, `curday` and `curtime`. The UTC offset comes from the
//! transition table, or for times after its last entry from the POSIX rule in
//! the file's footer (modern "slim" zone files rely on it).

use mep_core::env::LocalDateTime;

const DAY: i64 = 86_400;

/// Returns the days since 1970-01-01 of a civil date (proleptic Gregorian)
fn days_from_civil(year: i64, month: i64, day: i64) -> i64
{
    let year = if month <= 2
    {
        year - 1
    }
    else
    {
        year
    };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Returns (year, month, day) of a day number since 1970-01-01
fn civil_from_days(days: i64) -> (i64, i64, i64)
{
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10
    {
        mp + 3
    }
    else
    {
        mp - 9
    };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// Converts local seconds since the epoch into a date and time
pub fn civil(seconds: i64) -> LocalDateTime
{
    let days = seconds.div_euclid(DAY);
    let time = seconds.rem_euclid(DAY);
    let (year, month, day) = civil_from_days(days);
    LocalDateTime {
        year:    year as u16,
        month:   month as u8,
        day:     day as u8,
        weekday: (days + 4).rem_euclid(7) as u8,
        hour:    (time / 3600) as u8,
        minute:  (time / 60 % 60) as u8,
        second:  (time % 60) as u8
    }
}

/// Reads a big-endian signed number of `width` bytes at `at`
fn number(bytes: &[u8], at: usize, width: usize) -> Option<i64>
{
    let field = bytes.get(at..at + width)?;
    let unsigned = field.iter().fold(0u64, |value, byte| value << 8 | u64::from(*byte));
    let shift = 64 - 8 * width as u32;
    Some((unsigned << shift) as i64 >> shift)
}

/// Returns the UTC offset in seconds at `utc` from a TZif file
pub fn offset_at(tzif: &[u8], utc: i64) -> Option<i64>
{
    require!(tzif.get(..4) == Some(b"TZif".as_slice()), None);
    let version = *tzif.get(4)?;
    // Header: magic, version, 15 reserved bytes, then six counts.
    let counts = |at: usize| [0, 1, 2, 3, 4, 5].map(|index| number(tzif, at + 20 + 4 * index, 4).unwrap_or(0) as usize);
    let block_size = |counts: [usize; 6], time: usize| {
        let [isut, isstd, leap, times, types, chars] = counts;
        times * time + times + types * 6 + chars + leap * (time + 4) + isstd + isut
    };
    let (start, time) = match version
    {
        | 0 => (0, 4),
        | _ => (44 + block_size(counts(0), 4), 8)
    };
    let [isut, isstd, leap, times, types, chars] = counts(start);
    let data = start + 44;
    let transitions: Vec<i64> = (0..times).map(|index| number(tzif, data + index * time, time)).collect::<Option<_>>()?;
    let indices = tzif.get(data + times * time..data + times * time + times)?;
    let infos = data + times * time + times;
    let offset_of = |index: u8| number(tzif, infos + usize::from(index) * 6, 4);
    let last = transitions.iter().rposition(|at| *at <= utc);
    let footer_start = data + block_size([isut, isstd, leap, times, types, chars], time);
    let footer = tzif
        .get(footer_start..)
        .and_then(|rest| rest.strip_prefix(b"\n"))
        .and_then(|rest| rest.split(|byte| *byte == b'\n').next())
        .and_then(|rule| std::str::from_utf8(rule).ok())
        .filter(|rule| rule.is_empty() == false);
    match (last, footer)
    {
        | (Some(index), Some(rule)) if index + 1 == times => posix_offset(rule, utc),
        | (Some(index), _) => offset_of(indices[index]),
        | (None, Some(rule)) if times == 0 => posix_offset(rule, utc),
        | (None, _) if types > 0 => offset_of(0),
        | (None, _) => None
    }
}

/// Parses `[+|-]hh[:mm[:ss]]` and returns the seconds and the rest
fn hms(text: &str) -> Option<(i64, &str)>
{
    let (sign, text) = match text.strip_prefix('-')
    {
        | Some(rest) => (-1, rest),
        | None => (1, text.strip_prefix('+').unwrap_or(text))
    };
    let end = text.find(|ch: char| ch.is_ascii_digit() == false && ch != ':').unwrap_or(text.len());
    require!(end > 0, None);
    let mut parts = text[..end].split(':').map(|part| part.parse::<i64>().ok());
    let hours = parts.next().flatten()?;
    let minutes = parts.next().flatten().unwrap_or(0);
    let seconds = parts.next().flatten().unwrap_or(0);
    Some((sign * (hours * 3600 + minutes * 60 + seconds), &text[end..]))
}

/// Skips a zone abbreviation: letters, or anything between `<` and `>`
fn name(text: &str) -> Option<&str>
{
    let rest = match text.strip_prefix('<')
    {
        | Some(quoted) => &quoted[quoted.find('>')? + 1..],
        | None => text.trim_start_matches(|ch: char| ch.is_ascii_alphabetic())
    };
    require!(rest.len() < text.len(), None);
    Some(rest)
}

/// Returns the local time in seconds since the epoch at which a `Mm.w.d[/time]` rule fires in `year`
fn rule_time(rule: &str, year: i64) -> Option<i64>
{
    let (date, time) = rule.split_once('/').map_or((rule, None), |(date, time)| (date, Some(time)));
    let mut fields = date.strip_prefix('M')?.split('.').map(|field| field.parse::<i64>().ok());
    let (month, week, weekday) = (fields.next().flatten()?, fields.next().flatten()?, fields.next().flatten()?);
    let first = days_from_civil(year, month, 1);
    let mut day = 1 + (weekday - (first + 4).rem_euclid(7)).rem_euclid(7) + (week - 1) * 7;
    let next_month = if month == 12
    {
        days_from_civil(year + 1, 1, 1)
    }
    else
    {
        days_from_civil(year, month + 1, 1)
    };
    while first + day > next_month
    {
        day -= 7;
    }
    let seconds = time.map_or(Some(2 * 3600), |time| hms(time).map(|(seconds, _)| seconds))?;
    Some((first + day - 1) * DAY + seconds)
}

/// Returns the UTC offset at `utc` from a POSIX TZ rule such as `CET-1CEST,M3.5.0,M10.5.0/3`
pub fn posix_offset(rule: &str, utc: i64) -> Option<i64>
{
    let (std_west, rest) = hms(name(rule)?)?;
    let standard = -std_west;
    if rest.is_empty() == true
    {
        return Some(standard);
    }
    let rest = name(rest)?;
    let (dst, rest) = match rest.starts_with(',')
    {
        | true => (standard + 3600, rest),
        | false =>
        {
            let (west, rest) = hms(rest)?;
            (-west, rest)
        }
    };
    let (start, end) = rest.strip_prefix(',')?.split_once(',')?;
    let year = civil_from_days((utc + standard).div_euclid(DAY)).0;
    let begins = rule_time(start, year)? - standard;
    let ends = rule_time(end, year)? - dst;
    let summer = match begins < ends
    {
        | true => (begins..ends).contains(&utc),
        | false => (ends..begins).contains(&utc) == false
    };
    Some(
        if summer
        {
            dst
        }
        else
        {
            standard
        }
    )
}

#[cfg(test)]
#[coverage(off)]
mod tests
{
    use super::*;

    /// Builds a TZif file with the given version, transitions, offsets and footer
    fn tzif(version: u8, transitions: &[(i64, u8)], offsets: &[i32], footer: &str) -> Vec<u8>
    {
        let header = |time: usize| {
            let mut bytes = b"TZif".to_vec();
            bytes.push(version);
            bytes.extend([0; 15]);
            for count in [0, 0, 0, transitions.len(), offsets.len(), 4]
            {
                bytes.extend((count as u32).to_be_bytes());
            }
            let _ = time;
            bytes
        };
        let block = |time: usize| {
            let mut bytes = Vec::new();
            for (at, _) in transitions
            {
                bytes.extend(&at.to_be_bytes()[8 - time..]);
            }
            bytes.extend(transitions.iter().map(|(_, index)| *index));
            for offset in offsets
            {
                bytes.extend(offset.to_be_bytes());
                bytes.extend([0, 0]);
            }
            bytes.extend(b"UTC\0");
            bytes
        };
        let mut bytes = header(4);
        bytes.extend(block(4));
        if version != 0
        {
            bytes.extend(header(8));
            bytes.extend(block(8));
            bytes.extend(format!("\n{footer}\n").as_bytes());
        }
        bytes
    }

    const BERLIN: &str = "CET-1CEST,M3.5.0,M10.5.0/3";

    #[test]
    fn test_civil_dates()
    {
        assert_eq!(civil(0), LocalDateTime { year: 1970, month: 1, day: 1, weekday: 4, hour: 0, minute: 0, second: 0 });
        // 1987-06-27 13:45:55, a Saturday (the manual's example)
        assert_eq!(civil(551_799_955), LocalDateTime { year: 1987, month: 6, day: 27, weekday: 6, hour: 13, minute: 45, second: 55 });
        assert_eq!(civil(951_782_400).month, 2);
        assert_eq!(civil(951_782_400).day, 29);
        assert_eq!(civil(-86_400).year, 1969);
        assert_eq!(days_from_civil(2026, 11, 1), 20_758);
        assert_eq!(civil_from_days(20_758), (2026, 11, 1));
    }

    #[test]
    fn test_posix_rules()
    {
        // 2026-07-01 12:00 UTC and 2026-01-15 12:00 UTC
        let (summer, winter) = (1_782_907_200, 1_768_478_400);
        assert_eq!(posix_offset(BERLIN, summer), Some(7200));
        assert_eq!(posix_offset(BERLIN, winter), Some(3600));
        assert_eq!(posix_offset("EST5EDT,M3.2.0,M11.1.0", summer), Some(-4 * 3600));
        assert_eq!(posix_offset("UTC0", summer), Some(0));
        assert_eq!(posix_offset("<+0530>-5:30", summer), Some(19_800));
        // Southern hemisphere: summer from October to April
        assert_eq!(posix_offset("AEST-10AEDT,M10.1.0,M4.1.0/3", winter), Some(11 * 3600));
        assert_eq!(posix_offset("AEST-10AEDT,M10.1.0,M4.1.0/3", summer), Some(10 * 3600));
        assert_eq!(posix_offset("XST3XDT2,M3.5.0/-1:30:15,M12.5.6/25", summer), Some(-2 * 3600));
        assert_eq!(posix_offset("XST3XDT2,M3.5.0,M12.5.6", 1_798_675_200), Some(-3 * 3600));
    }

    #[test]
    fn test_posix_rule_errors()
    {
        for rule in [
            "", "123", "CET", "CET-1CEST", "CET-1CEST,M3.5.0", "CET-1CEST,J60,J300", "CET-1CEST,M3.x.0,M10.5.0", "CET-1CEST,M3.5.0/x,M10.5.0", "<UTC",
            "CET-1CEST1x,M3.5.0,M10.5.0"
        ]
        {
            assert_eq!(posix_offset(rule, 0), None, "{rule}");
        }
        assert_eq!(posix_offset("CET-1:", 0), Some(3600));
        for rule in ["CET-1,M3.5.0,M10.5.0", "CET-1CEST,M3.5.0,X", "CET:5", "CET-1CEST,M3,M10.5.0", "CET-1CEST,M3.5,M10.5.0", "CET-1CEST,Mx.5.0,M10.5.0"]
        {
            assert_eq!(posix_offset(rule, 0), None, "{rule}");
        }
    }

    #[test]
    fn test_tzif_table_and_footer()
    {
        let file = tzif(b'2', &[(-100, 0), (1000, 1)], &[3600, 7200], BERLIN);
        assert_eq!(offset_at(&file, 500), Some(3600));
        assert_eq!(offset_at(&file, -500), Some(3600));
        assert_eq!(offset_at(&file, 1_782_907_200), Some(7200));
        assert_eq!(offset_at(&file, 1_768_478_400), Some(3600));
        let no_footer = tzif(b'2', &[(1000, 1)], &[3600, 7200], "");
        assert_eq!(offset_at(&no_footer, 5000), Some(7200));
        let only_footer = tzif(b'3', &[], &[0], BERLIN);
        assert_eq!(offset_at(&only_footer, 1_782_907_200), Some(7200));
        let version_one = tzif(0, &[(1000, 1)], &[3600, 7200], "");
        assert_eq!(offset_at(&version_one, 5000), Some(7200));
        assert_eq!(offset_at(&version_one, 0), Some(3600));
    }

    #[test]
    fn test_tzif_errors()
    {
        assert_eq!(offset_at(b"nope", 0), None);
        assert_eq!(offset_at(b"TZif", 0), None);
        assert_eq!(offset_at(&tzif(b'2', &[], &[], ""), 0), None);
        let mut truncated = tzif(b'2', &[(1000, 1)], &[3600, 7200], BERLIN);
        truncated.truncate(60);
        assert_eq!(offset_at(&truncated, 0), None);
        let mut no_transitions = tzif(0, &[(1000, 0)], &[3600], "");
        no_transitions.truncate(44);
        assert_eq!(offset_at(&no_transitions, 0), None);
        let mut short_indices = tzif(0, &[(1000, 0)], &[3600], "");
        short_indices.truncate(48);
        assert_eq!(offset_at(&short_indices, 0), None);
        assert_eq!(number(b"\x01", 0, 4), None);
    }
}
