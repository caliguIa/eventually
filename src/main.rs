mod args;
mod calendar;
mod delegate;
mod event_observers;
mod ffi;
mod menu;
mod service;

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSStatusBar, NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSNotificationCenter, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use args::handle_args;
use calendar::{fetch_events, get_status_bar_title, request_calendar_access};
use delegate::MenuDelegate;
use menu::build_menu;

use crate::event_observers::observe_system_notifs;

fn main() {
    use crate::ffi::event_kit;
    
    match handle_args() {
        Some(Ok(())) => return,
        Some(Err(e)) => {
            eprintln!("Command failed: {e}");
            std::process::exit(1);
        }
        None => {}
    }

    let mtm = match MainThreadMarker::new() {
        Some(mtm) => mtm,
        None => {
            eprintln!("Must be called from main thread");
            std::process::exit(1);
        }
    };

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let event_store = event_kit::init_event_store(mtm);
    if !request_calendar_access(&event_store) {
        eprintln!("Calendar access denied. Please grant access in System Settings > Privacy & Security > Calendars");
        return;
    }

    let events = fetch_events(&event_store);
    let dismissed_events = Arc::new(Mutex::new(HashSet::new()));

    let status_item =
        NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);

    if let Some(button) = status_item.button(mtm) {
        let dismissed_set = dismissed_events
            .lock()
            .expect("dismissed_events lock should not be poisoned");
        let title = get_status_bar_title(&events, &dismissed_set);
        button.setTitle(&NSString::from_str(&title));
    } else {
        eprintln!("status is should have button");
        return;
    }

    let delegate = MenuDelegate::new(
        mtm,
        dismissed_events.clone(),
        event_store.clone(),
        status_item.clone(),
    );

    let menu = build_menu(events, &delegate, &dismissed_events, mtm);
    status_item.setMenu(Some(&menu));

    let notification_center = NSNotificationCenter::defaultCenter();
    observe_system_notifs(notification_center, &delegate);

    app.run();
}
