use chrono::{DateTime, Duration, Local};
use objc2::rc::Retained;
use objc2_event_kit::{EKCalendar, EKEvent, EKEventStore};
use objc2_foundation::NSDate;
use std::collections::HashSet;

use super::formatting;

const DAYS_TO_FETCH: u8 = 4;
const DEFAULT_CALENDAR_COLOR: (f64, f64, f64) = (0.5, 0.5, 0.5);

/// Calendar event information
#[derive(Clone, Debug, PartialEq)]
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

/// Status of an event (currently happening or upcoming)
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

/// Fetches calendar events for the next few days
pub fn fetch(store: &EKEventStore) -> Vec<EventInfo> {
    let (start_date, end_date) = date_range();
    let events = fetch_raw_events(store, &start_date, &end_date);

    let mut event_list: Vec<EventInfo> = events.iter().map(|e| parse_event(e)).collect();

    event_list.sort_by_key(|e| e.start);
    event_list
}

/// Finds the current or next event for today (excluding dismissed events)
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

/// Generates the status bar title based on current/upcoming events
pub fn get_title(events: &[EventInfo], dismissed: &HashSet<String>) -> String {
    let now = Local::now();

    match find_cur_or_next(events, dismissed) {
        Some(EventStatus::Current(e)) => {
            let remaining = e.end.signed_duration_since(now);
            formatting::format_event_title(&e.title, remaining, "{} • {} left")
        }
        Some(EventStatus::Upcoming(e)) => {
            let until = e.start.signed_duration_since(now);
            formatting::format_event_title(&e.title, until, "{} • in {}")
        }
        None => "No more events today".to_string(),
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

fn fetch_raw_events(
    store: &EKEventStore,
    start: &NSDate,
    end: &NSDate,
) -> Vec<Retained<EKEvent>> {
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
