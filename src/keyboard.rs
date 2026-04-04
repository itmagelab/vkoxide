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
