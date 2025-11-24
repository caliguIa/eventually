use objc2::rc::Retained;
use objc2_foundation::{ns_string, NSNotificationCenter, NSString};

use crate::ffi::foundation;
use crate::menu::MenuDelegate;

#[derive(Debug, Clone, Copy)]
pub enum SystemNotification {
    EventStoreChanged,
    WorkspaceDidWake,
}

impl SystemNotification {
    fn name(&self) -> &NSString {
        match self {
            Self::EventStoreChanged => ns_string!("EKEventStoreChangedNotification"),
            Self::WorkspaceDidWake => ns_string!("NSWorkspaceDidWakeNotification"),
        }
    }

    fn selector(&self) -> objc2::runtime::Sel {
        match self {
            Self::EventStoreChanged => objc2::sel!(eventStoreChanged:),
            Self::WorkspaceDidWake => objc2::sel!(didWakeNotification:),
        }
    }

    fn add(&self, center: &NSNotificationCenter, delegate: &Retained<MenuDelegate>) {
        foundation::add_observer(center, delegate, self.selector(), Some(self.name()), None);
    }
}

pub struct SystemNotificationObserver<'a> {
    notification_center: Retained<NSNotificationCenter>,
    delegate: &'a Retained<MenuDelegate>,
}

impl<'a> SystemNotificationObserver<'a> {
    pub fn new(delegate: &'a Retained<MenuDelegate>) -> Self {
        Self {
            notification_center: NSNotificationCenter::defaultCenter(),
            delegate,
        }
    }

    pub fn register(self) -> Self {
        self.add_notification(SystemNotification::EventStoreChanged);
        self.add_notification(SystemNotification::WorkspaceDidWake);
        self
    }

    fn add_notification(&self, notification: SystemNotification) {
        notification.add(&self.notification_center, &self.delegate);
    }
}
