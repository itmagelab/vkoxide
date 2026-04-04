pub mod utils;

use std::{collections::HashMap, future::Future, pin::Pin, str::FromStr, sync::Arc};

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
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypingStateObject {
    pub from_id: i64,
    pub to_id: i64,
    pub state: String,
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

        let mut params = HashMap::new();
        params.insert("group_id", group_id.as_str());
        params.insert("v", API_VERSION);

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

    pub fn add_handler<F, H>(mut self, filter: F, handler: H) -> Self
    where
        F: Fn(&Update) -> bool + Send + Sync + 'static,
        H: Fn(Update, Context<S>) -> BoxFuture<'static, Result<(), VkError>>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.push((Box::new(filter), Box::new(handler)));
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
