mod args;
mod calendar;
mod event_observers;
mod ffi;
mod launchd;
mod menu;

use crate::event_observers::observe_system_notifs;
use args::handle_args;
use calendar::EventCollection;
use menu::{MenuBuilder, MenuDelegate};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSStatusBar, NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSNotificationCenter, NSString};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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
            eprintln!("Error: Application must be called from main thread");
            std::process::exit(1);
        }
    };

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let event_store = event_kit::init_event_store(mtm);
    if let Err(e) = calendar::request_access(&event_store) {
        eprintln!("Error: Calendar access required but denied - {}", e);
        eprintln!("Please grant calendar access in:");
        eprintln!("  System Settings > Privacy & Security > Calendars");
        std::process::exit(1);
    }

    let events = EventCollection::fetch(&event_store);
    let dismissed_events = Arc::new(Mutex::new(HashSet::new()));

    let status_item =
        NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);

    if let Some(button) = status_item.button(mtm) {
        let title = match dismissed_events.lock() {
            Ok(dismissed_set) => events.get_title(&dismissed_set),
            Err(e) => {
                eprintln!("Error: Failed to acquire lock on dismissed events: {}", e);
                "Calendar".to_string()
            }
        };
        button.setTitle(&NSString::from_str(&title));
    } else {
        eprintln!("Error: Status item button is unavailable");
        std::process::exit(1);
    }

    let delegate = MenuDelegate::new(
        mtm,
        dismissed_events.clone(),
        event_store.clone(),
        status_item.clone(),
    );

    let menu = MenuBuilder::new(events.into_vec(), &delegate, &dismissed_events, mtm).build();
    status_item.setMenu(Some(&menu));

    let notification_center = NSNotificationCenter::defaultCenter();
    observe_system_notifs(notification_center, &delegate);

    app.run();
}
