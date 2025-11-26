use objc2::rc::Retained;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSNotificationCenter, NSString};

use crate::ffi::foundation;
use crate::menu::MenuDelegate;

#[derive(Debug, Clone, Copy)]
enum NotificationCenter {
    Default,
    Workspace,
}

impl NotificationCenter {
    fn get(&self) -> Retained<NSNotificationCenter> {
        match self {
            Self::Default => NSNotificationCenter::defaultCenter(),
            Self::Workspace => NSWorkspace::sharedWorkspace().notificationCenter(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NotificationConfig {
    name: &'static str,
    selector: &'static str,
    center: NotificationCenter,
}

impl NotificationConfig {
    const fn new(name: &'static str, selector: &'static str, center: NotificationCenter) -> Self {
        Self {
            name,
            selector,
            center,
        }
    }

    fn name(&self) -> Retained<NSString> {
        NSString::from_str(self.name)
    }

    fn selector(&self) -> objc2::runtime::Sel {
        match self.selector {
            "eventStoreChanged:" => objc2::sel!(eventStoreChanged:),
            "didWakeNotification:" => objc2::sel!(didWakeNotification:),
            _ => unreachable!("Unknown selector"),
        }
    }

    fn center(&self) -> Retained<NSNotificationCenter> {
        self.center.get()
    }

    fn register(&self, delegate: &Retained<MenuDelegate>) {
        let name = self.name();
        foundation::add_observer(&self.center(), delegate, self.selector(), Some(&name), None);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SystemNotification {
    EventStoreChanged,
    WorkspaceDidWake,
}

impl SystemNotification {
    const fn config(&self) -> NotificationConfig {
        match self {
            Self::EventStoreChanged => NotificationConfig::new(
                "EKEventStoreChangedNotification",
                "eventStoreChanged:",
                NotificationCenter::Default,
            ),
            Self::WorkspaceDidWake => NotificationConfig::new(
                "NSWorkspaceDidWakeNotification",
                "didWakeNotification:",
                NotificationCenter::Workspace,
            ),
        }
    }

    fn register(&self, delegate: &Retained<MenuDelegate>) {
        self.config().register(delegate);
    }
}

pub struct SystemNotificationObserver<'a> {
    delegate: &'a Retained<MenuDelegate>,
}

impl<'a> SystemNotificationObserver<'a> {
    pub fn new(delegate: &'a Retained<MenuDelegate>) -> Self {
        Self { delegate }
    }

    pub fn register(self) -> Self {
        SystemNotification::EventStoreChanged.register(self.delegate);
        SystemNotification::WorkspaceDidWake.register(self.delegate);
        self
    }
}
