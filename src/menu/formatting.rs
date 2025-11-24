use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSColor, NSFont};
use objc2_foundation::{NSRange, NSString};

use crate::ffi::app_kit;

pub struct AttributedString {
    inner: Retained<AnyObject>,
}

struct AttributedStringRef<'a> {
    inner: &'a AnyObject,
}

impl AttributedString {
    pub fn new(text: &str) -> Self {
        let ns_string = NSString::from_str(text);
        Self {
            inner: app_kit::init_attributed_string(&ns_string),
        }
    }

    fn from_objc(objc: &AnyObject) -> AttributedStringRef<'_> {
        AttributedStringRef { inner: objc }
    }

    pub fn apply_bold(&self, range: NSRange) -> &Self {
        let font_attr = app_kit::get_font_attribute();
        let bold_font = NSFont::boldSystemFontOfSize(0.0);
        app_kit::add_attribute(&self.inner, font_attr, &**bold_font, range);
        self
    }

    pub fn apply_color(&self, color: &NSColor, range: NSRange) -> &Self {
        let foreground_color_attr = app_kit::get_foreground_color_attribute();
        app_kit::add_attribute(&self.inner, foreground_color_attr, &**color, range);
        self
    }

    pub fn apply_secondary_color(&self, range: NSRange) -> &Self {
        let secondary_color = NSColor::secondaryLabelColor();
        self.apply_color(&secondary_color, range)
    }

    pub fn apply_tertiary_color(&self, range: NSRange) -> &Self {
        let tertiary_color = NSColor::tertiaryLabelColor();
        self.apply_color(&tertiary_color, range)
    }

    pub fn as_objc(&self) -> &AnyObject {
        &self.inner
    }
}

impl<'a> AttributedStringRef<'a> {
    pub fn apply_bold(self, range: NSRange) -> Self {
        let font_attr = app_kit::get_font_attribute();
        let bold_font = NSFont::boldSystemFontOfSize(0.0);
        app_kit::add_attribute(self.inner, font_attr, &**bold_font, range);
        self
    }

    pub fn apply_color(self, color: &NSColor, range: NSRange) -> Self {
        let foreground_color_attr = app_kit::get_foreground_color_attribute();
        app_kit::add_attribute(self.inner, foreground_color_attr, &**color, range);
        self
    }

    pub fn apply_secondary_color(self, range: NSRange) -> Self {
        let secondary_color = NSColor::secondaryLabelColor();
        self.apply_color(&secondary_color, range)
    }

    pub fn apply_tertiary_color(self, range: NSRange) -> Self {
        let tertiary_color = NSColor::tertiaryLabelColor();
        self.apply_color(&tertiary_color, range)
    }
}

pub fn create_attributed_string(text: &str) -> Retained<AnyObject> {
    AttributedString::new(text).inner.clone()
}

pub fn apply_bold_font(attr_string: &AnyObject, range: NSRange) {
    AttributedString::from_objc(attr_string).apply_bold(range);
}

pub fn apply_secondary_color(attr_string: &AnyObject, range: NSRange) {
    AttributedString::from_objc(attr_string).apply_secondary_color(range);
}

pub fn apply_tertiary_color(attr_string: &AnyObject, range: NSRange) {
    AttributedString::from_objc(attr_string).apply_tertiary_color(range);
}
