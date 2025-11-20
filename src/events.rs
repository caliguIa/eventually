use chrono::{DateTime, Duration, Local, Timelike};
use objc2_event_kit::{EKEntityType, EKEventStore};
use std::collections::HashSet;

const MAX_TITLE_LENGTH: usize = 30;

#[derive(Clone)]
pub struct EventInfo {
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub event_id: String,
    pub occurrence_key: String,
    pub has_recurrence: bool,
    pub location: Option<String>,
}

pub fn request_calendar_access(store: &EKEventStore) -> bool {
    use block2::StackBlock;
    use std::sync::mpsc::channel;

    let (tx, rx) = channel();

    unsafe {
        let block = StackBlock::new(
            move |granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
                let _ = tx.send(granted.as_bool());
            },
        );
        let block_ptr: *mut _ = &block as *const _ as *mut _;
        store.requestFullAccessToEventsWithCompletion(block_ptr);
    }

    rx.recv().unwrap_or(false)
}

pub fn fetch_events(store: &EKEventStore) -> Vec<EventInfo> {
    let now = Local::now();
    let start_of_today = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    let end_of_four_days = (start_of_today + Duration::days(4))
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();

    let start_ns_date = unsafe {
        objc2_foundation::NSDate::dateWithTimeIntervalSince1970(start_of_today.timestamp() as f64)
    };
    let end_ns_date = unsafe {
        objc2_foundation::NSDate::dateWithTimeIntervalSince1970(end_of_four_days.timestamp() as f64)
    };

    unsafe {
        let calendars = store.calendarsForEntityType(EKEntityType::Event);
        let predicate = store.predicateForEventsWithStartDate_endDate_calendars(
            &start_ns_date,
            &end_ns_date,
            Some(&calendars),
        );

        let events = store.eventsMatchingPredicate(&predicate);

        let mut event_list = Vec::new();
        for i in 0..events.count() {
            let event = events.objectAtIndex(i);
            let title = event.title();
            let start_date = event.startDate();
            let end_date = event.endDate();
            let event_id = event.eventIdentifier();
            let location = event.location();
            let has_recurrence = event.hasRecurrenceRules();

            let start_timestamp = start_date.timeIntervalSince1970();
            let end_timestamp = end_date.timeIntervalSince1970();

            let start_dt = DateTime::from_timestamp(start_timestamp as i64, 0)
                .unwrap()
                .with_timezone(&Local);
            let end_dt = DateTime::from_timestamp(end_timestamp as i64, 0)
                .unwrap()
                .with_timezone(&Local);

            let event_id_str = event_id.map(|id| id.to_string()).unwrap_or_default();
            let occurrence_key = format!("{}|||{}", event_id_str, start_timestamp as i64);

            let location_str = location.map(|loc| loc.to_string());

            event_list.push(EventInfo {
                title: title.to_string(),
                start: start_dt,
                end: end_dt,
                event_id: event_id_str,
                occurrence_key,
                has_recurrence,
                location: location_str,
            });
        }

        event_list.sort_by_key(|e| e.start);
        event_list
    }
}

pub fn format_time(dt: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

pub fn is_all_day_event(start: &DateTime<Local>, end: &DateTime<Local>) -> bool {
    start.time().num_seconds_from_midnight() == 0 && end.time().num_seconds_from_midnight() == 86399
}

pub fn extract_url_from_location(location: &Option<String>) -> Option<String> {
    location.as_ref().and_then(|loc| {
        if loc.starts_with("http://") || loc.starts_with("https://") {
            Some(loc.clone())
        } else {
            None
        }
    })
}

pub fn get_service_name_from_url(url: &str) -> String {
    if url.contains("slack.com") {
        "Slack".to_string()
    } else if url.contains("zoom.us") {
        "Zoom".to_string()
    } else if url.contains("meet.google.com") || url.contains("meet.google") {
        "Google Meet".to_string()
    } else if url.contains("teams.microsoft.com") || url.contains("teams.live.com") {
        "Teams".to_string()
    } else {
        "Video Call".to_string()
    }
}

pub fn find_current_or_next_event<'a>(
    events: &'a [EventInfo],
    dismissed: &HashSet<String>,
) -> Option<&'a EventInfo> {
    let now = Local::now();
    let today = now.date_naive();

    let today_events: Vec<_> = events
        .iter()
        .filter(|e| e.start.date_naive() == today && !dismissed.contains(&e.occurrence_key))
        .collect();

    for event in &today_events {
        if event.start <= now && now <= event.end {
            return Some(event);
        }
    }

    today_events.into_iter().find(|e| e.start > now)
}

pub fn get_status_bar_title(events: &[EventInfo], dismissed: &HashSet<String>) -> String {
    let now = Local::now();
    let today = now.date_naive();

    let today_events: Vec<_> = events
        .iter()
        .filter(|e| e.start.date_naive() == today && !dismissed.contains(&e.occurrence_key))
        .collect();

    for event in &today_events {
        if event.start <= now && now <= event.end {
            let remaining = event.end.signed_duration_since(now);
            let mins = remaining.num_minutes();

            let time_str = if mins > 60 {
                format!("{}h", mins / 60)
            } else {
                format!("{}m", mins)
            };

            let mut title = event.title.clone();
            let max_event_len = MAX_TITLE_LENGTH.saturating_sub(time_str.len() + 6);
            if title.len() > max_event_len {
                title.truncate(max_event_len.saturating_sub(1));
                title.push('…');
            }

            return format!("{} • {} left", title, time_str);
        }
    }

    for event in &today_events {
        if event.start > now {
            let until = event.start.signed_duration_since(now);
            let mins = until.num_minutes();

            let time_str = if mins > 60 {
                format!("{}h", mins / 60)
            } else {
                format!("{}m", mins)
            };

            let mut title = event.title.clone();
            let max_event_len = MAX_TITLE_LENGTH.saturating_sub(time_str.len() + 10);
            if title.len() > max_event_len {
                title.truncate(max_event_len.saturating_sub(1));
                title.push('…');
            }

            return format!("{} • in {}", title, time_str);
        }
    }

    if !today_events.is_empty() {
        "No more events today".to_string()
    } else {
        "No events today".to_string()
    }
}
