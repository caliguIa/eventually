use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSImage};

use crate::ffi::app_kit;

pub fn load_icon(name: &str) -> Option<Retained<NSImage>> {
    let icon_data: &[u8] = match name {
        "calendar" => include_bytes!("../../assets/icons/calendar.svg"),
        "circle-x" => include_bytes!("../../assets/icons/circle-x.svg"),
        "google" => include_bytes!("../../assets/icons/google.svg"),
        "slack" => include_bytes!("../../assets/icons/slack.svg"),
        "teams" => include_bytes!("../../assets/icons/teams.svg"),
        "video" => include_bytes!("../../assets/icons/video.svg"),
        _ => {
            eprintln!("Warning: Unknown icon requested: {}", name);
            return None;
        }
    };
    let data = objc2_foundation::NSData::with_bytes(icon_data);
    let image = app_kit::init_image_from_data(&data).or_else(|| {
        eprintln!("Error: Failed to create image from icon data: {}", name);
        None
    })?;
    let size = objc2_foundation::NSSize::new(16.0, 16.0);
    app_kit::set_image_properties(&image, size, true);
    Some(image)
}

pub fn load_colored_icon(_name: &str, color: &NSColor) -> Option<Retained<NSImage>> {
    let size = objc2_foundation::NSSize::new(16.0, 16.0);
    let image = app_kit::init_image_with_size(size);

    app_kit::lock_focus(&image);

    let circle_rect = objc2_foundation::NSRect::new(
        objc2_foundation::NSPoint::new(4.0, 4.0),
        objc2_foundation::NSSize::new(8.0, 8.0),
    );
    app_kit::draw_filled_circle(color, circle_rect);

    app_kit::unlock_focus(&image);

    Some(image)
}
