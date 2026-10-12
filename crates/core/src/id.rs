//! Job IDs: `yyyy.mm.dd-hh.mm.ss-N`, UTC creation time plus a counter that
//! restarts at 1 every second. See docs/decisions/0006.

use std::fmt;
use std::str::FromStr;

use crate::error::Error;

const SECS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId {
    // Field order matters: sorting compares `secs` first, so IDs sort by creation time.
    secs: u64,
    seq: u32,
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = civil_from_days(self.secs / SECS_PER_DAY);
        let time = self.secs % SECS_PER_DAY;
        write!(
            f,
            "{year:04}.{month:02}.{day:02}-{:02}.{:02}.{:02}-{}",
            time / 3600,
            time / 60 % 60,
            time % 60,
            self.seq
        )
    }
}

impl FromStr for JobId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        parse(s).ok_or_else(|| Error::InvalidJobId(s.to_owned()))
    }
}

fn parse(s: &str) -> Option<JobId> {
    let mut parts = s.splitn(3, '-');
    let (date, time, seq) = (parts.next()?, parts.next()?, parts.next()?);
    let [year, month, day] = three_numbers(date)?;
    let [hour, minute, second] = three_numbers(time)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let id = JobId {
        secs: days * SECS_PER_DAY + hour * 3600 + minute * 60 + second,
        seq: seq.parse().ok().filter(|&n| n > 0)?,
    };
    // Formatting back must give the same text. This rejects impossible
    // dates like 2025.02.30, missing zero padding, "+1", "01" and so on.
    (id.to_string() == s).then_some(id)
}

fn three_numbers(s: &str) -> Option<[u64; 3]> {
    let mut it = s.split('.').map(|n| n.parse::<u64>().ok());
    let numbers = [it.next()??, it.next()??, it.next()??];
    it.next().is_none().then_some(numbers)
}

/// Hands out IDs that only ever increase, even if the clock goes backwards.
#[derive(Debug, Default)]
pub(crate) struct IdGenerator {
    last: Option<JobId>,
}

impl IdGenerator {
    /// `now` is seconds since 1970 (UTC).
    pub(crate) fn next(&mut self, now: u64) -> JobId {
        let id = match self.last {
            Some(last) if now <= last.secs => JobId {
                secs: last.secs,
                seq: last.seq + 1,
            },
            _ => JobId { secs: now, seq: 1 },
        };
        self.last = Some(id);
        id
    }
}

// Date conversions from Howard Hinnant's public-domain algorithms
// (https://howardhinnant.github.io/date_algorithms.html), limited to dates
// from 1970 onwards so everything stays unsigned.

fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// `year` must be at least 1970.
fn days_from_civil(year: u64, month: u64, day: u64) -> u64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let yoe = year % 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(secs: u64, seq: u32) -> JobId {
        JobId { secs, seq }
    }

    #[test]
    fn formats_utc_dates() {
        // Reference values from `date -u -d @<secs>`.
        assert_eq!(id(0, 1).to_string(), "1970.01.01-00.00.00-1");
        assert_eq!(id(951_782_400, 2).to_string(), "2000.02.29-00.00.00-2");
        assert_eq!(id(1_709_164_800, 1).to_string(), "2024.02.29-00.00.00-1");
        assert_eq!(id(1_760_279_405, 12).to_string(), "2025.10.12-14.30.05-12");
        assert_eq!(id(4_102_444_799, 1).to_string(), "2099.12.31-23.59.59-1");
    }

    #[test]
    fn parsing_round_trips() {
        for secs in [0, 951_782_400, 1_760_279_405, 4_102_444_799] {
            let original = id(secs, 7);
            assert_eq!(original.to_string().parse::<JobId>().unwrap(), original);
        }
    }

    #[test]
    fn rejects_malformed_ids() {
        for bad in [
            "",
            "12",
            "2025.10.12-14.30.05",
            "2025.10.12-14.30.05-0",
            "2025.10.12-14.30.05-01",
            "2025.10.12-14.30.05-+1",
            "2025.02.30-00.00.00-1",
            "2025.13.01-00.00.00-1",
            "2025.10.12-24.00.00-1",
            "2025.1.12-14.30.05-1",
            "2025.10.12.1-14.30.05-1",
            "1969.12.31-23.59.59-1",
            "2025.10.12-14.30.05-1-2",
        ] {
            assert!(bad.parse::<JobId>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn counter_restarts_each_second() {
        let mut ids = IdGenerator::default();
        assert_eq!(ids.next(100), id(100, 1));
        assert_eq!(ids.next(100), id(100, 2));
        assert_eq!(ids.next(101), id(101, 1));
    }

    #[test]
    fn ids_keep_increasing_when_the_clock_goes_back() {
        let mut ids = IdGenerator::default();
        let first = ids.next(100);
        let second = ids.next(50);
        assert_eq!(second, id(100, 2));
        assert!(second > first);
    }
}
