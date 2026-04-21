use crate::types::{KnownUpdate, Update, UpdateKind, MessageNewObject};
use crate::dispatcher::HandlerResult;
use dptree::prelude::*;
use dptree::di::DependencyMap;

/// Extraction filter for any new message
pub fn any_message() -> Handler<'static, DependencyMap, HandlerResult> {
    dptree::filter_map(|update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
            Some(object)
        } else {
            None
        }
    })
}

/// Extraction filter for callback events
pub fn is_callback() -> Handler<'static, DependencyMap, HandlerResult> {
    dptree::filter_map(|update: Update| {
        if let UpdateKind::Known(KnownUpdate::MessageEvent { object }) = update.kind {
            Some(object)
        } else {
            None
        }
    })
}

/// Filter for text commands (exact match or string starting with `prefix `)
pub fn command(prefix: &'static str) -> impl Fn(&MessageNewObject) -> bool + Send + Sync + 'static {
    move |obj: &MessageNewObject| -> bool {
        let text = obj.message.text.trim();
        text == prefix || text.starts_with(&format!("{} ", prefix))
    }
}

/// Filter for specific message text
pub fn is_text(expected: &'static str) -> impl Fn(&MessageNewObject) -> bool + Send + Sync + 'static {
    move |obj: &MessageNewObject| -> bool {
        obj.message.text == expected
    }
}
