use chrono;
use objc2::rc::Retained;
use objc2::{define_class, DeclaredClass};
use objc2_app_kit::{NSMenuItem, NSStatusItem, NSWorkspace};
use objc2_event_kit::EKEventStore;
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSString, NSURL};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::calendar;
use crate::ffi::foundation::ns_menu_item_represented_object_to_string;
use crate::init_objc_super;
use crate::menu::build_menu;

pub struct Ivars {
    dismissed_events: Arc<Mutex<HashSet<String>>>,
    mtm: MainThreadMarker,
    event_store: Retained<EKEventStore>,
    status_item: Retained<NSStatusItem>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = Ivars]
    #[name = "MenuDelegate"]
    pub struct MenuDelegate;

    impl MenuDelegate {
        #[unsafe(method(eventStoreChanged:))]
        fn event_store_changed(&self, _notification: &NSNotification) {
            let events = calendar::events::fetch(&self.ivars().event_store);

            let dismissed_set = self.ivars().dismissed_events.lock().unwrap();
            let title = calendar::events::get_title(&events, &dismissed_set);
            drop(dismissed_set);

            let menu = build_menu(
                events,
                self,
                &self.ivars().dismissed_events,
                self.ivars().mtm,
            );

            let status_item = &self.ivars().status_item;

            if let Some(button) = status_item.button(self.ivars().mtm) {
                button.setTitle(&NSString::from_str(&title));
            }

            status_item.setMenu(Some(&menu));
        }

        #[unsafe(method(didWakeNotification:))]
        fn did_wake_notification(&self, _notification: &NSNotification) {
            let events = calendar::events::fetch(&self.ivars().event_store);

            let dismissed_set = self.ivars().dismissed_events.lock().unwrap();
            let title = calendar::events::get_title(&events, &dismissed_set);
            drop(dismissed_set);

            let menu = build_menu(
                events,
                self,
                &self.ivars().dismissed_events,
                self.ivars().mtm,
            );

            let status_item = &self.ivars().status_item;

            if let Some(button) = status_item.button(self.ivars().mtm) {
                button.setTitle(&NSString::from_str(&title));
            }

            status_item.setMenu(Some(&menu));
        }

        #[unsafe(method(openEvent:))]
        fn open_event(&self, sender: &NSMenuItem) {
            if let Some(obj) = sender.representedObject() {
                let data = ns_menu_item_represented_object_to_string(&obj);

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

        #[unsafe(method(openURL:))]
        fn open_url(&self, sender: &NSMenuItem) {
            if let Some(obj) = sender.representedObject() {
                let url_string = ns_menu_item_represented_object_to_string(&obj);

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

        #[unsafe(method(dismissEvent:))]
        fn dismiss_event(&self, sender: &NSMenuItem) {
            if let Some(obj) = sender.representedObject() {
                let event_id_string = ns_menu_item_represented_object_to_string(&obj);


                if let Ok(mut dismissed) = self.ivars().dismissed_events.lock() {
                    dismissed.insert(event_id_string.clone());
                }

                let events = calendar::events::fetch(&self.ivars().event_store);
                for e in &events {
                    if e.start.date_naive() == chrono::Local::now().date_naive() {
                    }
                }

                let dismissed_set = self.ivars().dismissed_events.lock().unwrap();
                let title = calendar::events::get_title(&events, &dismissed_set);
                drop(dismissed_set);

                let menu = build_menu(
                    events,
                    self,
                    &self.ivars().dismissed_events,
                    self.ivars().mtm,
                );

                let status_item = &self.ivars().status_item;

                if let Some(button) = status_item.button(self.ivars().mtm) {
                    button.setTitle(&NSString::from_str(&title));
                }

                status_item.setMenu(Some(&menu));
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
        let this = this.set_ivars(Ivars {
            dismissed_events,
            mtm,
            event_store,
            status_item,
        });
        init_objc_super!(this)
    }
}
