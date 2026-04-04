use crate::{KnownUpdate, Update, UpdateKind};
use serde_json::Value;

/// Фильтр для текстовой команды (строгое совпадение с текстом или строка, начинающаяся с `prefix `)
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

/// Фильтр для кнопки "Начать" (payload содержит {"command":"start"})
pub fn is_start() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = &update.kind {
            if let Some(payload_str) = &object.message.payload {
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(payload_str) {
                    if let Some(Value::String(cmd)) = map.get("command") {
                        return cmd == "start";
                    }
                }
            }
        }
        false
    }
}

/// Фильтр для конкретного текста сообщения
pub fn is_text(expected: &'static str) -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = &update.kind {
            return object.message.text == expected;
        }
        false
    }
}

/// Фильтр для всех событий `message_event` (callback от inline-кнопок)
pub fn is_callback() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        matches!(
            update.kind,
            UpdateKind::Known(KnownUpdate::MessageEvent { .. })
        )
    }
}

/// Базовый фильтр для любого нового сообщения
pub fn any_message() -> impl Fn(&Update) -> bool + Send + Sync + 'static {
    move |update: &Update| -> bool {
        matches!(
            update.kind,
            UpdateKind::Known(KnownUpdate::MessageNew { .. })
        )
    }
}
