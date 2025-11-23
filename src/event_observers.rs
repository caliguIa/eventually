use objc2::rc::Retained;
use objc2_foundation::{ns_string, NSNotificationCenter};

use crate::menu_delegate::MenuDelegate;

pub fn observe_system_notifs(
    notification_center: Retained<NSNotificationCenter>,
    delegate: &Retained<MenuDelegate>,
) {
    use crate::ffi::foundation;

    foundation::add_observer(
        &notification_center,
        delegate,
        objc2::sel!(eventStoreChanged:),
        Some(ns_string!("EKEventStoreChangedNotification")),
        None,
    );
    foundation::add_observer(
        &notification_center,
        delegate,
        objc2::sel!(didWakeNotification:),
        Some(ns_string!("NSWorkspaceDidWakeNotification")),
        None,
    );
}
