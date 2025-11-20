use chrono;
use objc2::rc::Retained;
use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSMenuItem, NSStatusItem, NSWorkspace};
use objc2_event_kit::EKEventStore;
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSString, NSURL};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::events::{fetch_events, get_status_bar_title};
use crate::menu::build_menu;

pub struct MenuDelegateIvars {
    dismissed_events: Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
    event_store: Retained<EKEventStore>,
    status_item: Retained<NSStatusItem>,
}

declare_class!(
    pub struct MenuDelegate;

    unsafe impl ClassType for MenuDelegate {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "MenuDelegate";
    }

    impl DeclaredClass for MenuDelegate {
        type Ivars = MenuDelegateIvars;
    }

    unsafe impl MenuDelegate {
        #[method(eventStoreChanged:)]
        fn event_store_changed(&self, _notification: &NSNotification) {
            unsafe {
                let events = fetch_events(&self.ivars().event_store);

                let dismissed_set = self.ivars().dismissed_events.lock().unwrap();
                let title = get_status_bar_title(&events, &dismissed_set);
                drop(dismissed_set);

                let menu = build_menu(events, self, &self.ivars().dismissed_events, self.ivars().mtm);

                let status_item = &self.ivars().status_item;

                if let Some(button) = status_item.button(self.ivars().mtm) {
                    button.setTitle(&NSString::from_str(&title));
                }

                status_item.setMenu(Some(&menu));
            }
        }

        #[method(openEvent:)]
        fn open_event(&self, sender: &NSMenuItem) {
            unsafe {
                if let Some(obj) = sender.representedObject() {
                    let ns_string: *const NSString = Retained::as_ptr(&obj).cast();
                    let data = (*ns_string).to_string();

                    let parts: Vec<&str> = data.split("|||").collect();
                    let event_id = parts[0];
                    let has_recurrence = parts.get(1).map(|s| *s == "true").unwrap_or(false);

                    let url_string = if has_recurrence {
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

                    let final_url = if url_string.contains("slack") {
                        if let Some(captures) = extract_slack_huddle_ids(&url_string) {
                            format!("slack://join-huddle?team={}&id={}", captures.0, captures.1)
                        } else {
                            url_string
                        }
                    } else {
                        url_string
                    };

                    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&final_url)) {
                        let workspace = NSWorkspace::sharedWorkspace();
                        workspace.openURL(&url);
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

                    eprintln!("Dismissing: {}", event_id_string);

                    if let Ok(mut dismissed) = self.ivars().dismissed_events.lock() {
                        dismissed.insert(event_id_string.clone());
                        eprintln!("Dismissed set: {:?}", dismissed);
                    }

                    // Rebuild the menu with updated dismissed events
                    let events = fetch_events(&self.ivars().event_store);
                    eprintln!("Today's event occurrence_keys:");
                    for e in &events {
                        if e.start.date_naive() == chrono::Local::now().date_naive() {
                            eprintln!("  - {}: {}", e.title, e.occurrence_key);
                        }
                    }

                    let dismissed_set = self.ivars().dismissed_events.lock().unwrap();
                    let title = get_status_bar_title(&events, &dismissed_set);
                    eprintln!("New title: {}", title);
                    drop(dismissed_set);

                    let menu = build_menu(events, self, &self.ivars().dismissed_events, self.ivars().mtm);

                    // Update the status bar using the stored status_item
                    let status_item = &self.ivars().status_item;

                    if let Some(button) = status_item.button(self.ivars().mtm) {
                        button.setTitle(&NSString::from_str(&title));
                    }

                    status_item.setMenu(Some(&menu));
                }
            }
        }
    }
);

fn extract_slack_huddle_ids(url: &str) -> Option<(String, String)> {
    if url.contains("/huddle/") {
        let parts: Vec<&str> = url.split('/').collect();
        if let Some(huddle_idx) = parts.iter().position(|&p| p == "huddle") {
            if huddle_idx + 2 < parts.len() {
                let team = parts[huddle_idx + 1].to_string();
                let channel = parts[huddle_idx + 2].to_string();
                return Some((team, channel));
            }
        }
    }

    None
}

impl MenuDelegate {
    pub fn new(
        mtm: MainThreadMarker,
        dismissed_events: Arc<Mutex<HashSet<String>>>,
        event_store: Retained<EKEventStore>,
        status_item: Retained<NSStatusItem>,
    ) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MenuDelegateIvars {
            dismissed_events,
            mtm,
            event_store,
            status_item,
        });
        unsafe { msg_send_id![super(this), init] }
    }
}
