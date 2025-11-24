#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ServiceInfo {
    Slack,
    Zoom,
    GoogleMeet,
    MicrosoftTeams,
    Generic,
}

impl ServiceInfo {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Slack => "Slack",
            Self::Zoom => "Zoom",
            Self::GoogleMeet => "Google Meet",
            Self::MicrosoftTeams => "Teams",
            Self::Generic => "Video Call",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Zoom => "zoom",
            Self::GoogleMeet => "google",
            Self::MicrosoftTeams => "teams",
            Self::Generic => "video",
        }
    }
}

pub fn detect_service(url: &str) -> ServiceInfo {
    if url.contains("slack.com") {
        ServiceInfo::Slack
    } else if url.contains("zoom.us") {
        ServiceInfo::Zoom
    } else if url.contains("meet.google") {
        ServiceInfo::GoogleMeet
    } else if url.contains("teams.microsoft.com") || url.contains("teams.live.com") {
        ServiceInfo::MicrosoftTeams
    } else {
        ServiceInfo::Generic
    }
}

pub fn extract_url(location: Option<&str>) -> Option<&str> {
    location.filter(|loc| loc.starts_with("http://") || loc.starts_with("https://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_slack() {
        assert_eq!(
            detect_service("https://slack.com/huddle/T123/C456"),
            ServiceInfo::Slack
        );
    }

    #[test]
    fn test_detect_zoom() {
        assert_eq!(
            detect_service("https://zoom.us/j/123456789"),
            ServiceInfo::Zoom
        );
    }

    #[test]
    fn test_detect_google_meet() {
        assert_eq!(
            detect_service("https://meet.google.com/abc-defg-hij"),
            ServiceInfo::GoogleMeet
        );
    }

    #[test]
    fn test_detect_teams_microsoft() {
        assert_eq!(
            detect_service("https://teams.microsoft.com/l/meetup/..."),
            ServiceInfo::MicrosoftTeams
        );
    }

    #[test]
    fn test_detect_teams_live() {
        assert_eq!(
            detect_service("https://teams.live.com/meet/..."),
            ServiceInfo::MicrosoftTeams
        );
    }

    #[test]
    fn test_detect_generic() {
        assert_eq!(
            detect_service("https://example.com/video"),
            ServiceInfo::Generic
        );
    }

    #[test]
    fn test_service_name() {
        assert_eq!(ServiceInfo::Slack.name(), "Slack");
        assert_eq!(ServiceInfo::GoogleMeet.name(), "Google Meet");
        assert_eq!(ServiceInfo::Generic.name(), "Video Call");
    }

    #[test]
    fn test_service_icon() {
        assert_eq!(ServiceInfo::Slack.icon(), "slack");
        assert_eq!(ServiceInfo::Zoom.icon(), "zoom");
        assert_eq!(ServiceInfo::Generic.icon(), "video");
    }

    #[test]
    fn test_extract_url_https() {
        assert_eq!(
            extract_url(Some("https://example.com")),
            Some("https://example.com")
        );
    }

    #[test]
    fn test_extract_url_http() {
        assert_eq!(
            extract_url(Some("http://example.com")),
            Some("http://example.com")
        );
    }

    #[test]
    fn test_extract_url_no_protocol() {
        assert_eq!(extract_url(Some("example.com")), None);
    }

    #[test]
    fn test_extract_url_none() {
        assert_eq!(extract_url(None), None);
    }

    #[test]
    fn test_extract_url_empty() {
        assert_eq!(extract_url(Some("")), None);
    }
}
