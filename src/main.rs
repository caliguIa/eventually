use chrono::{DateTime, Duration, Local, Timelike};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength, NSApplication, NSApplicationActivationPolicy, NSWorkspace, NSColor, NSFont};
use objc2_event_kit::{EKEventStore, EKEntityType};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSString, NSURL, NSRange};
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
        let block = StackBlock::new(move |granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
            let _ = tx.send(granted.as_bool());
        });
        let block_ptr: *mut _ = &block as *const _ as *mut _;
        store.requestFullAccessToEventsWithCompletion(block_ptr);
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

fn is_all_day_event(start: &DateTime<Local>, end: &DateTime<Local>) -> bool {
    start.time().num_seconds_from_midnight() == 0 &&
    end.time().num_seconds_from_midnight() == 86399
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

            return format!("{} in {}", title, time_str);
        }
    }

    if !today_events.is_empty() {
        "No more events today".to_string()
    } else {
        "No events today".to_string()
    }
}

fn find_current_or_next_event<'a>(events: &'a [EventInfo]) -> Option<&'a EventInfo> {
    let now = Local::now();
    
    // Check for current event
    for event in events {
        if event.start <= now && now <= event.end {
            return Some(event);
        }
    }
    
    // Find next future event
    events.iter().find(|e| e.start > now)
}

fn build_menu(events: Vec<EventInfo>, delegate: &MenuDelegate, mtm: MainThreadMarker) -> Retained<NSMenu> {
    unsafe {
        extern "C" {
            static NSForegroundColorAttributeName: &'static AnyObject;
            static NSFontAttributeName: &'static AnyObject;
        }

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
            let day_after_tomorrow = today + Duration::days(2);
            let three_days_out = today + Duration::days(3);
            
            let current_or_next = find_current_or_next_event(&events);

            let groups = [
                (
                    today,
                    "Today".to_string(),
                    format!("{} {}", today.format("%d"), today.format("%b")),
                ),
                (
                    tomorrow,
                    "Tomorrow".to_string(),
                    format!("{} {}", tomorrow.format("%d"), tomorrow.format("%b")),
                ),
                (
                    day_after_tomorrow,
                    day_after_tomorrow.format("%A").to_string(),
                    format!("{} {}", day_after_tomorrow.format("%d"), day_after_tomorrow.format("%b")),
                ),
                (
                    three_days_out,
                    three_days_out.format("%A").to_string(),
                    format!("{} {}", three_days_out.format("%d"), three_days_out.format("%b")),
                ),
            ];

            for (date, day_name, date_str) in &groups {
                let day_events: Vec<_> = events
                    .iter()
                    .filter(|e| e.start.date_naive() == *date)
                    .collect();

                if !day_events.is_empty() {
                    let header_text = format!("{}, {}", day_name, date_str);
                    let header_ns_string = NSString::from_str(&header_text);
                    
                    let attr_string: Retained<AnyObject> = msg_send_id![
                        msg_send_id![objc2::class!(NSMutableAttributedString), alloc],
                        initWithString: &*header_ns_string
                    ];
                    
                    let bold_font = NSFont::boldSystemFontOfSize(0.0);
                    let day_name_ns = NSString::from_str(day_name);
                    let day_name_range = NSRange::new(0, day_name_ns.length());
                    
                    let _: () = msg_send![
                        &*attr_string,
                        addAttribute: NSFontAttributeName,
                        value: &**bold_font,
                        range: day_name_range
                    ];
                    
                    let header_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                        mtm.alloc(),
                        ns_string!(""),
                        None,
                        ns_string!(""),
                    );
                    let _: () = msg_send![&*header_item, setAttributedTitle: &*attr_string];
                    header_item.setEnabled(false);
                    menu.addItem(&header_item);

                    for event in day_events {
                        let is_all_day = is_all_day_event(&event.start, &event.end);
                        let time_prefix = if is_all_day {
                            "All day:".to_string()
                        } else {
                            let start_time = format_time(&event.start);
                            let end_time = format_time(&event.end);
                            format!("{} - {}", start_time, end_time)
                        };
                        
                        let item_title = format!("  {} {}", time_prefix, event.title);
                        let item_ns_string = NSString::from_str(&item_title);
                        
                        let attr_string: Retained<AnyObject> = msg_send_id![
                            msg_send_id![objc2::class!(NSMutableAttributedString), alloc],
                            initWithString: &*item_ns_string
                        ];
                        
                        // Check if this is the current or next event
                        let is_current_or_next = current_or_next
                            .map(|e| e.event_id == event.event_id)
                            .unwrap_or(false);
                        
                        if is_current_or_next {
                            // Apply bold font to entire text
                            let bold_font = NSFont::boldSystemFontOfSize(0.0);
                            let full_range = NSRange::new(0, item_ns_string.length());
                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSFontAttributeName,
                                value: &**bold_font,
                                range: full_range
                            ];
                        }
                        
                        if !is_all_day {
                            let start_time_len = format_time(&event.start).chars().count();
                            let secondary_color = NSColor::secondaryLabelColor();
                            let dash_and_end_start = 2 + start_time_len + 1;
                            let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
                            let end_time_range = NSRange::new(dash_and_end_start, end_time_with_dash_len);
                            
                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSForegroundColorAttributeName,
                                value: &**secondary_color,
                                range: end_time_range
                            ];
                        }
                        
                        let is_past = event.end < now;
                        if is_past {
                            let tertiary_color = NSColor::tertiaryLabelColor();
                            let full_range = NSRange::new(0, item_ns_string.length());
                            
                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSForegroundColorAttributeName,
                                value: &**tertiary_color,
                                range: full_range
                            ];
                        }
                        
                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            ns_string!(""),
                            Some(objc2::sel!(openEvent:)),
                            ns_string!(""),
                        );
                        let _: () = msg_send![&*item, setAttributedTitle: &*attr_string];
                        
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
        
        if !request_calendar_access(&event_store) {
            eprintln!("Calendar access denied. Please grant access in System Settings > Privacy & Security > Calendars");
            return;
        }
        
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
