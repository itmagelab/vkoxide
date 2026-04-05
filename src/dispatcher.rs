use crate::bot::{API_VERSION, Bot};
use crate::types::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Default, Clone)]
pub struct DependencyMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl DependencyMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, data: T) {
        self.map.insert(TypeId::of::<T>(), Arc::new(data));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|any| any.clone().downcast::<T>().ok())
    }
}

pub struct Context {
    pub bot: Bot,
    pub(crate) data: Arc<DependencyMap>,
}

impl Context {
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.data.get::<T>()
    }
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type Filter<U> = Box<dyn Fn(&U) -> bool + Send + Sync>;
pub type Handler<U> =
    Box<dyn Fn(U, Context) -> BoxFuture<'static, Result<(), VkError>> + Send + Sync>;

#[derive(Clone)]
pub struct ShutdownToken {
    tx: mpsc::UnboundedSender<()>,
}

impl ShutdownToken {
    pub fn shutdown(self) -> Result<(), VkError> {
        self.tx.send(()).map_err(|_| VkError::Shutdown)
    }
}

pub struct Dispatcher {
    bot: Bot,
    data: Arc<DependencyMap>,
    handlers: Vec<(Filter<Update>, Handler<Update>)>,
    shutdown: Option<mpsc::UnboundedReceiver<()>>,
}

pub struct DispatcherBuilder {
    bot: Bot,
    data: DependencyMap,
    handlers: Vec<(Filter<Update>, Handler<Update>)>,
    shutdown: Option<(mpsc::UnboundedSender<()>, mpsc::UnboundedReceiver<()>)>,
}

impl Dispatcher {
    pub fn builder(bot: Bot) -> DispatcherBuilder {
        DispatcherBuilder::new(bot)
    }
}

impl Dispatcher {
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

    pub async fn dispatch(mut self) -> Result<(), VkError> {
        let server = self.server().await?;

        let mut ts = server.ts;
        let mut shutdown = self.shutdown.take();

        loop {
            let ts_string = ts.to_string();
            let params = &[
                ("act", "a_check"),
                ("key", &server.key),
                ("wait", "25"),
                ("ts", ts_string.as_str()),
            ];

            let url = server.server.to_string();

            let response = tokio::select! {
                _ = async {
                    if let Some(rx) = &mut shutdown {
                        rx.recv().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    break;
                }
                res = self.bot.client.inner.get(url).query(params).send() => {
                    res?.json::<LongPollResponse>().await?
                }
            };

            ts = response.ts.to_string();

            for update in response.updates {
                let update_clone = update.clone();

                for (filter, handler) in &self.handlers {
                    if filter(&update_clone) {
                        let ctx = Context {
                            bot: self.bot.clone(),
                            data: self.data.clone(),
                        };
                        handler(update_clone.clone(), ctx).await?;
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

impl DispatcherBuilder {
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            data: DependencyMap::new(),
            handlers: vec![],
            shutdown: None,
        }
    }

    pub fn inject<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        self.data.insert(data);
        self
    }

    pub fn shutdown_token(&mut self) -> ShutdownToken {
        let (tx, rx) = mpsc::unbounded_channel();
        self.shutdown = Some((tx.clone(), rx));
        ShutdownToken { tx }
    }

    pub fn add_handler<F, H, Fut>(mut self, filter: F, handler: H) -> Self
    where
        F: Fn(&Update) -> bool + Send + Sync + 'static,
        H: Fn(Update, Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), VkError>> + Send + 'static,
    {
        let boxed_handler = Box::new(move |update, ctx| {
            Box::pin(handler(update, ctx)) as BoxFuture<'static, Result<(), VkError>>
        });
        self.handlers.push((Box::new(filter), boxed_handler));
        self
    }

    pub fn build(self) -> Dispatcher {
        Dispatcher {
            bot: self.bot,
            data: Arc::new(self.data),
            handlers: self.handlers,
            shutdown: self.shutdown.map(|(_, rx)| rx),
        }
    }
}
