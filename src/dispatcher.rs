use crate::bot::{API_VERSION, Bot};
use crate::types::*;
use std::{future::Future, pin::Pin, sync::Arc};

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
