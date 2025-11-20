use chrono;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSMenuItem, NSStatusItem, NSWorkspace};
use objc2_event_kit::EKEventStore;
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSURL};
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

                    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&url_string)) {
                        let workspace = NSWorkspace::sharedWorkspace();

                        // Create configuration to prefer opening in native apps
                        let config: Retained<AnyObject> = msg_send_id![
                            objc2::class!(NSWorkspaceOpenConfiguration),
                            configuration
                        ];

                        // Open URL with configuration (no completion handler)
                        let _: () = msg_send![
                            &*workspace,
                            openURL: &*url,
                            configuration: &*config,
                            completionHandler: Option::<&AnyObject>::None
                        ];
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
