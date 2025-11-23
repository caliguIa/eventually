use chrono::{DateTime, Duration, Local, Timelike};
use std::borrow::Cow;

const MAX_TITLE_LENGTH: usize = 30;
const END_OF_DAY_SECS: u32 = 86399;

/// Formats time as HH:MM
pub fn format_time(dt: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

/// Checks if an event is all-day (starts at midnight, ends at 23:59:59)
pub fn is_all_day(start: &DateTime<Local>, end: &DateTime<Local>) -> bool {
    start.time().num_seconds_from_midnight() == 0
        && end.time().num_seconds_from_midnight() == END_OF_DAY_SECS
}

/// Formats an event title for the status bar with time information
pub fn format_event_title(title: &str, duration: Duration, template: &str) -> String {
    let mins = duration.num_minutes();
    let time_str = if mins > 60 {
        format!("{}h", mins / 60)
    } else {
        format!("{}m", mins)
    };

    let overhead = template.len() - 4 + time_str.len();
    let max_len = MAX_TITLE_LENGTH.saturating_sub(overhead);
    let title = truncate_title(title, max_len);

    template
        .replacen("{}", &title, 1)
        .replacen("{}", &time_str, 1)
}

/// Truncates a title to max length, adding ellipsis if needed
pub fn truncate_title(title: &str, max_len: usize) -> Cow<'_, str> {
    if title.chars().count() <= max_len {
        Cow::Borrowed(title)
    } else {
        let mut truncated: String = title.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('…');
        Cow::Owned(truncated)
    }
}
