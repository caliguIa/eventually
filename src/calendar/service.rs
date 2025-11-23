/// Information about video conferencing services
#[derive(Copy, Clone, Debug)]
pub struct ServiceInfo {
    pub pattern: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
}

const SERVICES: &[ServiceInfo] = &[
    ServiceInfo {
        pattern: "slack.com",
        name: "Slack",
        icon: "slack",
    },
    ServiceInfo {
        pattern: "zoom.us",
        name: "Zoom",
        icon: "zoom",
    },
    ServiceInfo {
        pattern: "meet.google",
        name: "Google Meet",
        icon: "google",
    },
    ServiceInfo {
        pattern: "teams.microsoft.com",
        name: "Teams",
        icon: "teams",
    },
    ServiceInfo {
        pattern: "teams.live.com",
        name: "Teams",
        icon: "teams",
    },
];

/// Detects which video conferencing service a URL belongs to
pub fn detect_service(url: &str) -> ServiceInfo {
    SERVICES
        .iter()
        .find(|s| url.contains(s.pattern))
        .copied()
        .unwrap_or(ServiceInfo {
            pattern: "",
            name: "Video Call",
            icon: "video",
        })
}

/// Extracts URL from location string if it's an HTTP/HTTPS URL
pub fn extract_url(location: Option<&str>) -> Option<&str> {
    location.filter(|loc| loc.starts_with("http://") || loc.starts_with("https://"))
}
