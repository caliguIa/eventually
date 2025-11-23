use objc2::rc::Retained;
use objc2_event_kit::{EKCalendar, EKEntityType, EKEvent, EKEventStore};
use objc2_foundation::{MainThreadMarker, NSDate};

pub fn init_event_store(mtm: MainThreadMarker) -> Retained<EKEventStore> {
    unsafe { EKEventStore::init(mtm.alloc::<EKEventStore>()) }
}

pub fn request_calendar_access(store: &EKEventStore) -> bool {
    use block2::StackBlock;
    use std::sync::mpsc::channel;

    let (tx, rx) = channel();
    unsafe {
        store.requestFullAccessToEventsWithCompletion(&StackBlock::new(
            move |granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
                let _ = tx.send(granted.as_bool());
            },
        ) as *const _ as *mut _);
    }

    rx.recv().unwrap_or(false)
}

pub fn fetch_events(store: &EKEventStore, start: &NSDate, end: &NSDate) -> Vec<Retained<EKEvent>> {
    unsafe {
        let calendars = store.calendarsForEntityType(EKEntityType::Event);
        let predicate =
            store.predicateForEventsWithStartDate_endDate_calendars(start, end, Some(&calendars));
        store.eventsMatchingPredicate(&predicate).to_vec()
    }
}

pub fn get_event_properties(
    event: &EKEvent,
) -> (
    Retained<NSDate>,
    Retained<NSDate>,
    Option<Retained<objc2_foundation::NSString>>,
    Retained<objc2_foundation::NSString>,
    Option<Retained<objc2_foundation::NSString>>,
    Option<Retained<EKCalendar>>,
    bool,
) {
    unsafe {
        (
            event.startDate(),
            event.endDate(),
            event.eventIdentifier(),
            event.title(),
            event.location(),
            event.calendar(),
            event.hasRecurrenceRules(),
        )
    }
}

pub fn get_calendar_color(calendar: &EKCalendar) -> (f64, f64, f64) {
    let color = unsafe { calendar.color() };
    (
        color.redComponent(),
        color.greenComponent(),
        color.blueComponent(),
    )
}
