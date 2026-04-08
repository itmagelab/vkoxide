use crate::{KnownUpdate, Update, UpdateKind};
use serde_json::Value;

/// Filter for text commands (exact match or string starting with `prefix `)
pub fn command(prefix: &'static str) -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = &update.kind {
            let text = object.message.text.trim();
            if text == prefix || text.starts_with(&format!("{} ", prefix)) {
                return true;
            }
        }
        false
    }
}

/// Filter for the "Start" button (payload contains {"command":"start"})
pub fn is_start() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = &update.kind
            && let Some(payload_str) = &object.message.payload
            && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(payload_str)
            && let Some(Value::String(cmd)) = map.get("command")
        {
            return cmd == "start";
        }
        false
    }
}

/// Filter for specific message text
pub fn is_text(expected: &'static str) -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = &update.kind {
            return object.message.text == expected;
        }
        false
    }
}

/// Filter for all `message_event` events (callback from inline buttons)
pub fn is_callback() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        matches!(
            update.kind,
            UpdateKind::Known(KnownUpdate::MessageEvent { .. })
        )
    }
}

/// Base filter for any new message
pub fn any_message() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        matches!(
            update.kind,
            UpdateKind::Known(KnownUpdate::MessageNew { .. })
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_message_update(text: &str, payload: Option<&str>) -> Update {
        let mut msg_obj = serde_json::json!({
            "id": 1,
            "date": 1234567,
            "peer_id": 1,
            "from_id": 1,
            "text": text,
            "random_id": 0,
            "fwd_messages": [],
            "attachments": [],
            "important": false,
            "is_hidden": false,
            "out": 0,
            "conversation_message_id": 1,
            "version": 1
        });

        if let Some(p) = payload {
            msg_obj["payload"] = serde_json::json!(p);
        }

        serde_json::from_value(serde_json::json!({
            "event_id": "event_id1",
            "group_id": 1,
            "v": "5.199",
            "type": "message_new",
            "object": {
                "message": msg_obj,
                "client_info": null
            }
        }))
        .unwrap()
    }

    #[test]
    fn test_command_filter() {
        let update = create_message_update("/help me", None);
        assert!(command("/help")(&update));

        // Команда в середине текста не должна срабатывать
        let wrong_update = create_message_update("hello /help", None);
        assert!(!command("/help")(&wrong_update));
    }

    #[test]
    fn test_is_start_filter() {
        let update = create_message_update("Start", Some("{\"command\":\"start\"}"));
        assert!(is_start()(&update));

        let wrong_update = create_message_update("Start", Some("{\"command\":\"stop\"}"));
        assert!(!is_start()(&wrong_update));
    }

    #[test]
    fn test_is_text_filter() {
        let update = create_message_update("Hello", None);
        assert!(is_text("Hello")(&update));
        assert!(!is_text("Bye")(&update));
    }
}
