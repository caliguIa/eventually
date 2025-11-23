use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSColor, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSData, NSRange, NSRect, NSSize, NSString};

pub fn init_image_from_data(data: &NSData) -> Option<Retained<NSImage>> {
    let image: Option<Retained<NSImage>> =
        unsafe { msg_send![msg_send![objc2::class!(NSImage), alloc], initWithData: data] };
    image
}

pub fn set_image_properties(image: &NSImage, size: NSSize, is_template: bool) {
    image.setSize(size);
    image.setTemplate(is_template);
}

pub fn init_image_with_size(size: NSSize) -> Retained<NSImage> {
    unsafe {
        let image: Retained<NSImage> =
            msg_send![msg_send![objc2::class!(NSImage), alloc], initWithSize: size];
        image
    }
}

pub fn lock_focus(image: &NSImage) {
    unsafe {
        let _: () = msg_send![&*image, lockFocus];
    }
}

pub fn unlock_focus(image: &NSImage) {
    unsafe {
        let _: () = msg_send![&*image, unlockFocus];
    }
}

pub fn draw_filled_circle(color: &NSColor, rect: NSRect) {
    unsafe {
        color.setFill();
        let bezier_path: *mut AnyObject = msg_send![
            objc2::class!(NSBezierPath),
            bezierPathWithOvalInRect: rect
        ];
        let _: () = msg_send![bezier_path, fill];
    }
}

pub fn init_menu(mtm: MainThreadMarker, title: &NSString) -> Retained<NSMenu> {
    NSMenu::initWithTitle(mtm.alloc(), title)
}

pub fn init_menu_item(
    mtm: MainThreadMarker,
    title: &NSString,
    action: Option<objc2::runtime::Sel>,
    key_equivalent: &NSString,
) -> Retained<NSMenuItem> {
    unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), title, action, key_equivalent) }
}

pub fn init_attributed_string(string: &NSString) -> Retained<AnyObject> {
    unsafe {
        msg_send![
            msg_send![objc2::class!(NSMutableAttributedString), alloc],
            initWithString: &*string
        ]
    }
}

pub fn set_attributed_title(item: &NSMenuItem, attr_string: &AnyObject) {
    unsafe {
        let _: () = msg_send![&*item, setAttributedTitle: attr_string];
    }
}

pub fn add_attribute(
    attr_string: &AnyObject,
    attribute: &AnyObject,
    value: &AnyObject,
    range: NSRange,
) {
    unsafe {
        let _: () = msg_send![
            attr_string,
            addAttribute: attribute,
            value: value,
            range: range
        ];
    }
}

pub fn get_foreground_color_attribute() -> &'static AnyObject {
    unsafe extern "C" {
        static NSForegroundColorAttributeName: &'static AnyObject;
    }
    unsafe { NSForegroundColorAttributeName }
}

pub fn get_font_attribute() -> &'static AnyObject {
    unsafe extern "C" {
        static NSFontAttributeName: &'static AnyObject;
    }
    unsafe { NSFontAttributeName }
}

pub fn set_menu_item_target<T>(item: &NSMenuItem, target: Option<&T>)
where
    T: objc2::Message,
{
    unsafe {
        let target_anyobject = target.map(|t| {
            let ptr: *const T = t as *const T;
            &*(ptr as *const AnyObject)
        });
        item.setTarget(target_anyobject);
    }
}

pub fn set_menu_item_represented_object(item: &NSMenuItem, object: Option<&objc2::runtime::AnyObject>) {
    unsafe {
        item.setRepresentedObject(object);
    }
}
