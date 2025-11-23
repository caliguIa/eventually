use objc2::rc::Retained;
use objc2_foundation::{NSNotificationCenter, NSString};

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
        notification_center.addObserver_selector_name_object(&*observer_anyobject, selector, name, object);
    }
}

pub fn ns_string_to_string(ns_string_ptr: *const NSString) -> String {
    unsafe { (*ns_string_ptr).to_string() }
}
