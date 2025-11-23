use chrono::{Duration, Local};
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSRange, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::calendar::events::{
    extract_url, find_cur_or_next, format_time, get_service_info, is_all_day, EventInfo,
    EventStatus,
};
use crate::ffi::app_kit;

use super::delegate::MenuDelegate;
use super::formatting;
use super::icons;

/// Builds the complete status bar menu with events and quick actions
pub fn build_menu(
    events: Vec<EventInfo>,
    delegate: &MenuDelegate,
    dismissed: &Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
) -> Retained<NSMenu> {
    let menu = app_kit::init_menu(mtm, ns_string!(""));

    let current_or_next: Option<EventStatus> = {
        let dismissed_set = dismissed.lock().unwrap();
        find_cur_or_next(&events, &dismissed_set)
    };

    // Add quick action items for current/next event
    if let Some(ref event_status) = current_or_next {
        add_quick_actions(&menu, event_status, delegate, mtm);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
    }

    // Add event list grouped by day
    if events.is_empty() {
        add_empty_state(&menu, mtm);
    } else {
        add_event_groups(&menu, &events, &current_or_next, dismissed, delegate, mtm);
    }

    // Add quit item
    add_quit_item(&menu, mtm);

    menu
}

/// Adds quick action items (join video, open calendar, dismiss) for the current/next event
fn add_quick_actions(
    menu: &NSMenu,
    event_status: &EventStatus,
    delegate: &MenuDelegate,
    mtm: MainThreadMarker,
) {
    let event = event_status.event();

    // Add "Join <Service> Event" if there's a video URL
    if let Some(url) = extract_url(event.location.as_deref()) {
        add_join_video_item(menu, &url, delegate, mtm);
    }

    // Add "Open in Calendar"
    add_open_calendar_item(menu, event, delegate, mtm);

    // Add "Dismiss Event"
    add_dismiss_item(menu, event, delegate, mtm);
}

/// Adds "Join <Service> Event" menu item
fn add_join_video_item(menu: &NSMenu, url: &str, delegate: &MenuDelegate, mtm: MainThreadMarker) {
    let service_info = get_service_info(url);
    let join_title = format!("Join {} Event", service_info.name);
    let join_item = app_kit::init_menu_item(
        mtm,
        &NSString::from_str(&join_title),
        Some(objc2::sel!(openURL:)),
        ns_string!(""),
    );
    if let Some(icon) = icons::load_icon(service_info.icon) {
        join_item.setImage(Some(&icon));
    }
    app_kit::set_menu_item_target(&join_item, Some(delegate));
    app_kit::set_menu_item_represented_object(&join_item, Some(&*NSString::from_str(url)));
    menu.addItem(&join_item);
}

/// Adds "Open in Calendar" menu item
fn add_open_calendar_item(
    menu: &NSMenu,
    event: &EventInfo,
    delegate: &MenuDelegate,
    mtm: MainThreadMarker,
) {
    let calendar_item = app_kit::init_menu_item(
        mtm,
        ns_string!("Open in Calendar"),
        Some(objc2::sel!(openEvent:)),
        ns_string!(""),
    );
    if let Some(icon) = icons::load_icon("calendar") {
        calendar_item.setImage(Some(&icon));
    }
    app_kit::set_menu_item_target(&calendar_item, Some(delegate));
    let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
    app_kit::set_menu_item_represented_object(
        &calendar_item,
        Some(&*NSString::from_str(&open_data)),
    );
    menu.addItem(&calendar_item);
}

/// Adds "Dismiss Event" menu item
fn add_dismiss_item(
    menu: &NSMenu,
    event: &EventInfo,
    delegate: &MenuDelegate,
    mtm: MainThreadMarker,
) {
    let dismiss_item = app_kit::init_menu_item(
        mtm,
        ns_string!("Dismiss Event"),
        Some(objc2::sel!(dismissEvent:)),
        ns_string!(""),
    );
    if let Some(icon) = icons::load_icon("circle-x") {
        dismiss_item.setImage(Some(&icon));
    }
    app_kit::set_menu_item_target(&dismiss_item, Some(delegate));
    app_kit::set_menu_item_represented_object(
        &dismiss_item,
        Some(&*NSString::from_str(&event.occurrence_key)),
    );
    menu.addItem(&dismiss_item);
}

/// Adds "No events" disabled menu item
fn add_empty_state(menu: &NSMenu, mtm: MainThreadMarker) {
    let item = app_kit::init_menu_item(mtm, ns_string!("No events"), None, ns_string!(""));
    item.setEnabled(false);
    menu.addItem(&item);
}

/// Adds event groups organized by day (Today, Tomorrow, etc.)
fn add_event_groups(
    menu: &NSMenu,
    events: &[EventInfo],
    current_or_next: &Option<EventStatus>,
    dismissed: &Arc<Mutex<HashSet<String>>>,
    delegate: &MenuDelegate,
    mtm: MainThreadMarker,
) {
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
            add_day_header(menu, day_name, date_str, mtm);

            for event in day_events {
                add_event_item(menu, event, current_or_next, dismissed, delegate, now, mtm);
            }

            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
    }
}

/// Adds a day header (e.g., "Today, 22 Nov")
fn add_day_header(menu: &NSMenu, day_name: &str, date_str: &str, mtm: MainThreadMarker) {
    let header_text = format!("{}, {}", day_name, date_str);
    let attr_string = formatting::create_attributed_string(&header_text);

    let day_name_ns = NSString::from_str(day_name);
    let day_name_range = NSRange::new(0, day_name_ns.length());
    formatting::apply_bold_font(&attr_string, day_name_range);

    let header_item = app_kit::init_menu_item(mtm, ns_string!(""), None, ns_string!(""));
    app_kit::set_attributed_title(&header_item, &attr_string);
    header_item.setEnabled(false);
    menu.addItem(&header_item);
}

/// Adds a single event menu item with styled text and calendar color dot
fn add_event_item(
    menu: &NSMenu,
    event: &EventInfo,
    current_or_next: &Option<EventStatus>,
    dismissed: &Arc<Mutex<HashSet<String>>>,
    delegate: &MenuDelegate,
    now: chrono::DateTime<Local>,
    mtm: MainThreadMarker,
) {
    let is_dismissed = dismissed.lock().unwrap().contains(&event.occurrence_key);
    let is_all_day = is_all_day(&event.start, &event.end);

    // Build title with time prefix
    let time_prefix = if is_all_day {
        "All day:".to_string()
    } else {
        let start_time = format_time(&event.start);
        let end_time = format_time(&event.end);
        format!("{} - {}", start_time, end_time)
    };

    let item_title = format!("{} {}", time_prefix, event.title);
    let attr_string = formatting::create_attributed_string(&item_title);

    // Check if this is the current or next event
    let is_current_or_next = current_or_next
        .as_ref()
        .map(|status| status.event().occurrence_key == event.occurrence_key)
        .unwrap_or(false);

    // Make current/next event bold
    if is_current_or_next {
        let full_range = NSRange::new(0, NSString::from_str(&item_title).length());
        formatting::apply_bold_font(&attr_string, full_range);
    }

    // Dim the end time for non-all-day events
    if !is_all_day {
        let start_time_len = format_time(&event.start).chars().count();
        let dash_and_end_start = start_time_len + 1;
        let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
        let end_time_range = NSRange::new(dash_and_end_start, end_time_with_dash_len);
        formatting::apply_secondary_color(&attr_string, end_time_range);
    }

    // Dim past or dismissed events
    let is_past = event.end < now || is_dismissed;
    if is_past {
        let full_range = NSRange::new(0, NSString::from_str(&item_title).length());
        formatting::apply_secondary_color(&attr_string, full_range);

        if !is_all_day {
            let start_time_len = format_time(&event.start).chars().count();
            let dash_and_end_start = start_time_len + 1;
            let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
            let end_time_range = NSRange::new(dash_and_end_start, end_time_with_dash_len);
            formatting::apply_tertiary_color(&attr_string, end_time_range);
        }
    }

    // Create menu item
    let item = app_kit::init_menu_item(
        mtm,
        ns_string!(""),
        Some(objc2::sel!(openEvent:)),
        ns_string!(""),
    );
    app_kit::set_attributed_title(&item, &attr_string);

    // Add calendar color dot
    let calendar_color = NSColor::colorWithSRGBRed_green_blue_alpha(
        event.calendar_color.0,
        event.calendar_color.1,
        event.calendar_color.2,
        1.0,
    );
    if let Some(circle_icon) = icons::load_colored_icon("circle", &calendar_color) {
        item.setImage(Some(&circle_icon));
    }

    // Set target and action
    app_kit::set_menu_item_target(&item, Some(delegate));
    let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
    app_kit::set_menu_item_represented_object(&item, Some(&*NSString::from_str(&open_data)));

    menu.addItem(&item);
}

/// Adds "Quit" menu item
fn add_quit_item(menu: &NSMenu, mtm: MainThreadMarker) {
    let quit_item = app_kit::init_menu_item(
        mtm,
        ns_string!("Quit"),
        Some(objc2::sel!(terminate:)),
        ns_string!("q"),
    );
    menu.addItem(&quit_item);
}
