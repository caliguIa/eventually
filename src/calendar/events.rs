use chrono::{DateTime, Duration, Local, Timelike};
use objc2::rc::Retained;
use objc2_event_kit::{EKCalendar, EKEvent, EKEventStore};
use objc2_foundation::NSDate;
use std::collections::HashSet;

use super::formatting;

impl From<Vec<EventInfo>> for EventCollection {
    fn from(events: Vec<EventInfo>) -> Self {
        Self(events)
    }
}

const DAYS_TO_FETCH: u8 = 4;
const DEFAULT_CALENDAR_COLOR: (f64, f64, f64) = (0.5, 0.5, 0.5);

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

pub struct EventCollection(Vec<EventInfo>);

impl EventCollection {
    pub fn fetch(store: &EKEventStore) -> Self {
        let (start_date, end_date) = Self::date_range();
        let events = Self::fetch_raw_events(store, &start_date, &end_date);

        let mut event_list: Vec<EventInfo> = events.iter().map(|e| Self::parse_event(e)).collect();

        event_list.sort_by_key(|e| e.start);
        Self(event_list)
    }

    pub fn find_cur_or_next(&self, dismissed: &HashSet<String>) -> Option<EventStatus<'_>> {
        let now = Local::now();
        let today = now.date_naive();
        let mut upcoming = None;

        for event in self
            .0
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

    pub fn get_title(&self, dismissed: &HashSet<String>) -> String {
        let now = Local::now();

        match self.find_cur_or_next(dismissed) {
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

    pub fn into_vec(self) -> Vec<EventInfo> {
        self.0
    }

    fn date_range() -> (Retained<NSDate>, Retained<NSDate>) {
        let today = Local::now().date_naive();

        let start = today
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| dt.and_local_timezone(Local).single())
            .unwrap_or_else(|| {
                Local::now()
                    .with_hour(0)
                    .and_then(|t| t.with_minute(0))
                    .and_then(|t| t.with_second(0))
                    .unwrap_or_else(|| Local::now())
            });

        let end = (today + Duration::days(DAYS_TO_FETCH as i64))
            .and_hms_opt(23, 59, 59)
            .and_then(|dt| dt.and_local_timezone(Local).single())
            .unwrap_or_else(|| {
                Local::now()
                    .with_hour(23)
                    .and_then(|t| t.with_minute(59))
                    .and_then(|t| t.with_second(59))
                    .unwrap_or_else(|| Local::now())
                    + Duration::days(DAYS_TO_FETCH as i64)
            });

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
        use super::super::ffi::event_kit;
        event_kit::fetch_events(store, start, end)
    }

    fn parse_event(event: &EKEvent) -> EventInfo {
        use super::super::ffi::event_kit;
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
            start: Self::timestamp_to_local(start_ts),
            end: Self::timestamp_to_local(end_ts),
            occurrence_key: format!("{event_id_str}|||{}", start_ts as i64),
            event_id: event_id_str,
            has_recurrence,
            location: location.map(|l| l.to_string()),
            calendar_color: calendar
                .map(|c| Self::extract_color(&c))
                .unwrap_or(DEFAULT_CALENDAR_COLOR),
        }
    }

    fn timestamp_to_local(ts: f64) -> DateTime<Local> {
        DateTime::from_timestamp(ts as i64, 0)
            .unwrap_or_else(|| DateTime::UNIX_EPOCH)
            .with_timezone(&Local)
    }

    fn extract_color(calendar: &EKCalendar) -> (f64, f64, f64) {
        use super::super::ffi::event_kit;
        event_kit::get_calendar_color(calendar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_status_current() {
        let event = EventInfo {
            title: "Test Event".to_string(),
            start: Local::now(),
            end: Local::now() + Duration::hours(1),
            event_id: "test-id".to_string(),
            occurrence_key: "test-key".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        };

        let status = EventStatus::Current(&event);
        assert_eq!(status.event().title, "Test Event");
    }

    #[test]
    fn test_event_status_upcoming() {
        let event = EventInfo {
            title: "Test Event".to_string(),
            start: Local::now() + Duration::hours(1),
            end: Local::now() + Duration::hours(2),
            event_id: "test-id".to_string(),
            occurrence_key: "test-key".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        };

        let status = EventStatus::Upcoming(&event);
        assert_eq!(status.event().title, "Test Event");
    }

    #[test]
    fn test_event_collection_find_cur_or_next_current() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Current Event".to_string(),
            start: now - Duration::minutes(30),
            end: now + Duration::minutes(30),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let result = collection.find_cur_or_next(&dismissed);

        assert!(result.is_some());
        if let Some(EventStatus::Current(event)) = result {
            assert_eq!(event.title, "Current Event");
        } else {
            panic!("Expected current event");
        }
    }

    #[test]
    fn test_event_collection_find_cur_or_next_upcoming() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Upcoming Event".to_string(),
            start: now + Duration::hours(1),
            end: now + Duration::hours(2),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let result = collection.find_cur_or_next(&dismissed);

        assert!(result.is_some());
        if let Some(EventStatus::Upcoming(event)) = result {
            assert_eq!(event.title, "Upcoming Event");
        } else {
            panic!("Expected upcoming event");
        }
    }

    #[test]
    fn test_event_collection_dismissed() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Dismissed Event".to_string(),
            start: now + Duration::hours(1),
            end: now + Duration::hours(2),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let mut dismissed = HashSet::new();
        dismissed.insert("key1".to_string());
        let result = collection.find_cur_or_next(&dismissed);

        assert!(result.is_none());
    }

    #[test]
    fn test_event_collection_different_day() {
        let tomorrow = Local::now() + Duration::days(1);
        let events = vec![EventInfo {
            title: "Tomorrow Event".to_string(),
            start: tomorrow,
            end: tomorrow + Duration::hours(1),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let result = collection.find_cur_or_next(&dismissed);

        assert!(result.is_none());
    }

    #[test]
    fn test_event_collection_get_title_current() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Current".to_string(),
            start: now - Duration::minutes(30),
            end: now + Duration::minutes(30),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let title = collection.get_title(&dismissed);

        assert!(title.contains("Current"));
        assert!(title.contains("left"));
    }

    #[test]
    fn test_event_collection_get_title_upcoming() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Upcoming".to_string(),
            start: now + Duration::hours(1),
            end: now + Duration::hours(2),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let title = collection.get_title(&dismissed);

        assert!(title.contains("Upcoming"));
        assert!(title.contains("in"));
    }

    #[test]
    fn test_event_collection_get_title_no_events() {
        let events = vec![];
        let collection = EventCollection(events);
        let dismissed = HashSet::new();
        let title = collection.get_title(&dismissed);

        assert_eq!(title, "No more events today");
    }

    #[test]
    fn test_event_collection_into_vec() {
        let now = Local::now();
        let events = vec![EventInfo {
            title: "Test".to_string(),
            start: now,
            end: now + Duration::hours(1),
            event_id: "id1".to_string(),
            occurrence_key: "key1".to_string(),
            has_recurrence: false,
            location: None,
            calendar_color: (0.5, 0.5, 0.5),
        }];

        let collection = EventCollection(events);
        let vec = collection.into_vec();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0].title, "Test");
    }
}
