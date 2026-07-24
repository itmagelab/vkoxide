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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(35))
            .connect_timeout(std::time::Duration::from_secs(10))
            .tcp_keepalive(Some(std::time::Duration::from_secs(15)))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            token: Arc::from(token.into()),
            group_id: Arc::from(group_id.into()),
            api_url: Arc::from(api_url),
            client: Client {
                inner: Arc::new(client),
            },
        }
    }

    pub fn new_with_client<S>(token: S, group_id: S, client: reqwest::Client) -> Self
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
                inner: Arc::new(client),
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

    #[tracing::instrument(skip(self, message, keyboard), fields(peer_id = %peer_id))]
    pub async fn edit_message(
        &self,
        peer_id: i64,
        message_id: Option<i64>,
        conversation_message_id: Option<i64>,
        message: Option<&str>,
        keyboard: Option<&keyboard::Keyboard>,
    ) -> Result<serde_json::Value, VkError> {
        let peer_id_str = peer_id.to_string();
        let message_id_str;
        let cmid_str;

        let mut params = vec![("peer_id", peer_id_str.as_str()), ("v", API_VERSION)];

        if let Some(msg_id) = message_id {
            message_id_str = msg_id.to_string();
            params.push(("message_id", message_id_str.as_str()));
        }

        if let Some(cmid) = conversation_message_id {
            cmid_str = cmid.to_string();
            params.push(("conversation_message_id", cmid_str.as_str()));
        }

        if let Some(msg) = message {
            params.push(("message", msg));
        }

        let keyboard_json;
        if let Some(kb) = keyboard {
            keyboard_json = serde_json::to_string(kb).unwrap_or_default();
            params.push(("keyboard", keyboard_json.as_str()));
        }

        tracing::debug!(
            has_message_id = message_id.is_some(),
            has_cmid = conversation_message_id.is_some(),
            has_message = message.is_some(),
            has_keyboard = keyboard.is_some(),
            "Editing message"
        );

        let url = format!("{}/method/messages.edit", self.api_url);
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
                tracing::debug!("Message edited successfully");
                Ok(response)
            }
            Response::Err { error } => {
                tracing::error!(
                    error_code = error.error_code,
                    error_msg = %error.error_msg,
                    "VK API Error on messages.edit"
                );
                Err(VkError::Api(error))
            }
        }
    }

    /// Convenience method to edit a message by `conversation_message_id` (CMID).
    #[inline]
    pub async fn edit_message_by_cmid(
        &self,
        peer_id: i64,
        conversation_message_id: i64,
        message: Option<&str>,
        keyboard: Option<&keyboard::Keyboard>,
    ) -> Result<serde_json::Value, VkError> {
        self.edit_message(
            peer_id,
            None,
            Some(conversation_message_id),
            message,
            keyboard,
        )
        .await
    }

    /// Convenience method to edit a message by `message_id`.
    #[inline]
    pub async fn edit_message_by_id(
        &self,
        peer_id: i64,
        message_id: i64,
        message: Option<&str>,
        keyboard: Option<&keyboard::Keyboard>,
    ) -> Result<serde_json::Value, VkError> {
        self.edit_message(peer_id, Some(message_id), None, message, keyboard)
            .await
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
                if user_opt.is_some() {
                    tracing::debug!("Fetched user info successfully");
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
                if conv_opt.is_some() {
                    tracing::debug!("Fetched conversation successfully");
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

    /// Downloads a file from the given URL and returns its raw bytes.
    #[tracing::instrument(skip(self, url), fields(url = %url))]
    pub async fn download_file(&self, url: &str) -> Result<Vec<u8>, VkError> {
        tracing::debug!("Downloading file");
        let response = self
            .client
            .inner
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}
