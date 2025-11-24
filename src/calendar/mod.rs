mod error;
mod events;
mod formatting;
mod service;

use objc2_event_kit::EKEventStore;

pub use error::CalendarError;
pub use events::{fetch, find_cur_or_next, get_title, EventInfo, EventStatus};
pub use formatting::{format_time, is_all_day};
pub use service::{detect_service as get_service_info, extract_url};

pub fn request_access(store: &EKEventStore) -> Result<(), CalendarError> {
    use crate::ffi::event_kit;
    if event_kit::request_calendar_access(store) {
        Ok(())
    } else {
        Err(CalendarError::AccessDenied)
    }
}
