mod args;
mod delegate;
mod events;
mod menu;
mod service;

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSStatusBar, NSVariableStatusItemLength,
};
use objc2_event_kit::EKEventStore;
use objc2_foundation::{ns_string, MainThreadMarker, NSNotificationCenter, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use args::handle_args;
use delegate::MenuDelegate;
use events::{fetch_events, get_status_bar_title, request_calendar_access};
use menu::build_menu;

fn main() {
    if let Some(result) = handle_args() {
        if let Err(e) = result {
            eprintln!("Command failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    unsafe {
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let event_store = EKEventStore::init(mtm.alloc::<EKEventStore>());

        if !request_calendar_access(&event_store) {
            eprintln!("Calendar access denied. Please grant access in System Settings > Privacy & Security > Calendars");
            return;
        }

        let events = fetch_events(&event_store);

        let dismissed_events = Arc::new(Mutex::new(HashSet::new()));

        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

        let delegate = MenuDelegate::new(
            mtm,
            dismissed_events.clone(),
            event_store.clone(),
            status_item.clone(),
        );

        if let Some(button) = status_item.button(mtm) {
            let dismissed_set = dismissed_events.lock().unwrap();
            let title = get_status_bar_title(&events, &dismissed_set);
            button.setTitle(&NSString::from_str(&title));
        }

        let menu = build_menu(events, &delegate, &dismissed_events, mtm);
        status_item.setMenu(Some(&menu));

        let notification_center = NSNotificationCenter::defaultCenter();
        let notification_name = ns_string!("EKEventStoreChangedNotification");
        notification_center.addObserver_selector_name_object(
            &delegate,
            objc2::sel!(eventStoreChanged:),
            Some(notification_name),
            Some(&event_store),
        );

        app.run();
    }
}
