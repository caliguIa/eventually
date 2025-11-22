use objc2::rc::Retained;
use objc2_foundation::{ns_string, NSNotificationCenter};

use crate::delegate::MenuDelegate;

pub fn observe_system_notifs(
    notification_center: Retained<NSNotificationCenter>,
    delegate: &Retained<MenuDelegate>,
) {
    unsafe {
        notification_center.addObserver_selector_name_object(
            delegate,
            objc2::sel!(eventStoreChanged:),
            Some(ns_string!("EKEventStoreChangedNotification")),
            None,
        );
        notification_center.addObserver_selector_name_object(
            delegate,
            objc2::sel!(didWakeNotification:),
            Some(ns_string!("NSWorkspaceDidWakeNotification")),
            None,
        );
    }
}
