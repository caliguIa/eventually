use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSImage};
use objc2_foundation::NSData;

use crate::calendar::Icon;
use crate::ffi::app_kit;

impl Icon {
    fn data(self) -> &'static [u8] {
        match self {
            Self::Calendar => include_bytes!("../../assets/icons/calendar.svg"),
            Self::CircleX => include_bytes!("../../assets/icons/circle-x.svg"),
            Self::Google => include_bytes!("../../assets/icons/google.svg"),
            Self::Slack => include_bytes!("../../assets/icons/slack.svg"),
            Self::Teams => include_bytes!("../../assets/icons/teams.svg"),
            Self::Video => include_bytes!("../../assets/icons/video.svg"),
        }
    }

    pub fn load(self) -> Option<Retained<NSImage>> {
        let data = NSData::with_bytes(self.data());
        let image = app_kit::init_image_from_data(&data).or_else(|| {
            eprintln!("Error: Failed to create image from icon data: {:?}", self);
            None
        })?;
        let size = objc2_foundation::NSSize::new(16.0, 16.0);
        app_kit::set_image_properties(&image, size, true);
        Some(image)
    }

    pub fn load_colored(color: &NSColor) -> Option<Retained<NSImage>> {
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
}
