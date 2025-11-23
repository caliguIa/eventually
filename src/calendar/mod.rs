mod events;
mod formatting;
mod service;

use objc2_event_kit::EKEventStore;

// Re-export public API
pub use events::{fetch, find_cur_or_next, get_title, EventInfo, EventStatus};
pub use formatting::{format_time, is_all_day};
pub use service::{detect_service as get_service_info, extract_url};

/// Requests calendar access from the user
pub fn request_access(store: &EKEventStore) -> bool {
    use crate::ffi::event_kit;
    event_kit::request_calendar_access(store)
}
