pub mod keyboard;
pub mod utils;

use std::{future::Future, pin::Pin, str::FromStr, sync::Arc};

const API_VERSION: &str = "5.199";

#[derive(Clone)]
pub struct Client {
    inner: Arc<reqwest::Client>,
}

#[derive(Clone)]
pub struct Bot {
    token: Arc<str>,
    group_id: Arc<str>,
    api_url: Arc<reqwest::Url>,
    client: Client,
}

impl Bot {
    pub fn new<S>(token: S, group_id: S) -> Self
    where
        S: Into<String>,
    {
        let api_url = reqwest::Url::from_str("https://api.vk.ru").expect("");
        Self {
            token: Arc::from(token.into()),
            group_id: Arc::from(group_id.into()),
            api_url: Arc::from(api_url),
            client: Client {
                inner: Arc::new(reqwest::Client::new()),
            },
        }
    }

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
            Response::Ok { response } => Ok(response),
            Response::Err { error } => Err(VkError::Api(error)),
        }
    }

    pub async fn get_user(&self, user_id: i64) -> Result<User, VkError> {
        let user_id_str = user_id.to_string();
        let params = vec![("user_ids", user_id_str.as_str()), ("v", API_VERSION)];

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
            Response::Ok { response } => response.into_iter().next().ok_or_else(|| {
                VkError::Api(ApiError {
                    error_code: 0,
                    error_msg: "User not found".to_string(),
                    request_params: vec![],
                })
            }),
            Response::Err { error } => Err(VkError::Api(error)),
        }
    }

    pub async fn get_conversation(&self, peer_id: i64) -> Result<Conversation, VkError> {
        let peer_ids_str = peer_id.to_string();
        let group_id_str = self.group_id.to_string();

        let params = vec![
            ("peer_ids", peer_ids_str.as_str()),
            ("group_id", group_id_str.as_str()),
            ("v", API_VERSION),
        ];

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
            Response::Ok { response } => response.items.into_iter().next().ok_or_else(|| {
                VkError::Api(ApiError {
                    error_code: 0,
                    error_msg: "Conversation not found".to_string(),
                    request_params: vec![],
                })
            }),
            Response::Err { error } => Err(VkError::Api(error)),
        }
    }

    pub async fn send_message_event_answer(
        &self,
        event_id: &str,
        user_id: i64,
        peer_id: i64,
        event_data: Option<serde_json::Value>,
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
            event_data_str = serde_json::to_string(&da).unwrap_or_default();
            params.push(("event_data", event_data_str.as_str()));
        }

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
            Response::Ok { response } => Ok(response),
            Response::Err { error } => Err(VkError::Api(error)),
        }
    }
}

#[derive(Clone)]
pub struct Context<State = ()> {
    pub bot: Bot,
    pub state: Arc<State>,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type Filter<U> = Box<dyn Fn(&U) -> bool + Send + Sync>;
pub type Handler<U, S> =
    Box<dyn Fn(U, Context<S>) -> BoxFuture<'static, Result<(), VkError>> + Send + Sync>;

pub struct Dispatcher<S = ()> {
    bot: Bot,
    state: Arc<S>,
    handlers: Vec<(Filter<Update>, Handler<Update, S>)>,
}
pub struct DispatcherBuilder<S = ()> {
    bot: Bot,
    state: Arc<S>,
    handlers: Vec<(Filter<Update>, Handler<Update, S>)>,
}

use serde::Deserialize;
use serde_json::Value;

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
    pub version: i64,
}

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
}

impl Dispatcher<()> {
    pub fn builder(bot: Bot) -> DispatcherBuilder<()> {
        DispatcherBuilder::new(bot)
    }
}

impl<S: Send + Sync + 'static> Dispatcher<S> {
    pub async fn server(&self) -> Result<LongPollServer, VkError> {
        let token = &self.bot.token;
        let group_id = self.bot.group_id.to_string();

        let params = [("group_id", group_id.as_str()), ("v", API_VERSION)];

        let url = format!("{}/method/groups.getLongPollServer", self.bot.api_url);
        let response = self
            .bot
            .client
            .inner
            .get(url)
            .bearer_auth(token)
            .query(&params)
            .send()
            .await?
            .json::<Response<LongPollServer>>()
            .await?;

        match response {
            Response::Ok { response } => Ok(response),
            Response::Err { error } => Err(VkError::Api(error)),
        }
    }

    pub async fn dispatch(self) -> Result<(), VkError> {
        let server = self.server().await?;

        let mut ts = server.ts;

        loop {
            let ts_string = ts.to_string();
            let params = &[
                ("act", "a_check"),
                ("key", &server.key),
                ("wait", "25"),
                ("ts", ts_string.as_str()),
            ];

            let url = server.server.to_string();

            let response = self
                .bot
                .client
                .inner
                .get(url)
                .query(&params)
                .send()
                .await?
                .json::<LongPollResponse>()
                .await?;

            ts = response.ts.to_string();

            for update in response.updates {
                println!("Update: {:#?}", update);
                let update_clone = update.clone();

                for (filter, handler) in &self.handlers {
                    if filter(&update_clone) {
                        let ctx = Context {
                            bot: self.bot.clone(),
                            state: self.state.clone(),
                        };
                        handler(update_clone.clone(), ctx).await?;
                        break;
                    }
                }
            }
        }
    }
}

impl DispatcherBuilder<()> {
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            state: Arc::new(()),
            handlers: vec![],
        }
    }
}

impl<S: Send + Sync + 'static> DispatcherBuilder<S> {
    pub fn state<NewState>(self, state: NewState) -> DispatcherBuilder<NewState> {
        DispatcherBuilder {
            bot: self.bot,
            state: Arc::new(state),
            handlers: vec![],
        }
    }

    pub fn add_handler<F, H, Fut>(mut self, filter: F, handler: H) -> Self
    where
        F: Fn(&Update) -> bool + Send + Sync + 'static,
        H: Fn(Update, Context<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), VkError>> + Send + 'static,
    {
        let boxed_handler = Box::new(move |update, ctx| {
            Box::pin(handler(update, ctx)) as BoxFuture<'static, Result<(), VkError>>
        });
        self.handlers.push((Box::new(filter), boxed_handler));
        self
    }

    pub fn build(self) -> Dispatcher<S> {
        Dispatcher {
            bot: self.bot,
            state: self.state,
            handlers: self.handlers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        dotenvy::dotenv().ok();

        let token = std::env::var("VKOXIDE_TOKEN").unwrap();
        let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
        let bot = Bot::new(token, group_id);

        Dispatcher::builder(bot).build().dispatch().await.unwrap();
    }
}
