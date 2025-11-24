mod error;
mod events;
mod formatting;
mod service;

use objc2_event_kit::EKEventStore;

pub use error::CalendarError;
pub use events::{EventCollection, EventInfo, EventStatus};
pub use formatting::{format_time, is_all_day};
pub use service::{extract_url, Icon, ServiceInfo, SlackHuddleUrl};

pub fn request_access(store: &EKEventStore) -> Result<(), CalendarError> {
    use super::ffi::event_kit;
    if event_kit::request_calendar_access(store) {
        Ok(())
    } else {
        Err(CalendarError::AccessDenied)
    }
}
