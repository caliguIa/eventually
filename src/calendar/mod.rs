pub mod events;

use objc2_event_kit::EKEventStore;

pub fn request_access(store: &EKEventStore) -> bool {
    use crate::ffi::event_kit;
    event_kit::request_calendar_access(store)
}
