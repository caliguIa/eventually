use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSColor, NSFont};
use objc2_foundation::{NSRange, NSString};

use crate::ffi::app_kit;

/// Creates an attributed string with styled text for menu items
pub fn create_attributed_string(text: &str) -> Retained<AnyObject> {
    let ns_string = NSString::from_str(text);
    app_kit::init_attributed_string(&ns_string)
}

/// Applies bold font to a specific range in an attributed string
pub fn apply_bold_font(attr_string: &AnyObject, range: NSRange) {
    let font_attr = app_kit::get_font_attribute();
    let bold_font = NSFont::boldSystemFontOfSize(0.0);
    app_kit::add_attribute(attr_string, font_attr, &**bold_font, range);
}

/// Applies a color to a specific range in an attributed string
pub fn apply_color(attr_string: &AnyObject, color: &NSColor, range: NSRange) {
    let foreground_color_attr = app_kit::get_foreground_color_attribute();
    app_kit::add_attribute(attr_string, foreground_color_attr, &**color, range);
}

/// Applies secondary label color (for dimmed text like end times)
pub fn apply_secondary_color(attr_string: &AnyObject, range: NSRange) {
    let secondary_color = NSColor::secondaryLabelColor();
    apply_color(attr_string, &secondary_color, range);
}

/// Applies tertiary label color (for very dimmed text)
pub fn apply_tertiary_color(attr_string: &AnyObject, range: NSRange) {
    let tertiary_color = NSColor::tertiaryLabelColor();
    apply_color(attr_string, &tertiary_color, range);
}
