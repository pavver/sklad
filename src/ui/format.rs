//! Numbers and dates the way a person reads them.

use crate::i18n::{DateStyle, Text};
use crate::model::{Evidence, Usage};

pub fn size(bytes: u64, text: &Text) -> String {
    let units = text.size_units;
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", units[0])
    } else {
        let number = format!("{value:.1}").replace('.', &text.decimal_mark.to_string());
        format!("{number} {}", units[unit])
    }
}

pub fn ago(at: i64, now: i64, text: &Text) -> String {
    let days = (now - at).max(0) / 86_400;
    match days {
        0 => text.today.to_owned(),
        1 => text.yesterday.to_owned(),
        2..=29 => (text.days_ago)(days),
        30..=364 => (text.months_ago)(days / 30),
        _ => (text.years_ago)(days / 365),
    }
}

/// A calendar date from Unix time, by the civil-from-days algorithm.
pub fn date(at: i64, text: &Text) -> String {
    let days = at.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    match text.date_style {
        DateStyle::DayFirst => format!("{day:02}.{month:02}.{year}"),
        DateStyle::YearFirst => format!("{year}-{month:02}-{day:02}"),
    }
}

/// The short form for the list.
pub fn usage(usage: Usage, now: i64, text: &Text) -> String {
    match usage {
        Usage::Running => text.running_now.to_owned(),
        Usage::Last { at, .. } => ago(at, now, text),
        Usage::NoRecord => text.no_record.to_owned(),
        Usage::NotTracked => "—".to_owned(),
    }
}

/// The long form for the card of a program.
pub fn usage_detail(usage: Usage, now: i64, text: &Text) -> String {
    match usage {
        Usage::Running => text.running_now_long.to_owned(),
        Usage::Last { at, by } => {
            let source = match by {
                Evidence::Plasma => text.plasma_log,
                Evidence::Steam => "Steam",
            };
            let when = format!("{} ({})", date(at, text), ago(at, now, text));
            (text.last_launch)(&when, source)
        }
        Usage::NoRecord => text.no_record_long.to_owned(),
        Usage::NotTracked => text.not_tracked_long.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn uk() -> &'static Text {
        Lang::Uk.text()
    }

    fn en() -> &'static Text {
        Lang::En.text()
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(size(512, uk()), "512 Б");
        assert_eq!(size(1536, uk()), "1,5 КБ");
        assert_eq!(size(500_313_517, uk()), "477,1 МБ");
        assert_eq!(size(512, en()), "512 B");
        assert_eq!(size(500_313_517, en()), "477.1 MB");
    }

    #[test]
    fn relative_time_reads_naturally() {
        let now = 1_000_000_000;
        assert_eq!(ago(now - 100, now, uk()), "сьогодні");
        assert_eq!(ago(now - 86_400, now, uk()), "вчора");
        assert_eq!(ago(now - 5 * 86_400, now, uk()), "5 днів тому");
        assert_eq!(ago(now - 62 * 86_400, now, uk()), "2 місяці тому");
        assert_eq!(ago(now - 400 * 86_400, now, uk()), "1 рік тому");
        assert_eq!(ago(now - 86_400, now, en()), "yesterday");
        assert_eq!(ago(now - 62 * 86_400, now, en()), "2 months ago");
        assert_eq!(ago(now - 400 * 86_400, now, en()), "1 year ago");
    }

    #[test]
    fn a_future_time_counts_as_today() {
        assert_eq!(ago(2_000, 1_000, uk()), "сьогодні");
    }

    #[test]
    fn dates_are_correct_around_a_leap_day() {
        assert_eq!(date(0, uk()), "01.01.1970");
        assert_eq!(date(1_709_164_800, uk()), "29.02.2024");
        assert_eq!(date(1_709_251_200, uk()), "01.03.2024");
        assert_eq!(date(1_709_164_800, en()), "2024-02-29");
    }
}
