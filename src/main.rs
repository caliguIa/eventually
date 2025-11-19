use chrono::{DateTime, Duration, Local, Timelike};
use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength, NSApplication, NSApplicationActivationPolicy, NSWorkspace};
use objc2_event_kit::{EKEventStore, EKEntityType};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSString, NSURL};
use block2::StackBlock;

const MAX_TITLE_LENGTH: usize = 30;

#[derive(Clone)]
struct EventInfo {
    title: String,
    start: DateTime<Local>,
    end: DateTime<Local>,
    event_id: String,
}

struct MenuDelegateIvars {}

declare_class!(
    struct MenuDelegate;

    unsafe impl ClassType for MenuDelegate {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "MenuDelegate";
    }

    impl DeclaredClass for MenuDelegate {
        type Ivars = MenuDelegateIvars;
    }

    unsafe impl MenuDelegate {
        #[method(openEvent:)]
        fn open_event(&self, sender: &NSMenuItem) {
            unsafe {
                if let Some(obj) = sender.representedObject() {
                    let ns_string: *const NSString = Retained::as_ptr(&obj).cast();
                    let event_id_string = (*ns_string).to_string();
                    
                    let url_string = format!("ical://ekevent/{}", event_id_string);
                    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&url_string)) {
                        NSWorkspace::sharedWorkspace().openURL(&url);
                    }
                }
            }
        }
    }
);

impl MenuDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MenuDelegateIvars {});
        unsafe { msg_send_id![super(this), init] }
    }
}

fn request_calendar_access(store: &EKEventStore) -> bool {
    use std::sync::mpsc::channel;
    let (tx, rx) = channel();
    
    unsafe {
        let block = StackBlock::new(|granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
            let _ = tx.send(granted.as_bool());
        });
        store.requestFullAccessToEventsWithCompletion(&block as *const _ as *mut _);
    }
    
    rx.recv().unwrap_or(false)
}

fn fetch_events(store: &EKEventStore) -> Vec<EventInfo> {
    let now = Local::now();
    let start_of_today = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    let end_of_three_days = (start_of_today + Duration::days(3))
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();

    let start_ns_date = unsafe { 
        objc2_foundation::NSDate::dateWithTimeIntervalSince1970(start_of_today.timestamp() as f64)
    };
    let end_ns_date = unsafe { 
        objc2_foundation::NSDate::dateWithTimeIntervalSince1970(end_of_three_days.timestamp() as f64)
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

            let start_timestamp = start_date.timeIntervalSince1970();
            let end_timestamp = end_date.timeIntervalSince1970();

            let start_dt = DateTime::from_timestamp(start_timestamp as i64, 0)
                .unwrap()
                .with_timezone(&Local);
            let end_dt = DateTime::from_timestamp(end_timestamp as i64, 0)
                .unwrap()
                .with_timezone(&Local);

            event_list.push(EventInfo {
                title: title.to_string(),
                start: start_dt,
                end: end_dt,
                event_id: event_id.map(|id| id.to_string()).unwrap_or_default(),
            });
        }
        
        event_list.sort_by_key(|e| e.start);
        event_list
    }
}

fn format_time(dt: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

fn get_status_bar_title(events: &[EventInfo]) -> String {
    let now = Local::now();
    let today = now.date_naive();
    
    let today_events: Vec<_> = events.iter()
        .filter(|e| e.start.date_naive() == today)
        .collect();

    for event in &today_events {
        if event.start <= now && now <= event.end {
            let remaining = event.end.signed_duration_since(now);
            let mins = remaining.num_minutes();

            let time_str = if mins > 60 {
                format!("{} hrs", mins / 60)
            } else {
                format!("{} mins", mins)
            };

            let mut title = event.title.clone();
            let max_event_len = MAX_TITLE_LENGTH.saturating_sub(time_str.len() + 6);
            if title.len() > max_event_len {
                title.truncate(max_event_len.saturating_sub(1));
                title.push('…');
            }

            return format!("{} left {}", time_str, title);
        }
    }

    for event in &today_events {
        if event.start > now {
            let until = event.start.signed_duration_since(now);
            let mins = until.num_minutes();

            let time_str = if mins > 60 {
                format!("{} hr", mins / 60)
            } else {
                format!("{} mins", mins)
            };

            let mut title = event.title.clone();
            let max_event_len = MAX_TITLE_LENGTH.saturating_sub(time_str.len() + 8);
            if title.len() > max_event_len {
                title.truncate(max_event_len.saturating_sub(1));
                title.push('…');
            }

            return format!("{} until {}", time_str, title);
        }
    }

    if !today_events.is_empty() {
        "No more events today".to_string()
    } else {
        "No events today".to_string()
    }
}

fn build_menu(events: Vec<EventInfo>, delegate: &MenuDelegate, mtm: MainThreadMarker) -> Retained<NSMenu> {
    unsafe {
        let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!(""));

        if events.is_empty() {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                ns_string!("No events"),
                None,
                ns_string!(""),
            );
            item.setEnabled(false);
            menu.addItem(&item);
        } else {
            let now = Local::now();
            let today = now.date_naive();
            let tomorrow = today + Duration::days(1);
            let day_after = today + Duration::days(2);

            let groups = [
                (
                    today,
                    format!("Today, {} {}", today.format("%d"), today.format("%b")),
                ),
                (
                    tomorrow,
                    format!("Tomorrow, {} {}", tomorrow.format("%d"), tomorrow.format("%b")),
                ),
                (
                    day_after,
                    format!(
                        "{}, {} {}",
                        day_after.format("%A"),
                        day_after.format("%d"),
                        day_after.format("%b")
                    ),
                ),
            ];

            for (date, header) in &groups {
                let day_events: Vec<_> = events
                    .iter()
                    .filter(|e| e.start.date_naive() == *date)
                    .collect();

                if !day_events.is_empty() {
                    let header_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                        mtm.alloc(),
                        &NSString::from_str(header),
                        None,
                        ns_string!(""),
                    );
                    header_item.setEnabled(false);
                    menu.addItem(&header_item);

                    for event in day_events {
                        let time_range = format!(
                            "{} - {}",
                            format_time(&event.start),
                            format_time(&event.end)
                        );
                        let item_title = format!("  {} {}", time_range, event.title);

                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            &NSString::from_str(&item_title),
                            Some(objc2::sel!(openEvent:)),
                            ns_string!(""),
                        );
                        
                        item.setTarget(Some(delegate));
                        item.setRepresentedObject(Some(&*NSString::from_str(&event.event_id)));

                        menu.addItem(&item);
                    }

                    menu.addItem(&NSMenuItem::separatorItem(mtm));
                }
            }
        }

        let quit_item = NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            ns_string!("Quit"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
        menu.addItem(&quit_item);

        menu
    }
}

fn main() {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let event_store = EKEventStore::init(EKEventStore::alloc());
        
        println!("Requesting calendar access...");
        if !request_calendar_access(&event_store) {
            eprintln!("Calendar access denied. Please grant access in System Settings > Privacy & Security > Calendars");
            return;
        }
        
        println!("Fetching events...");
        let events = fetch_events(&event_store);

        let delegate = MenuDelegate::new(mtm);

        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

        if let Some(button) = status_item.button(mtm) {
            let title = get_status_bar_title(&events);
            button.setTitle(&NSString::from_str(&title));
        }

        let menu = build_menu(events, &delegate, mtm);
        status_item.setMenu(Some(&menu));

        app.run();
    }
}
