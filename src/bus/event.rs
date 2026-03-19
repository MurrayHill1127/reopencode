use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait EventProperties: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

impl<T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static> EventProperties for T {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition<T: EventProperties> {
    pub event_type: &'static str,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T: EventProperties> EventDefinition<T> {
    pub const fn new(event_type: &'static str) -> Self {
        Self {
            event_type,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub properties: serde_json::Value,
}

impl Event {
    pub fn new<T: EventProperties>(event_type: &str, properties: T) -> Self {
        Self {
            event_type: event_type.to_string(),
            properties: serde_json::to_value(properties).unwrap_or(serde_json::json!({})),
        }
    }

    pub fn properties<T: EventProperties>(&self) -> Option<T> {
        serde_json::from_value(self.properties.clone()).ok()
    }
}

pub mod definitions {
    use super::*;
    use serde::{Deserialize, Serialize};

    pub const INSTANCE_DISPOSED: EventDefinition<InstanceDisposedProperties> =
        EventDefinition::new("server.instance.disposed");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InstanceDisposedProperties {
        pub directory: String,
    }

    pub const MCP_TOOLS_CHANGED: EventDefinition<McpToolsChangedProperties> =
        EventDefinition::new("mcp.tools.changed");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpToolsChangedProperties {
        pub server: String,
    }

    pub const MCP_BROWSER_OPEN_FAILED: EventDefinition<McpBrowserOpenFailedProperties> =
        EventDefinition::new("mcp.browser.open.failed");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct McpBrowserOpenFailedProperties {
        pub mcp_name: String,
        pub url: String,
    }

    pub const PTY_CREATED: EventDefinition<PtyCreatedProperties> =
        EventDefinition::new("pty.created");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyCreatedProperties {
        pub info: PtyInfo,
    }

    pub const PTY_UPDATED: EventDefinition<PtyUpdatedProperties> =
        EventDefinition::new("pty.updated");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyUpdatedProperties {
        pub info: PtyInfo,
    }

    pub const PTY_EXITED: EventDefinition<PtyExitedProperties> = EventDefinition::new("pty.exited");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyExitedProperties {
        pub id: String,
        pub exit_code: i32,
    }

    pub const PTY_DELETED: EventDefinition<PtyDeletedProperties> =
        EventDefinition::new("pty.deleted");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyDeletedProperties {
        pub id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PtyInfo {
        pub id: String,
        pub title: String,
        pub command: String,
        pub args: Vec<String>,
        pub cwd: String,
        pub status: String,
        pub pid: u32,
    }

    pub const TUI_TOAST_SHOW: EventDefinition<TuiToastShowProperties> =
        EventDefinition::new("tui.toast.show");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TuiToastShowProperties {
        pub title: String,
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variant: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub duration: Option<u64>,
    }

    pub const TUI_PROMPT_APPEND: EventDefinition<TuiPromptAppendProperties> =
        EventDefinition::new("tui.prompt.append");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TuiPromptAppendProperties {
        pub prompt: String,
    }

    pub const TUI_COMMAND_EXECUTE: EventDefinition<TuiCommandExecuteProperties> =
        EventDefinition::new("tui.command.execute");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TuiCommandExecuteProperties {
        pub command: String,
    }

    pub const TUI_SESSION_SELECT: EventDefinition<TuiSessionSelectProperties> =
        EventDefinition::new("tui.session.select");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TuiSessionSelectProperties {
        pub session_id: String,
    }

    pub const TUI_MESSAGE_RECEIVED: EventDefinition<TuiMessageReceivedProperties> =
        EventDefinition::new("tui.message.received");

    /// Properties for TUI message received events (transcript messages).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TuiMessageReceivedProperties {
        pub role: String,
        pub content: String,
        pub timestamp: i64,
    }

    pub const SERVER_CONNECTED: EventDefinition<ServerConnectedProperties> =
        EventDefinition::new("server.connected");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServerConnectedProperties {}

    pub const SERVER_HEARTBEAT: EventDefinition<ServerHeartbeatProperties> =
        EventDefinition::new("server.heartbeat");

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServerHeartbeatProperties {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_message_received_event_type() {
        use definitions::TUI_MESSAGE_RECEIVED;
        assert_eq!(TUI_MESSAGE_RECEIVED.event_type, "tui.message.received");
    }

    #[test]
    fn test_tui_message_received_properties_serialize() {
        use definitions::TuiMessageReceivedProperties;

        let props = TuiMessageReceivedProperties {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&props).expect("should serialize");
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
        assert!(json.contains("\"timestamp\":1234567890"));
    }

    #[test]
    fn test_tui_message_received_properties_deserialize() {
        use definitions::TuiMessageReceivedProperties;

        let json = r#"{"role":"assistant","content":"Response here","timestamp":9876543210}"#;

        let props: TuiMessageReceivedProperties =
            serde_json::from_str(json).expect("should deserialize");

        assert_eq!(props.role, "assistant");
        assert_eq!(props.content, "Response here");
        assert_eq!(props.timestamp, 9876543210);
    }

    #[test]
    fn test_tui_message_received_properties_roundtrip() {
        use definitions::TuiMessageReceivedProperties;

        let original = TuiMessageReceivedProperties {
            role: "system".to_string(),
            content: "System message content".to_string(),
            timestamp: 1111111111,
        };

        let json = serde_json::to_string(&original).expect("should serialize");
        let deserialized: TuiMessageReceivedProperties =
            serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(deserialized.role, original.role);
        assert_eq!(deserialized.content, original.content);
        assert_eq!(deserialized.timestamp, original.timestamp);
    }

    #[test]
    fn test_create_event_with_tui_message_received() {
        use definitions::{TUI_MESSAGE_RECEIVED, TuiMessageReceivedProperties};

        let props = TuiMessageReceivedProperties {
            role: "user".to_string(),
            content: "Test message".to_string(),
            timestamp: 5555555555,
        };

        let event = Event::new(TUI_MESSAGE_RECEIVED.event_type, props);

        assert_eq!(event.event_type, "tui.message.received");
        assert!(event.properties.get("role").is_some());
        assert!(event.properties.get("content").is_some());
        assert!(event.properties.get("timestamp").is_some());
    }

    #[test]
    fn test_extract_properties_from_event() {
        use definitions::{TUI_MESSAGE_RECEIVED, TuiMessageReceivedProperties};

        let props = TuiMessageReceivedProperties {
            role: "assistant".to_string(),
            content: "Extracted content".to_string(),
            timestamp: 7777777777,
        };

        let event = Event::new(TUI_MESSAGE_RECEIVED.event_type, props);
        let extracted: TuiMessageReceivedProperties =
            event.properties().expect("should extract properties");

        assert_eq!(extracted.role, "assistant");
        assert_eq!(extracted.content, "Extracted content");
        assert_eq!(extracted.timestamp, 7777777777);
    }
}
