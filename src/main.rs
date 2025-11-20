use chrono::{DateTime, Duration, Local, Timelike};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength, NSApplication, NSApplicationActivationPolicy, NSWorkspace, NSColor, NSFont};
use objc2_event_kit::{EKEventStore, EKEntityType};
use objc2_foundation::{ns_string, MainThreadMarker, NSObject, NSString, NSURL, NSRange};
use block2::StackBlock;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

const MAX_TITLE_LENGTH: usize = 30;

#[derive(Clone)]
struct EventInfo {
    title: String,
    start: DateTime<Local>,
    end: DateTime<Local>,
    event_id: String,
    occurrence_key: String,
    has_recurrence: bool,
    location: Option<String>,
}

struct MenuDelegateIvars {
    dismissed_events: Arc<Mutex<HashSet<String>>>,
}

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
                    let data = (*ns_string).to_string();
                    
                    // Data is "event_id|||has_recurrence"
                    let parts: Vec<&str> = data.split("|||").collect();
                    let event_id = parts[0];
                    let has_recurrence = parts.get(1).map(|s| *s == "true").unwrap_or(false);
                    
                    let url_string = if has_recurrence {
                        // For recurring events, just open Calendar to today
                        // (no way to directly open specific occurrence)
                        "ical://".to_string()
                    } else {
                        format!("ical://ekevent/{}", event_id)
                    };
                    
                    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&url_string)) {
                        NSWorkspace::sharedWorkspace().openURL(&url);
                    }
                }
            }
        }
        
        #[method(openURL:)]
        fn open_url(&self, sender: &NSMenuItem) {
            unsafe {
                if let Some(obj) = sender.representedObject() {
                    let ns_string: *const NSString = Retained::as_ptr(&obj).cast();
                    let url_string = (*ns_string).to_string();
                    
                    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&url_string)) {
                        NSWorkspace::sharedWorkspace().openURL(&url);
                    }
                }
            }
        }
        
        #[method(dismissEvent:)]
        fn dismiss_event(&self, sender: &NSMenuItem) {
            unsafe {
                if let Some(obj) = sender.representedObject() {
                    let ns_string: *const NSString = Retained::as_ptr(&obj).cast();
                    let event_id_string = (*ns_string).to_string();
                    
                    if let Ok(mut dismissed) = self.ivars().dismissed_events.lock() {
                        dismissed.insert(event_id_string);
                    }
                }
            }
        }
    }
);

impl MenuDelegate {
    fn new(mtm: MainThreadMarker, dismissed_events: Arc<Mutex<HashSet<String>>>) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MenuDelegateIvars { dismissed_events });
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
            let occurrence_key = format!("{}:{}", event_id_str, start_timestamp as i64);
            
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

fn format_time(dt: &DateTime<Local>) -> String {
    format!("{:02}:{:02}", dt.hour(), dt.minute())
}

fn is_all_day_event(start: &DateTime<Local>, end: &DateTime<Local>) -> bool {
    start.time().num_seconds_from_midnight() == 0 &&
    end.time().num_seconds_from_midnight() == 86399
}

fn extract_url_from_location(location: &Option<String>) -> Option<String> {
    location.as_ref().and_then(|loc| {
        // Check if location looks like a URL
        if loc.starts_with("http://") || loc.starts_with("https://") {
            Some(loc.clone())
        } else {
            None
        }
    })
}

fn get_service_name_from_url(url: &str) -> String {
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

fn find_current_or_next_event<'a>(events: &'a [EventInfo], dismissed: &HashSet<String>) -> Option<&'a EventInfo> {
    let now = Local::now();
    let today = now.date_naive();
    
    // Filter to only today's non-dismissed events
    let today_events: Vec<_> = events.iter()
        .filter(|e| e.start.date_naive() == today && !dismissed.contains(&e.occurrence_key))
        .collect();
    
    // Check for current event
    for event in &today_events {
        if event.start <= now && now <= event.end {
            return Some(event);
        }
    }
    
    // Find next future event
    today_events.into_iter().find(|e| e.start > now)
}

fn get_status_bar_title(events: &[EventInfo], dismissed: &HashSet<String>) -> String {
    let now = Local::now();
    let today = now.date_naive();
    
    let today_events: Vec<_> = events.iter()
        .filter(|e| e.start.date_naive() == today && !dismissed.contains(&e.occurrence_key))
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

fn build_menu(events: Vec<EventInfo>, delegate: &MenuDelegate, dismissed: &Arc<Mutex<HashSet<String>>>, mtm: MainThreadMarker) -> Retained<NSMenu> {
    unsafe {
        extern "C" {
            static NSForegroundColorAttributeName: &'static AnyObject;
            static NSFontAttributeName: &'static AnyObject;
        }

        let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!(""));
        
        // Add top menu items if there's a current or upcoming event today
        let current_or_next = {
            let dismissed_set = dismissed.lock().unwrap();
            find_current_or_next_event(&events, &dismissed_set)
        };
        
        if let Some(event) = current_or_next {
            // 1. Join X event (if URL present)
            if let Some(url) = extract_url_from_location(&event.location) {
                let service_name = get_service_name_from_url(&url);
                let join_title = format!("Join {} Event", service_name);
                let join_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(&join_title),
                    Some(objc2::sel!(openURL:)),
                    ns_string!(""),
                );
                join_item.setTarget(Some(delegate));
                join_item.setRepresentedObject(Some(&*NSString::from_str(&url)));
                menu.addItem(&join_item);
            }
            
            // 2. Open in Calendar
            let calendar_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                ns_string!("Open in Calendar"),
                Some(objc2::sel!(openEvent:)),
                ns_string!(""),
            );
            calendar_item.setTarget(Some(delegate));
            let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
            calendar_item.setRepresentedObject(Some(&*NSString::from_str(&open_data)));
            menu.addItem(&calendar_item);
            
            // 3. Dismiss Event
            let dismiss_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                ns_string!("Dismiss Event"),
                Some(objc2::sel!(dismissEvent:)),
                ns_string!(""),
            );
            dismiss_item.setTarget(Some(delegate));
            dismiss_item.setRepresentedObject(Some(&*NSString::from_str(&event.occurrence_key)));
            menu.addItem(&dismiss_item);
            
            // Add separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }

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
                        let is_dismissed = dismissed.lock().unwrap().contains(&event.occurrence_key);
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
                            .map(|e| e.occurrence_key == event.occurrence_key)
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
                        
                        let is_past = event.end < now || is_dismissed;
                        if is_past {
                            // Apply secondary color to the entire text for past events
                            let secondary_color = NSColor::secondaryLabelColor();
                            let full_range = NSRange::new(0, item_ns_string.length());
                            
                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSForegroundColorAttributeName,
                                value: &**secondary_color,
                                range: full_range
                            ];
                            
                            // Then apply tertiary (darker) color to the "- XX:XX" part if not all-day
                            if !is_all_day {
                                let start_time_len = format_time(&event.start).chars().count();
                                let tertiary_color = NSColor::tertiaryLabelColor();
                                let dash_and_end_start = 2 + start_time_len + 1;
                                let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
                                let end_time_range = NSRange::new(dash_and_end_start, end_time_with_dash_len);
                                
                                let _: () = msg_send![
                                    &*attr_string,
                                    addAttribute: NSForegroundColorAttributeName,
                                    value: &**tertiary_color,
                                    range: end_time_range
                                ];
                            }
                        }
                        
                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            mtm.alloc(),
                            ns_string!(""),
                            Some(objc2::sel!(openEvent:)),
                            ns_string!(""),
                        );
                        let _: () = msg_send![&*item, setAttributedTitle: &*attr_string];
                        
                        item.setTarget(Some(delegate));
                        let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
                        item.setRepresentedObject(Some(&*NSString::from_str(&open_data)));

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
        
        let dismissed_events = Arc::new(Mutex::new(HashSet::new()));
        let delegate = MenuDelegate::new(mtm, dismissed_events.clone());

        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

        if let Some(button) = status_item.button(mtm) {
            let dismissed_set = dismissed_events.lock().unwrap();
            let title = get_status_bar_title(&events, &dismissed_set);
            button.setTitle(&NSString::from_str(&title));
        }

        let menu = build_menu(events, &delegate, &dismissed_events, mtm);
        status_item.setMenu(Some(&menu));

        app.run();
    }
}
