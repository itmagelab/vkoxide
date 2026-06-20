use crate::keyboard;
use crate::types::*;
use std::{str::FromStr, sync::Arc};

pub const API_VERSION: &str = "5.199";

#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<reqwest::Client>,
}

#[derive(Clone)]
pub struct Bot {
    pub(crate) token: Arc<str>,
    pub(crate) group_id: Arc<str>,
    pub(crate) api_url: Arc<reqwest::Url>,
    pub(crate) client: Client,
}

impl Bot {
    pub fn new<S>(token: S, group_id: S) -> Self
    where
        S: Into<String>,
    {
        let api_url = reqwest::Url::from_str("https://api.vk.ru")
            .expect("hardcoded VK API URL should always be valid");
        Self {
            token: Arc::from(token.into()),
            group_id: Arc::from(group_id.into()),
            api_url: Arc::from(api_url),
            client: Client {
                inner: Arc::new(reqwest::Client::new()),
            },
        }
    }

    #[tracing::instrument(skip(self, message, keyboard), fields(peer_id = %peer_id))]
    pub async fn send_message(
        &self,
        peer_id: i64,
        message: &str,
        keyboard: Option<&keyboard::Keyboard>,
    ) -> Result<serde_json::Value, VkError> {
        let peer_id_str = peer_id.to_string();

        let mut params = vec![
            ("peer_id", peer_id_str.as_str()),
            ("message", message),
            ("random_id", "0"),
            ("v", API_VERSION),
        ];

        let keyboard_json;
        if let Some(kb) = keyboard {
            keyboard_json = serde_json::to_string(kb).unwrap_or_default();
            params.push(("keyboard", keyboard_json.as_str()));
        }

        tracing::debug!(
            message_len = message.len(),
            has_keyboard = keyboard.is_some(),
            "Sending message"
        );

        let url = format!("{}/method/messages.send", self.api_url);
        let response = self
            .client
            .inner
            .post(url)
            .bearer_auth(self.token.as_ref())
            .query(&params)
            .send()
            .await?
            .json::<Response<serde_json::Value>>()
            .await?;

        match response {
            Response::Ok { response } => {
                tracing::debug!("Message sent successfully");
                Ok(response)
            }
            Response::Err { error } => {
                tracing::error!(
                    error_code = error.error_code,
                    error_msg = %error.error_msg,
                    "VK API Error on messages.send"
                );
                Err(VkError::Api(error))
            }
        }
    }

    #[tracing::instrument(skip(self), fields(user_id = %user_id))]
    pub async fn get_user(&self, user_id: i64) -> Result<User, VkError> {
        let user_id_str = user_id.to_string();
        let params = vec![("user_ids", user_id_str.as_str()), ("v", API_VERSION)];

        tracing::debug!("Fetching user info");

        let url = format!("{}/method/users.get", self.api_url);
        let response = self
            .client
            .inner
            .post(url)
            .bearer_auth(self.token.as_ref())
            .query(&params)
            .send()
            .await?
            .json::<Response<Vec<User>>>()
            .await?;

        match response {
            Response::Ok { response } => {
                let user_opt = response.into_iter().next();
                if let Some(user) = &user_opt {
                    tracing::debug!(
                        first_name = %user.first_name,
                        last_name = %user.last_name,
                        "Fetched user info successfully"
                    );
                }
                user_opt.ok_or_else(|| {
                    VkError::Api(ApiError {
                        error_code: 0,
                        error_msg: "User not found".to_string(),
                        request_params: vec![],
                    })
                })
            }
            Response::Err { error } => {
                tracing::error!(
                    error_code = error.error_code,
                    error_msg = %error.error_msg,
                    "VK API Error on users.get"
                );
                Err(VkError::Api(error))
            }
        }
    }

    #[tracing::instrument(skip(self), fields(peer_id = %peer_id))]
    pub async fn get_conversation(&self, peer_id: i64) -> Result<Conversation, VkError> {
        let peer_ids_str = peer_id.to_string();
        let group_id_str = self.group_id.to_string();

        let params = vec![
            ("peer_ids", peer_ids_str.as_str()),
            ("group_id", group_id_str.as_str()),
            ("v", API_VERSION),
        ];

        tracing::debug!("Fetching conversation info");

        let url = format!("{}/method/messages.getConversationsById", self.api_url);
        let response = self
            .client
            .inner
            .post(url)
            .bearer_auth(self.token.as_ref())
            .query(&params)
            .send()
            .await?
            .json::<Response<ConversationsResponse>>()
            .await?;

        match response {
            Response::Ok { response } => {
                let conv_opt = response.items.into_iter().next();
                if let Some(conv) = &conv_opt {
                    tracing::debug!(
                        title = ?conv.chat_settings.as_ref().map(|s| &s.title),
                        "Fetched conversation successfully"
                    );
                }
                conv_opt.ok_or_else(|| {
                    VkError::Api(ApiError {
                        error_code: 0,
                        error_msg: "Conversation not found".to_string(),
                        request_params: vec![],
                    })
                })
            }
            Response::Err { error } => {
                tracing::error!(
                    error_code = error.error_code,
                    error_msg = %error.error_msg,
                    "VK API Error on messages.getConversationsById"
                );
                Err(VkError::Api(error))
            }
        }
    }

    #[tracing::instrument(skip(self, event_data), fields(event_id = %event_id, user_id = %user_id, peer_id = %peer_id))]
    pub async fn send_message_event_answer(
        &self,
        event_id: &str,
        user_id: i64,
        peer_id: i64,
        event_data: Option<&crate::event_data::EventData>,
    ) -> Result<serde_json::Value, VkError> {
        let user_id_str = user_id.to_string();
        let peer_id_str = peer_id.to_string();

        let mut params = vec![
            ("event_id", event_id),
            ("user_id", user_id_str.as_str()),
            ("peer_id", peer_id_str.as_str()),
            ("v", API_VERSION),
        ];

        let event_data_str;
        if let Some(da) = event_data {
            event_data_str = serde_json::to_string(da).unwrap_or_default();
            params.push(("event_data", event_data_str.as_str()));
        }

        tracing::debug!("Sending message event answer");

        let url = format!("{}/method/messages.sendMessageEventAnswer", self.api_url);
        let response = self
            .client
            .inner
            .post(url)
            .bearer_auth(self.token.as_ref())
            .query(&params)
            .send()
            .await?
            .json::<Response<serde_json::Value>>()
            .await?;

        match response {
            Response::Ok { response } => {
                tracing::debug!("Message event answer sent successfully");
                Ok(response)
            }
            Response::Err { error } => {
                tracing::error!(
                    error_code = error.error_code,
                    error_msg = %error.error_msg,
                    "VK API Error on messages.sendMessageEventAnswer"
                );
                Err(VkError::Api(error))
            }
        }
    }
}
