use crate::dispatcher::HandlerResult;
use crate::types::{KnownUpdate, MessageNewObject, Update, UpdateKind};
use dptree::prelude::*;
use serde_json::Value;

/// Extraction filter for any new message
pub fn any_message() -> Handler<'static, HandlerResult> {
    dptree::filter_map(|update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
            Some(object)
        } else {
            None
        }
    })
}

/// Extraction filter for message payload commands matching a specific string
pub fn is_payload_command(expected: &'static str) -> Handler<'static, HandlerResult> {
    dptree::filter_map(move |update: Update| {
        match update.kind {
            UpdateKind::Known(KnownUpdate::MessageNew { object })
                if let Some(payload_str) = &object.message.payload
                    && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(payload_str)
                    && let Some(Value::String(cmd)) = map.get("command")
                    && cmd == expected =>
            {
                return Some(object);
            }
            _ => (),
        };
        None
    })
}

/// Extraction filter for the initial "Start" payload command
pub fn is_start() -> Handler<'static, HandlerResult> {
    is_payload_command("start")
}

/// Extraction filter for callback events
pub fn is_callback() -> Handler<'static, HandlerResult> {
    dptree::filter_map(|update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageEvent { object }) = update.kind {
            Some(object)
        } else {
            None
        }
    })
}

/// Extraction filter for text commands (exact match or string starting with `prefix `)
pub fn command(prefix: &'static str) -> Handler<'static, HandlerResult> {
    dptree::filter_map(move |update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
            let text = object.message.text.trim();
            if text == prefix || (text.starts_with(prefix) && text[prefix.len()..].starts_with(' ')) {
                return Some(object);
            }
        }
        None
    })
}

/// Extraction filter for specific message text
pub fn is_text(expected: &'static str) -> Handler<'static, HandlerResult> {
    dptree::filter_map(move |update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind
            && object.message.text == expected
        {
            return Some(object);
        }
        None
    })
}

/// Extraction filter for messages containing a voice message (audio_message)
pub fn voice_message() -> Handler<'static, HandlerResult> {
    dptree::filter_map(|update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
            Some(object)
        } else {
            None
        }
    })
    .chain(dptree::filter_map(|object: MessageNewObject| {
        object.message.voice_message().cloned()
    }))
}
