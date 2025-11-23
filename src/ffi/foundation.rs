use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSNotificationCenter, NSString};

/// Macro to encapsulate the unsafe super init pattern required by objc2
/// This cannot be abstracted into a function due to objc2's type system requiring
/// PartialInit<T> with specific trait bounds that don't work with generics.
#[macro_export]
macro_rules! init_objc_super {
    ($this:expr) => {
        unsafe { objc2::msg_send![super($this), init] }
    };
}

pub fn add_observer<T>(
    notification_center: &NSNotificationCenter,
    observer: &Retained<T>,
    selector: objc2::runtime::Sel,
    name: Option<&NSString>,
    object: Option<&objc2::runtime::AnyObject>,
) where
    T: objc2::Message,
{
    unsafe {
        let observer_ptr: *const T = Retained::as_ptr(observer);
        let observer_anyobject = observer_ptr as *const objc2::runtime::AnyObject;
        notification_center.addObserver_selector_name_object(
            &*observer_anyobject,
            selector,
            name,
            object,
        );
    }
}

/// Safely extracts a String from an NSMenuItem's representedObject
///
/// This function encapsulates the unsafe pointer casting required to extract
/// a String from the opaque AnyObject returned by NSMenuItem.representedObject()
pub fn ns_menu_item_represented_object_to_string(
    represented_object: &Retained<AnyObject>,
) -> String {
    unsafe {
        let ns_string: *const NSString = Retained::as_ptr(represented_object).cast();
        (*ns_string).to_string()
    }
}
