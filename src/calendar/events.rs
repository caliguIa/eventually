use chrono::{DateTime, Duration, Local, Timelike};
use objc2::rc::Retained;
use objc2_event_kit::{EKCalendar, EKEvent, EKEventStore};
use objc2_foundation::NSDate;
use std::{borrow::Cow, collections::HashSet};

const MAX_TITLE_LENGTH: usize = 30;
const DAYS_TO_FETCH: u8 = 4;
const DEFAULT_CALENDAR_COLOR: (f64, f64, f64) = (0.5, 0.5, 0.5);
const END_OF_DAY_SECS: u32 = 86399;

#[derive(Copy, Clone)]
pub struct ServiceInfo {
    pub pattern: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
}
const SERVICES: &[ServiceInfo] = &[
    ServiceInfo {
        pattern: "slack.com",
        name: "Slack",
        icon: "slack",
    },
    ServiceInfo {
        pattern: "zoom.us",
        name: "Zoom",
        icon: "zoom",
    },
    ServiceInfo {
        pattern: "meet.google",
        name: "Google Meet",
        icon: "google",
    },
    ServiceInfo {
        pattern: "teams.microsoft.com",
        name: "Teams",
        icon: "teams",
    },
    ServiceInfo {
        pattern: "teams.live.com",
        name: "Teams",
        icon: "teams",
    },
];

#[derive(Clone)]
pub struct EventInfo {
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub event_id: String,
    pub occurrence_key: String,
    pub has_recurrence: bool,
    pub location: Option<String>,
    pub calendar_color: (f64, f64, f64),
}

pub enum EventStatus<'a> {
    Current(&'a EventInfo),
    Upcoming(&'a EventInfo),
}

impl<'a> EventStatus<'a> {
    pub fn event(&self) -> &'a EventInfo {
        match self {
            EventStatus::Current(e) => e,
            EventStatus::Upcoming(e) => e,
        }
    }
}

pub fn fetch(store: &EKEventStore) -> Vec<EventInfo> {
    let (start_date, end_date) = date_range();
    let events = fetch_raw_events(store, &start_date, &end_date);

    let mut event_list: Vec<EventInfo> = events.iter().map(|e| parse_event(e)).collect();

    event_list.sort_by_key(|e| e.start);
    event_list
}

pub fn format_time(dt: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

pub fn is_all_day(start: &DateTime<Local>, end: &DateTime<Local>) -> bool {
    start.time().num_seconds_from_midnight() == 0
        && end.time().num_seconds_from_midnight() == END_OF_DAY_SECS
}

pub fn extract_url(location: Option<&str>) -> Option<&str> {
    location.filter(|loc| loc.starts_with("http://") || loc.starts_with("https://"))
}

pub fn get_service_info(url: &str) -> ServiceInfo {
    SERVICES
        .iter()
        .find(|s| url.contains(s.pattern))
        .copied()
        .unwrap_or(ServiceInfo {
            pattern: "",
            name: "Video Call",
            icon: "video",
        })
}

pub fn find_cur_or_next<'a>(
    events: &'a [EventInfo],
    dismissed: &HashSet<String>,
) -> Option<EventStatus<'a>> {
    let now = Local::now();
    let today = now.date_naive();
    let mut upcoming = None;

    for event in events
        .iter()
        .filter(|e| e.start.date_naive() == today && !dismissed.contains(&e.occurrence_key))
    {
        if event.start <= now && now <= event.end {
            return Some(EventStatus::Current(event));
        }
        if event.start > now {
            upcoming.get_or_insert(EventStatus::Upcoming(event));
        }
    }

    upcoming
}

pub fn get_title(events: &[EventInfo], dismissed: &HashSet<String>) -> String {
    let now = Local::now();

    match find_cur_or_next(events, dismissed) {
        Some(EventStatus::Current(e)) => {
            let remaining = e.end.signed_duration_since(now);
            format_event_title(&e.title, remaining, "{} • {} left")
        }
        Some(EventStatus::Upcoming(e)) => {
            let until = e.start.signed_duration_since(now);
            format_event_title(&e.title, until, "{} • in {}")
        }
        None => "No more events today".to_string(),
    }
}

fn format_event_title(title: &str, duration: Duration, template: &str) -> String {
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

fn truncate_title(title: &str, max_len: usize) -> Cow<'_, str> {
    if title.chars().count() <= max_len {
        Cow::Borrowed(title)
    } else {
        let mut truncated: String = title.chars().take(max_len.saturating_sub(1)).collect();
        truncated.push('…');
        Cow::Owned(truncated)
    }
}

fn date_range() -> (Retained<NSDate>, Retained<NSDate>) {
    let today = Local::now().date_naive();

    let start = today
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .expect("valid start of day");

    let end = (today + Duration::days(DAYS_TO_FETCH as i64))
        .and_hms_opt(23, 59, 59)
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .expect("valid end of day");

    (
        NSDate::dateWithTimeIntervalSince1970(start.timestamp() as f64),
        NSDate::dateWithTimeIntervalSince1970(end.timestamp() as f64),
    )
}

fn fetch_raw_events(store: &EKEventStore, start: &NSDate, end: &NSDate) -> Vec<Retained<EKEvent>> {
    use crate::ffi::event_kit;
    event_kit::fetch_events(store, start, end)
}

fn parse_event(event: &EKEvent) -> EventInfo {
    use crate::ffi::event_kit;
    let (start_date, end_date, event_id, title, location, calendar, has_recurrence) =
        event_kit::get_event_properties(event);

    let start_ts = start_date.timeIntervalSince1970();
    let end_ts = end_date.timeIntervalSince1970();
    let event_id_str = event_id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_default();

    EventInfo {
        title: title.to_string(),
        start: timestamp_to_local(start_ts),
        end: timestamp_to_local(end_ts),
        occurrence_key: format!("{event_id_str}|||{}", start_ts as i64),
        event_id: event_id_str,
        has_recurrence,
        location: location.map(|l| l.to_string()),
        calendar_color: calendar
            .map(|c| extract_color(&c))
            .unwrap_or(DEFAULT_CALENDAR_COLOR),
    }
}

fn timestamp_to_local(ts: f64) -> DateTime<Local> {
    DateTime::from_timestamp(ts as i64, 0)
        .expect("valid timestamp")
        .with_timezone(&Local)
}

fn extract_color(calendar: &EKCalendar) -> (f64, f64, f64) {
    use crate::ffi::event_kit;
    event_kit::get_calendar_color(calendar)
}
