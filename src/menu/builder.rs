use chrono::{Duration, Local};
use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSRange, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::calendar::{extract_url, format_time, is_all_day, EventInfo, EventStatus, Icon, ServiceInfo};
use crate::ffi::app_kit;

use super::delegate::MenuDelegate;
use super::formatting;

pub struct MenuBuilder<'a> {
    events: Vec<EventInfo>,
    delegate: &'a MenuDelegate,
    dismissed: &'a Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
}

impl<'a> MenuBuilder<'a> {
    pub fn new(
        events: Vec<EventInfo>,
        delegate: &'a MenuDelegate,
        dismissed: &'a Arc<Mutex<HashSet<String>>>,
        mtm: MainThreadMarker,
    ) -> Self {
        Self {
            events,
            delegate,
            dismissed,
            mtm,
        }
    }

    pub fn build(self) -> Retained<NSMenu> {
        let menu = app_kit::init_menu(self.mtm, ns_string!(""));
        
        let collection = crate::calendar::EventCollection::from(self.events.clone());
        let current_or_next: Option<EventStatus> = match self.dismissed.lock() {
            Ok(dismissed_set) => collection.find_cur_or_next(&dismissed_set),
            Err(e) => {
                eprintln!("Error: Failed to acquire lock in build_menu: {}", e);
                None
            }
        };

        if let Some(ref event_status) = current_or_next {
            self.add_quick_actions(&menu, event_status);
            menu.addItem(&NSMenuItem::separatorItem(self.mtm));
        }

        if self.events.is_empty() {
            self.add_empty_state(&menu);
        } else {
            self.add_event_groups(&menu, &current_or_next);
        }

        self.add_quit_item(&menu);
        menu
    }

    fn add_quick_actions(&self, menu: &NSMenu, event_status: &EventStatus) {
        let event = event_status.event();
        if let Some(url) = extract_url(event.location.as_deref()) {
            self.add_join_video_item(menu, url);
        }
        self.add_open_calendar_item(menu, event);
        self.add_dismiss_item(menu, event);
    }

    fn add_join_video_item(&self, menu: &NSMenu, url: &str) {
        let service_info = ServiceInfo::from_url(url);
        let join_title = format!("Join {} Event", service_info.name());
        let join_item = app_kit::init_menu_item(
            self.mtm,
            &NSString::from_str(&join_title),
            Some(objc2::sel!(openURL:)),
            ns_string!(""),
        );
        if let Some(icon) = service_info.icon().load() {
            join_item.setImage(Some(&icon));
        }
        app_kit::set_menu_item_target(&join_item, Some(self.delegate));
        app_kit::set_menu_item_represented_object(&join_item, Some(&*NSString::from_str(url)));
        menu.addItem(&join_item);
    }

    fn add_open_calendar_item(&self, menu: &NSMenu, event: &EventInfo) {
        let calendar_item = app_kit::init_menu_item(
            self.mtm,
            ns_string!("Open in Calendar"),
            Some(objc2::sel!(openEvent:)),
            ns_string!(""),
        );
        if let Some(icon) = Icon::Calendar.load() {
            calendar_item.setImage(Some(&icon));
        }
        app_kit::set_menu_item_target(&calendar_item, Some(self.delegate));
        let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
        app_kit::set_menu_item_represented_object(
            &calendar_item,
            Some(&*NSString::from_str(&open_data)),
        );
        menu.addItem(&calendar_item);
    }

    fn add_dismiss_item(&self, menu: &NSMenu, event: &EventInfo) {
        let dismiss_item = app_kit::init_menu_item(
            self.mtm,
            ns_string!("Dismiss Event"),
            Some(objc2::sel!(dismissEvent:)),
            ns_string!(""),
        );
        if let Some(icon) = Icon::CircleX.load() {
            dismiss_item.setImage(Some(&icon));
        }
        app_kit::set_menu_item_target(&dismiss_item, Some(self.delegate));
        app_kit::set_menu_item_represented_object(
            &dismiss_item,
            Some(&*NSString::from_str(&event.occurrence_key)),
        );
        menu.addItem(&dismiss_item);
    }

    fn add_empty_state(&self, menu: &NSMenu) {
        let item = app_kit::init_menu_item(self.mtm, ns_string!("No events"), None, ns_string!(""));
        item.setEnabled(false);
        menu.addItem(&item);
    }

    fn add_event_groups(&self, menu: &NSMenu, current_or_next: &Option<EventStatus>) {
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
            let day_events: Vec<_> = self
                .events
                .iter()
                .filter(|e| e.start.date_naive() == *date)
                .collect();

            if !day_events.is_empty() {
                self.add_day_header(menu, day_name, date_str);

                for event in day_events {
                    self.add_event_item(menu, event, current_or_next, now);
                }

                menu.addItem(&NSMenuItem::separatorItem(self.mtm));
            }
        }
    }

    fn add_day_header(&self, menu: &NSMenu, day_name: &str, date_str: &str) {
        let header_text = format!("{}, {}", day_name, date_str);
        let attr_string = formatting::create_attributed_string(&header_text);

        let day_name_ns = NSString::from_str(day_name);
        let day_name_range = NSRange::new(0, day_name_ns.length());
        formatting::apply_bold_font(&attr_string, day_name_range);

        let header_item = app_kit::init_menu_item(self.mtm, ns_string!(""), None, ns_string!(""));
        app_kit::set_attributed_title(&header_item, &attr_string);
        header_item.setEnabled(false);
        menu.addItem(&header_item);
    }

    fn add_event_item(
        &self,
        menu: &NSMenu,
        event: &EventInfo,
        current_or_next: &Option<EventStatus>,
        now: chrono::DateTime<Local>,
    ) {
        let is_dismissed = self
            .dismissed
            .lock()
            .map(|set| set.contains(&event.occurrence_key))
            .unwrap_or_else(|e| {
                eprintln!("Error: Failed to check if event is dismissed: {}", e);
                false
            });
        let is_all_day = is_all_day(&event.start, &event.end);

        let time_prefix = if is_all_day {
            "All day:".to_string()
        } else {
            let start_time = format_time(&event.start);
            let end_time = format_time(&event.end);
            format!("{} - {}", start_time, end_time)
        };

        let item_title = format!("{} {}", time_prefix, event.title);
        let attr_string = formatting::create_attributed_string(&item_title);

        let is_current_or_next = current_or_next
            .as_ref()
            .map(|status| status.event().occurrence_key == event.occurrence_key)
            .unwrap_or(false);

        if is_current_or_next {
            let full_range = NSRange::new(0, NSString::from_str(&item_title).length());
            formatting::apply_bold_font(&attr_string, full_range);
        }

        if !is_all_day {
            let start_time_len = format_time(&event.start).chars().count();
            let dash_and_end_start = start_time_len + 1;
            let end_time_with_dash_len = 2 + format_time(&event.end).chars().count();
            let end_time_range = NSRange::new(dash_and_end_start, end_time_with_dash_len);
            formatting::apply_secondary_color(&attr_string, end_time_range);
        }

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

        let item = app_kit::init_menu_item(
            self.mtm,
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
        if let Some(circle_icon) = Icon::load_colored(&calendar_color) {
            item.setImage(Some(&circle_icon));
        }

        app_kit::set_menu_item_target(&item, Some(self.delegate));
        let open_data = format!("{}|||{}", event.event_id, event.has_recurrence);
        app_kit::set_menu_item_represented_object(&item, Some(&*NSString::from_str(&open_data)));

        menu.addItem(&item);
    }

    fn add_quit_item(&self, menu: &NSMenu) {
        let quit_item = app_kit::init_menu_item(
            self.mtm,
            ns_string!("Quit"),
            Some(objc2::sel!(terminate:)),
            ns_string!("q"),
        );
        menu.addItem(&quit_item);
    }
}
