use objc2::{declare_class, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2::rc::Retained;
use objc2_app_kit::{NSMenuItem, NSWorkspace};
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSURL};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

pub struct MenuDelegateIvars {
    dismissed_events: Arc<Mutex<HashSet<String>>>,
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
                        NSWorkspace::sharedWorkspace().openURL(&url);
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
                    
                    if let Ok(mut dismissed) = self.ivars().dismissed_events.lock() {
                        dismissed.insert(event_id_string);
                    }
                }
            }
        }
    }
);

impl MenuDelegate {
    pub fn new(mtm: MainThreadMarker, dismissed_events: Arc<Mutex<HashSet<String>>>) -> Retained<Self> {
        let this = mtm.alloc();
        let this = this.set_ivars(MenuDelegateIvars { dismissed_events });
        unsafe { msg_send_id![super(this), init] }
    }
}
