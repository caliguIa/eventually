use std::borrow::Cow;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ServiceInfo {
    Slack,
    Zoom,
    GoogleMeet,
    MicrosoftTeams,
    Generic,
}

impl ServiceInfo {
    pub fn from_url(url: &str) -> Self {
        if url.contains("slack.com") {
            Self::Slack
        } else if url.contains("zoom.us") {
            Self::Zoom
        } else if url.contains("meet.google") {
            Self::GoogleMeet
        } else if url.contains("teams.microsoft.com") || url.contains("teams.live.com") {
            Self::MicrosoftTeams
        } else {
            Self::Generic
        }
    }

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

pub fn extract_url(location: Option<&str>) -> Option<&str> {
    location.filter(|loc| loc.starts_with("http://") || loc.starts_with("https://"))
}

pub struct SlackHuddleUrl<'a> {
    team: Cow<'a, str>,
    channel: Cow<'a, str>,
}

impl<'a> SlackHuddleUrl<'a> {
    pub fn parse(url: &'a str) -> Option<Self> {
        if !url.contains("/huddle/") {
            return None;
        }

        let parts: Vec<&str> = url.split('/').collect();
        let huddle_idx = parts.iter().position(|&p| p == "huddle")?;

        if huddle_idx + 2 >= parts.len() {
            return None;
        }

        Some(Self {
            team: Cow::Borrowed(parts[huddle_idx + 1]),
            channel: Cow::Borrowed(parts[huddle_idx + 2]),
        })
    }

    pub fn to_native_url(&self) -> String {
        format!("slack://join-huddle?team={}&id={}", self.team, self.channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_from_url_slack() {
        assert_eq!(
            ServiceInfo::from_url("https://slack.com/huddle/T123/C456"),
            ServiceInfo::Slack
        );
    }

    #[test]
    fn test_service_from_url_zoom() {
        assert_eq!(
            ServiceInfo::from_url("https://zoom.us/j/123456789"),
            ServiceInfo::Zoom
        );
    }

    #[test]
    fn test_service_from_url_google_meet() {
        assert_eq!(
            ServiceInfo::from_url("https://meet.google.com/abc-defg-hij"),
            ServiceInfo::GoogleMeet
        );
    }

    #[test]
    fn test_service_from_url_teams_microsoft() {
        assert_eq!(
            ServiceInfo::from_url("https://teams.microsoft.com/l/meetup/..."),
            ServiceInfo::MicrosoftTeams
        );
    }

    #[test]
    fn test_service_from_url_teams_live() {
        assert_eq!(
            ServiceInfo::from_url("https://teams.live.com/meet/..."),
            ServiceInfo::MicrosoftTeams
        );
    }

    #[test]
    fn test_service_from_url_generic() {
        assert_eq!(
            ServiceInfo::from_url("https://example.com/video"),
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

    #[test]
    fn test_slack_huddle_url_parse() {
        let url = "https://slack.com/huddle/T123ABC/C456DEF";
        let huddle = SlackHuddleUrl::parse(url).expect("Should parse slack huddle URL");
        assert_eq!(huddle.team.as_ref(), "T123ABC");
        assert_eq!(huddle.channel.as_ref(), "C456DEF");
    }

    #[test]
    fn test_slack_huddle_url_to_native() {
        let url = "https://slack.com/huddle/T123ABC/C456DEF";
        let huddle = SlackHuddleUrl::parse(url).expect("Should parse slack huddle URL");
        assert_eq!(
            huddle.to_native_url(),
            "slack://join-huddle?team=T123ABC&id=C456DEF"
        );
    }

    #[test]
    fn test_slack_huddle_url_parse_invalid() {
        assert!(SlackHuddleUrl::parse("https://slack.com/messages").is_none());
        assert!(SlackHuddleUrl::parse("https://slack.com/huddle/T123").is_none());
    }
}
