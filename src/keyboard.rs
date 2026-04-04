use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Keyboard {
    pub one_time: bool,
    pub inline: bool,
    pub buttons: Vec<Vec<KeyboardButton>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct KeyboardButton {
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ButtonColor>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "text")]
    Text {
        label: String,
        payload: Option<String>,
    },
    #[serde(rename = "callback")]
    Callback {
        label: String,
        payload: Option<String>,
    },
    #[serde(rename = "open_link")]
    OpenLink { link: String, label: String },
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ButtonColor {
    Primary,
    Secondary,
    Negative,
    Positive,
}

impl Keyboard {
    pub fn new(one_time: bool, inline: bool) -> Self {
        Self {
            one_time,
            inline,
            buttons: vec![],
        }
    }

    pub fn add_row(mut self, row: Vec<KeyboardButton>) -> Self {
        self.buttons.push(row);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_serialization() {
        let kb = Keyboard::new(false, true).add_row(vec![KeyboardButton {
            action: Action::Text {
                label: "Btn".to_string(),
                payload: Some("payload".to_string()),
            },
            color: Some(ButtonColor::Primary),
        }]);

        let json = serde_json::to_string(&kb).unwrap();
        // Проверяем, что нет лишних пробелов или неправильных кейсов у enum
        assert_eq!(
            json,
            r#"{"one_time":false,"inline":true,"buttons":[[{"action":{"type":"text","label":"Btn","payload":"payload"},"color":"primary"}]]}"#
        );
    }
}
