use chrono::{Duration, Local};
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFont, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSRange, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::calendar::events::{
    extract_url, find_cur_or_next, format_time, get_service_info, is_all_day, EventInfo,
    EventStatus,
};
use crate::menu_delegate::MenuDelegate;

fn load_icon(name: &str) -> Option<Retained<NSImage>> {
    use crate::ffi::app_kit;

    let icon_data: &[u8] = match name {
        "calendar" => include_bytes!("../assets/icons/calendar.svg"),
        "circle-x" => include_bytes!("../assets/icons/circle-x.svg"),
        "google" => include_bytes!("../assets/icons/google.svg"),
        "slack" => include_bytes!("../assets/icons/slack.svg"),
        "teams" => include_bytes!("../assets/icons/teams.svg"),
        "video" => include_bytes!("../assets/icons/video.svg"),
        _ => return None,
    };

    let data = objc2_foundation::NSData::with_bytes(icon_data);
    let image = app_kit::init_image_from_data(&data)?;
    let size = objc2_foundation::NSSize::new(16.0, 16.0);
    app_kit::set_image_properties(&image, size, true);
    Some(image)
}

fn load_colored_icon(_name: &str, color: &NSColor) -> Option<Retained<NSImage>> {
    use crate::ffi::app_kit;

    let size = objc2_foundation::NSSize::new(16.0, 16.0);
    let image = app_kit::init_image_with_size(size);

    app_kit::lock_focus(&image);

    let circle_rect = objc2_foundation::NSRect::new(
        objc2_foundation::NSPoint::new(4.0, 4.0),
        objc2_foundation::NSSize::new(8.0, 8.0),
    );
    app_kit::draw_filled_circle(color, circle_rect);

    app_kit::unlock_focus(&image);

    Some(image)
}

pub fn build_menu(
    events: Vec<EventInfo>,
    delegate: &MenuDelegate,
    dismissed: &Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
) -> Retained<NSMenu> {
    use crate::ffi::app_kit;

    let foreground_color_attr = app_kit::get_foreground_color_attribute();
    let font_attr = app_kit::get_font_attribute();

    let menu = app_kit::init_menu(mtm, ns_string!(""));

    let current_or_next: Option<EventStatus> = {
        let dismissed_set = dismissed.lock().unwrap();
        find_cur_or_next(&events, &dismissed_set)
    };

    if let Some(ref event_status) = current_or_next {
        let event = event_status.event();
        if let Some(url) = extract_url(event.location.as_deref()) {
            let service_info = get_service_info(&url);
            let service_name = service_info.name;
            let service_icon_name = service_info.icon;
            let join_title = format!("Join {} Event", service_name);
            let join_item = app_kit::init_menu_item(
                mtm,
                &NSString::from_str(&join_title),
                Some(objc2::sel!(openURL:)),
                ns_string!(""),
            );
            if let Some(icon) = load_icon(service_icon_name) {
                join_item.setImage(Some(&icon));
            }
            app_kit::set_menu_item_target(&join_item, Some(delegate));
            app_kit::set_menu_item_represented_object(&join_item, Some(&*NSString::from_str(&url)));
            menu.addItem(&join_item);
        }

        let calendar_item = app_kit::init_menu_item(
            mtm,
            ns_string!("Open in Calendar"),
            Some(objc2::sel!(openEvent:)),
            ns_string!(""),
        );
        if let Some(icon) = load_icon("calendar") {
            calendar_item.setImage(Some(&icon));
        }
        app_kit::set_menu_item_target(&calendar_item, Some(delegate));
        let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
        app_kit::set_menu_item_represented_object(
            &calendar_item,
            Some(&*NSString::from_str(&open_data)),
        );
        menu.addItem(&calendar_item);

        let dismiss_item = app_kit::init_menu_item(
            mtm,
            ns_string!("Dismiss Event"),
            Some(objc2::sel!(dismissEvent:)),
            ns_string!(""),
        );
        if let Some(icon) = load_icon("circle-x") {
            dismiss_item.setImage(Some(&icon));
        }
        app_kit::set_menu_item_target(&dismiss_item, Some(delegate));
        app_kit::set_menu_item_represented_object(
            &dismiss_item,
            Some(&*NSString::from_str(&event.occurrence_key)),
        );
        menu.addItem(&dismiss_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));
    }

    if events.is_empty() {
        let item = app_kit::init_menu_item(mtm, ns_string!("No events"), None, ns_string!(""));
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

                let attr_string = app_kit::init_attributed_string(&header_ns_string);

                let bold_font = NSFont::boldSystemFontOfSize(0.0);
                let day_name_ns = NSString::from_str(day_name);
                let day_name_range = NSRange::new(0, day_name_ns.length());

                app_kit::add_attribute(&attr_string, font_attr, &**bold_font, day_name_range);

                let header_item =
                    app_kit::init_menu_item(mtm, ns_string!(""), None, ns_string!(""));
                app_kit::set_attributed_title(&header_item, &attr_string);
                header_item.setEnabled(false);
                menu.addItem(&header_item);

                for event in day_events {
                    let is_dismissed = dismissed.lock().unwrap().contains(&event.occurrence_key);
                    let is_all_day = is_all_day(&event.start, &event.end);
                    let time_prefix = if is_all_day {
                        "All day:".to_string()
                    } else {
                        let start_time = format_time(&event.start);
                        let end_time = format_time(&event.end);
                        format!("{} - {}", start_time, end_time)
                    };

                    let item_title = format!("{} {}", time_prefix, event.title);
                    let item_ns_string = NSString::from_str(&item_title);

                    let attr_string = app_kit::init_attributed_string(&item_ns_string);

                    let is_current_or_next = current_or_next
                        .as_ref()
                        .map(|status| status.event().occurrence_key == event.occurrence_key)
                        .unwrap_or(false);

                    if is_current_or_next {
                        let bold_font = NSFont::boldSystemFontOfSize(0.0);
                        let full_range = NSRange::new(0, item_ns_string.length());
                        app_kit::add_attribute(&attr_string, font_attr, &**bold_font, full_range);
                    }

                    if !is_all_day {
                        let start_time_len = format_time(&event.start).chars().count();
                        let secondary_color = NSColor::secondaryLabelColor();
                        let dash_and_end_start = start_time_len + 1;
                        let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
                        let end_time_range =
                            NSRange::new(dash_and_end_start, end_time_with_dash_len);

                        app_kit::add_attribute(
                            &attr_string,
                            foreground_color_attr,
                            &**secondary_color,
                            end_time_range,
                        );
                    }

                    let is_past = event.end < now || is_dismissed;
                    if is_past {
                        let secondary_color = NSColor::secondaryLabelColor();
                        let full_range = NSRange::new(0, item_ns_string.length());

                        app_kit::add_attribute(
                            &attr_string,
                            foreground_color_attr,
                            &**secondary_color,
                            full_range,
                        );

                        if !is_all_day {
                            let start_time_len = format_time(&event.start).chars().count();
                            let tertiary_color = NSColor::tertiaryLabelColor();
                            let dash_and_end_start = start_time_len + 1;
                            let end_time_with_dash_len =
                                2 + format_time(&event.end).chars().count();
                            let end_time_range =
                                NSRange::new(dash_and_end_start, end_time_with_dash_len);

                            app_kit::add_attribute(
                                &attr_string,
                                foreground_color_attr,
                                &**tertiary_color,
                                end_time_range,
                            );
                        }
                    }

                    let item = app_kit::init_menu_item(
                        mtm,
                        ns_string!(""),
                        Some(objc2::sel!(openEvent:)),
                        ns_string!(""),
                    );
                    app_kit::set_attributed_title(&item, &attr_string);

                    let calendar_color = NSColor::colorWithSRGBRed_green_blue_alpha(
                        event.calendar_color.0,
                        event.calendar_color.1,
                        event.calendar_color.2,
                        1.0,
                    );
                    if let Some(circle_icon) = load_colored_icon("circle", &calendar_color) {
                        item.setImage(Some(&circle_icon));
                    }

                    app_kit::set_menu_item_target(&item, Some(delegate));
                    let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
                    app_kit::set_menu_item_represented_object(
                        &item,
                        Some(&*NSString::from_str(&open_data)),
                    );

                    menu.addItem(&item);
                }

                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
        }
    }

    let quit_item = app_kit::init_menu_item(
        mtm,
        ns_string!("Quit"),
        Some(objc2::sel!(terminate:)),
        ns_string!("q"),
    );
    menu.addItem(&quit_item);

    menu
}
