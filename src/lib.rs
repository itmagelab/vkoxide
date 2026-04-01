pub mod utils;

use std::{collections::HashMap, str::FromStr, sync::Arc};

#[derive(Clone)]
pub struct Client {
    inner: Arc<reqwest::Client>,
}

pub struct Bot {
    token: Arc<str>,
    api_url: Arc<reqwest::Url>,
    client: Client,
}

impl Bot {
    pub fn new<S>(token: S) -> Self
    where
        S: Into<String>,
    {
        let api_url = reqwest::Url::from_str("https://api.vk.ru").expect("");
        Self {
            token: Arc::from(token.into()),
            api_url: Arc::from(api_url),
            client: Client {
                inner: Arc::new(reqwest::Client::new()),
            },
        }
    }
}

pub struct Dispatcher {
    bot: Bot,
}
pub struct DispatcherBuilder {
    bot: Bot,
}

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct LongPollServer {
    pub server: String,
    pub key: String,
    pub ts: i64,
}

#[derive(Debug, Deserialize)]
pub struct LongPollResponse {
    pub ts: i64,
    pub updates: Vec<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    response: T,
}

#[derive(Debug)]
pub enum Event {
    Message(Message),
    Typing(Typing),
    Unknown(Vec<Value>),
}

#[derive(Debug)]
pub struct Message {
    pub message_id: i64,
    pub flags: i64,
    pub user_id: i64,
    pub ts: i64,
    pub text: String,
}

#[derive(Debug)]
pub struct Typing {
    pub user_id: i64,
    pub flags: i64,
}

impl Dispatcher {
    pub fn builder(bot: Bot) -> DispatcherBuilder {
        DispatcherBuilder { bot }
    }

    pub async fn server(&self) -> anyhow::Result<LongPollServer> {
        let token = &self.bot.token;

        let mut params = HashMap::new();
        params.insert("group_id", "0");
        params.insert("v", "5.199");

        let url = format!("{}/method/messages.getLongPollServer", self.bot.api_url);

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
        Ok(response.response)
    }

    pub async fn dispatch(self) -> anyhow::Result<()> {
        let server = self.server().await?;

        let mut ts = server.ts;

        loop {
            let ts_string = ts.to_string();
            let params = &[
                ("act", "a_check"),
                ("key", &server.key),
                ("wait", "25"),
                ("version", "3"),
                ("ts", ts_string.as_str()),
            ];

            let url = format!("https://{}", server.server);
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

            ts = response.ts;

            dbg!(&response.updates);
            for update in response.updates {
                match update.as_slice() {
                    [
                        Value::Number(code),
                        Value::Number(user_id),
                        Value::Number(flags),
                    ] if code.as_i64() == Some(61) => {
                        let user_id = user_id
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message user_id"))?;
                        let flags = flags
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message flags"))?;
                        Event::Typing(Typing { user_id, flags })
                    }
                    [
                        Value::Number(code),
                        Value::Number(message_id),
                        Value::Number(flags),
                        Value::Number(user_id),
                        Value::Number(ts),
                        Value::String(text),
                    ] if code.as_i64() == Some(4) => {
                        let message_id = message_id
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message message_id"))?;
                        let flags = flags
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message flags"))?;
                        let user_id = user_id
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message user_id"))?;
                        let ts = ts
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("Invalid message ts"))?;
                        let text = text.into();
                        Event::Message(Message {
                            message_id,
                            flags,
                            user_id,
                            ts,
                            text,
                        })
                    }
                    _ => Event::Unknown(vec![]),
                };
            }
        }
    }
}

impl DispatcherBuilder {
    pub fn build(self) -> Dispatcher {
        let Self { bot } = self;
        Dispatcher { bot }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        dotenvy::dotenv().ok();

        let token = std::env::var("VKOXIDE_TOKEN").unwrap();
        let bot = Bot::new(token);

        Dispatcher::builder(bot).build().dispatch().await.unwrap();
    }
}
