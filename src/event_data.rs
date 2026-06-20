use serde::Serialize;

/// Data representing callback button click response actions (event_data).
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum EventData {
    /// Shows a temporary pop-up notification (snackbar) in the user interface.
    #[serde(rename = "show_snackbar")]
    ShowSnackbar {
        /// Text to display in the notification (up to 90 characters).
        text: String,
    },
    /// Opens an external link.
    #[serde(rename = "open_link")]
    OpenLink {
        /// URL to open.
        link: String,
    },
    /// Opens a VK Mini App.
    #[serde(rename = "open_app")]
    OpenApp {
        /// ID of the Mini App.
        app_id: i64,
        /// Owner ID (group or user ID).
        owner_id: i64,
        /// Hash path for inner app navigation.
        hash: String,
    },
}

impl EventData {
    /// Create a new event response to show a transient pop-up notification.
    pub fn show_snackbar<S: Into<String>>(text: S) -> Self {
        Self::ShowSnackbar { text: text.into() }
    }

    /// Create a new event response to open a link.
    pub fn open_link<S: Into<String>>(link: S) -> Self {
        Self::OpenLink { link: link.into() }
    }

    /// Create a new event response to open a VK Mini App.
    pub fn open_app<S: Into<String>>(app_id: i64, owner_id: i64, hash: S) -> Self {
        Self::OpenApp {
            app_id,
            owner_id,
            hash: hash.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_data_serialization() {
        let show_snackbar = EventData::show_snackbar("Hello World");
        let json = serde_json::to_string(&show_snackbar).unwrap();
        assert_eq!(json, r#"{"type":"show_snackbar","text":"Hello World"}"#);

        let open_link = EventData::open_link("https://example.com");
        let json = serde_json::to_string(&open_link).unwrap();
        assert_eq!(json, r#"{"type":"open_link","link":"https://example.com"}"#);

        let open_app = EventData::open_app(1234, -5678, "some_hash");
        let json = serde_json::to_string(&open_app).unwrap();
        assert_eq!(
            json,
            r#"{"type":"open_app","app_id":1234,"owner_id":-5678,"hash":"some_hash"}"#
        );
    }
}
