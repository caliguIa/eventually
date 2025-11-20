use chrono::{Duration, Local};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, msg_send_id, ClassType};
use objc2_app_kit::{NSColor, NSFont, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSRange, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::delegate::MenuDelegate;
use crate::events::{
    extract_url_from_location, find_current_or_next_event, format_time, get_service_icon_from_url,
    get_service_name_from_url, is_all_day_event, EventInfo,
};

fn load_icon(name: &str) -> Option<Retained<NSImage>> {
    unsafe {
        let path = format!("assets/icons/{}.svg", name);
        let path_ns = NSString::from_str(&path);
        if let Some(image) = NSImage::initWithContentsOfFile(NSImage::alloc(), &path_ns) {
            let size = objc2_foundation::NSSize::new(18.0, 18.0);
            image.setSize(size);
            Some(image)
        } else {
            None
        }
    }
}

pub fn build_menu(
    events: Vec<EventInfo>,
    delegate: &MenuDelegate,
    dismissed: &Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
) -> Retained<NSMenu> {
    unsafe {
        extern "C" {
            static NSForegroundColorAttributeName: &'static AnyObject;
            static NSFontAttributeName: &'static AnyObject;
        }

        let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!(""));

        let current_or_next = {
            let dismissed_set = dismissed.lock().unwrap();
            find_current_or_next_event(&events, &dismissed_set)
        };

        if let Some(event) = current_or_next {
            if let Some(url) = extract_url_from_location(&event.location) {
                let service_name = get_service_name_from_url(&url);
                let service_icon_name = get_service_icon_from_url(&url);
                let join_title = format!("Join {} Event", service_name);
                let join_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(&join_title),
                    Some(objc2::sel!(openURL:)),
                    ns_string!(""),
                );
                if let Some(icon) = load_icon(service_icon_name) {
                    join_item.setImage(Some(&icon));
                }
                join_item.setTarget(Some(delegate));
                join_item.setRepresentedObject(Some(&*NSString::from_str(&url)));
                menu.addItem(&join_item);
            }

            let calendar_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                ns_string!("Open in Calendar"),
                Some(objc2::sel!(openEvent:)),
                ns_string!(""),
            );
            if let Some(icon) = load_icon("calendar") {
                calendar_item.setImage(Some(&icon));
            }
            calendar_item.setTarget(Some(delegate));
            let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
            calendar_item.setRepresentedObject(Some(&*NSString::from_str(&open_data)));
            menu.addItem(&calendar_item);

            let dismiss_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                ns_string!("Dismiss Event"),
                Some(objc2::sel!(dismissEvent:)),
                ns_string!(""),
            );
            if let Some(icon) = load_icon("circle-x") {
                dismiss_item.setImage(Some(&icon));
            }
            dismiss_item.setTarget(Some(delegate));
            dismiss_item.setRepresentedObject(Some(&*NSString::from_str(&event.occurrence_key)));
            menu.addItem(&dismiss_item);

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
                    format!(
                        "{} {}",
                        day_after_tomorrow.format("%d"),
                        day_after_tomorrow.format("%b")
                    ),
                ),
                (
                    three_days_out,
                    three_days_out.format("%A").to_string(),
                    format!(
                        "{} {}",
                        three_days_out.format("%d"),
                        three_days_out.format("%b")
                    ),
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
                        let is_dismissed =
                            dismissed.lock().unwrap().contains(&event.occurrence_key);
                        let is_all_day = is_all_day_event(&event.start, &event.end);
                        let time_prefix = if is_all_day {
                            "All day:".to_string()
                        } else {
                            let start_time = format_time(&event.start);
                            let end_time = format_time(&event.end);
                            format!("{} - {}", start_time, end_time)
                        };

                        let item_title = format!("{} {}", time_prefix, event.title);
                        let item_ns_string = NSString::from_str(&item_title);

                        let attr_string: Retained<AnyObject> = msg_send_id![
                            msg_send_id![objc2::class!(NSMutableAttributedString), alloc],
                            initWithString: &*item_ns_string
                        ];

                        let is_current_or_next = current_or_next
                            .map(|e| e.occurrence_key == event.occurrence_key)
                            .unwrap_or(false);

                        if is_current_or_next {
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
                            let dash_and_end_start = start_time_len + 1;
                            let end_time_with_dash_len =
                                2 + format_time(&event.end).chars().count();
                            let end_time_range =
                                NSRange::new(dash_and_end_start, end_time_with_dash_len);

                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSForegroundColorAttributeName,
                                value: &**secondary_color,
                                range: end_time_range
                            ];
                        }

                        let is_past = event.end < now || is_dismissed;
                        if is_past {
                            let secondary_color = NSColor::secondaryLabelColor();
                            let full_range = NSRange::new(0, item_ns_string.length());

                            let _: () = msg_send![
                                &*attr_string,
                                addAttribute: NSForegroundColorAttributeName,
                                value: &**secondary_color,
                                range: full_range
                            ];

                            if !is_all_day {
                                let start_time_len = format_time(&event.start).chars().count();
                                let tertiary_color = NSColor::tertiaryLabelColor();
                                let dash_and_end_start = start_time_len + 1;
                                let end_time_with_dash_len =
                                    2 + format_time(&event.end).chars().count();
                                let end_time_range =
                                    NSRange::new(dash_and_end_start, end_time_with_dash_len);

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

                        if let Some(circle_icon) = load_icon("circle") {
                            item.setImage(Some(&circle_icon));
                        }

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
