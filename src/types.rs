use serde::Deserialize;
use serde_json::Value;

use crate::dispatcher::BoxError;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response<T> {
    Ok { response: T },
    Err { error: ApiError },
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error_code: i32,
    pub error_msg: String,
    pub request_params: Vec<RequestParam>,
}

#[derive(Debug, Deserialize)]
pub struct RequestParam {
    pub key: String,
    pub value: String,
}

#[derive(thiserror::Error, Debug)]
pub enum VkError {
    #[error("API Error {}: {}", .0.error_code, .0.error_msg)]
    Api(ApiError),
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Shutdown send error")]
    Send(#[from] tokio::sync::mpsc::error::SendError<()>),
}

#[derive(Debug, Deserialize)]
pub struct LongPollServer {
    pub server: String,
    pub key: String,
    pub ts: String,
}

#[derive(Debug, Deserialize)]
pub struct LongPollResponse {
    pub ts: String,
    pub updates: Vec<Update>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Update {
    pub event_id: String,
    pub group_id: i64,
    pub v: String,

    #[serde(flatten)]
    pub kind: UpdateKind,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum UpdateKind {
    Known(KnownUpdate),
    Unknown(Value),
}

impl TryFrom<UpdateKind> for KnownUpdate {
    type Error = BoxError;

    fn try_from(value: UpdateKind) -> Result<Self, Self::Error> {
        match value {
            UpdateKind::Known(k) => Ok(k),
            UpdateKind::Unknown(u) => {
                Err(anyhow::anyhow!("Unexpected update payload: {:?}", u).into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum KnownUpdate {
    #[serde(rename = "message_reply")]
    MessageReply { object: MessageObject },
    #[serde(rename = "message_new")]
    MessageNew { object: MessageNewObject },
    #[serde(rename = "message_typing_state")]
    MessageTypingState { object: TypingStateObject },
    #[serde(rename = "message_read")]
    MessageRead { object: MessageReadObject },
    #[serde(rename = "message_event")]
    MessageEvent { object: MessageEventObject },
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypingStateObject {
    pub from_id: i64,
    pub to_id: i64,
    pub state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub is_closed: Option<bool>,
    pub can_access_closed: Option<bool>,
    pub screen_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Conversation {
    pub peer: Peer,
    pub chat_settings: Option<ChatSettings>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Peer {
    pub id: i64,
    #[serde(rename = "type")]
    pub peer_type: String,
    pub local_id: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatSettings {
    pub title: String,
    pub members_count: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConversationsResponse {
    pub count: i64,
    pub items: Vec<Conversation>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageReadObject {
    pub from_id: i64,
    pub peer_id: i64,
    pub read_message_id: i64,
    pub conversation_message_id: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageEventObject {
    pub user_id: i64,
    pub peer_id: i64,
    pub event_id: String,
    pub payload: Option<serde_json::Value>,
    pub conversation_message_id: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageNewObject {
    pub message: MessageObject,
    pub client_info: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MessageObject {
    pub admin_author_id: Option<i64>,
    pub attachments: Vec<serde_json::Value>,
    pub conversation_message_id: i64,
    pub date: i64,
    pub from_id: i64,
    pub fwd_messages: Vec<serde_json::Value>,
    pub id: i64,
    pub important: bool,
    pub is_hidden: bool,
    pub out: i32,
    pub peer_id: i64,
    pub random_id: i64,
    pub text: String,
    pub payload: Option<String>,
    pub version: i64,
}
