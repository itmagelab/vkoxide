use crate::bot::{API_VERSION, Bot};
use crate::types::*;
pub use dptree::di::{DependencyMap, Injectable};
pub use dptree::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct ShutdownToken {
    tx: mpsc::UnboundedSender<()>,
}

impl ShutdownToken {
    pub fn shutdown(self) -> Result<(), VkError> {
        Ok(self.tx.send(())?)
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type HandlerResult = Result<(), BoxError>;

pub struct Dispatcher {
    bot: Bot,
    deps: DependencyMap,
    handler: Arc<dptree::Handler<'static, HandlerResult>>,
    shutdown_rx: Option<mpsc::UnboundedReceiver<()>>,
}

pub struct DispatcherBuilder {
    bot: Bot,
    deps: DependencyMap,
    handler: Option<Arc<dptree::Handler<'static, HandlerResult>>>,
    shutdown: Option<(mpsc::UnboundedSender<()>, mpsc::UnboundedReceiver<()>)>,
}

async fn wait_shutdown(rx: &mut Option<mpsc::UnboundedReceiver<()>>) {
    if let Some(rx) = rx {
        rx.recv().await;
    } else {
        std::future::pending::<()>().await;
    }
}

async fn sleep_or_shutdown(duration: std::time::Duration, rx: &mut Option<mpsc::UnboundedReceiver<()>>) -> bool {
    tokio::select! {
        _ = wait_shutdown(rx) => true,
        _ = tokio::time::sleep(duration) => false,
    }
}

impl Dispatcher {
    pub fn builder(bot: Bot) -> DispatcherBuilder {
        DispatcherBuilder::new(bot)
    }

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

    async fn poll_once(
        &self,
        server: &LongPollServer,
        ts: &str,
        shutdown_rx: &mut Option<mpsc::UnboundedReceiver<()>>,
        retry_delay: &mut u64,
    ) -> Option<LongPollResponse> {
        loop {
            let ts_string = ts.to_string();
            let params = &[
                ("act", "a_check"),
                ("key", &server.key),
                ("wait", "25"),
                ("ts", ts_string.as_str()),
            ];

            let url = server.server.to_string();

            tracing::debug!(
                server = %url,
                ts = %ts_string,
                "Sending LongPoll request"
            );

            let response_result = tokio::select! {
                _ = wait_shutdown(shutdown_rx) => {
                    return None;
                }
                res = self.bot.client.inner.get(url).query(params).send() => {
                    res
                }
            };

            let response_http = match response_result {
                Ok(resp) => resp,
                Err(err) => {
                    tracing::error!(error = %err, "LongPoll request failed, retrying...");
                    if sleep_or_shutdown(std::time::Duration::from_secs(*retry_delay), shutdown_rx).await {
                        return None;
                    }
                    *retry_delay = std::cmp::min(*retry_delay * 2, 30);
                    continue;
                }
            };

            tracing::debug!(
                status = %response_http.status(),
                "LongPoll request finished"
            );

            let response_text = match response_http.text().await {
                Ok(text) => text,
                Err(err) => {
                    tracing::error!(error = %err, "Failed to read LongPoll response text, retrying...");
                    if sleep_or_shutdown(std::time::Duration::from_secs(*retry_delay), shutdown_rx).await {
                        return None;
                    }
                    *retry_delay = std::cmp::min(*retry_delay * 2, 30);
                    continue;
                }
            };

            tracing::trace!(raw_response = %response_text, "Received raw LongPoll response");

            match serde_json::from_str::<LongPollResponse>(&response_text) {
                Ok(resp) => return Some(resp),
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        raw_response = %response_text,
                        "Failed to parse LongPoll response, retrying..."
                    );
                    if sleep_or_shutdown(std::time::Duration::from_secs(*retry_delay), shutdown_rx).await {
                        return None;
                    }
                    *retry_delay = std::cmp::min(*retry_delay * 2, 30);
                }
            }
        }
    }

    async fn handle_longpoll_failure(
        &self,
        failed_code: i32,
        response_ts: Option<String>,
        server: &mut LongPollServer,
        ts: &mut String,
        shutdown_rx: &mut Option<mpsc::UnboundedReceiver<()>>,
        retry_delay: &mut u64,
    ) -> Result<ControlFlow<(), ()>, VkError> {
        match failed_code {
            1 => {
                if let Some(new_ts) = response_ts {
                    *ts = new_ts;
                } else {
                    tracing::error!("failed: 1 was returned but new ts is missing");
                }
            }
            2 | 3 => {
                tracing::info!("Requesting new LongPoll server...");
                match self.server().await {
                    Ok(new_server) => {
                        *server = new_server;
                        *ts = server.ts.clone();
                        *retry_delay = 1;
                    }
                    Err(err) => {
                        tracing::error!(error = %err, "Failed to request new LongPoll server");
                        if sleep_or_shutdown(std::time::Duration::from_secs(*retry_delay), shutdown_rx).await {
                            return Ok(ControlFlow::Break(()));
                        }
                        *retry_delay = std::cmp::min(*retry_delay * 2, 30);
                    }
                }
            }
            _ => {
                if sleep_or_shutdown(std::time::Duration::from_secs(*retry_delay), shutdown_rx).await {
                    return Ok(ControlFlow::Break(()));
                }
                *retry_delay = std::cmp::min(*retry_delay * 2, 30);
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    async fn process_update(&self, update: Update) {
        let update_type = match &update.kind {
            UpdateKind::Known(KnownUpdate::MessageNew { .. }) => "message_new",
            UpdateKind::Known(KnownUpdate::MessageReply { .. }) => "message_reply",
            UpdateKind::Known(KnownUpdate::MessageTypingState { .. }) => {
                "message_typing_state"
            }
            UpdateKind::Known(KnownUpdate::MessageRead { .. }) => "message_read",
            UpdateKind::Known(KnownUpdate::MessageEvent { .. }) => "message_event",
            UpdateKind::Unknown(_) => "unknown",
        };

        let span = tracing::info_span!(
            "vkoxide_update",
            event_id = %update.event_id,
            update_type = %update_type,
        );

        let mut deps = self.deps.clone();
        deps.insert(update.clone());
        deps.insert(self.bot.clone());

        let handler = self.handler.clone();
        let process_update = async move {
            match &update.kind {
                UpdateKind::Known(KnownUpdate::MessageNew { object }) => {
                    tracing::info!(
                        peer_id = %object.message.peer_id,
                        from_id = %object.message.from_id,
                        text_len = %object.message.text.len(),
                        conversation_message_id = %object.message.conversation_message_id,
                        attachments_count = %object.message.attachments.len(),
                        has_voice = %object.message.voice_message().is_some(),
                        "Received new message"
                    );
                }
                UpdateKind::Known(KnownUpdate::MessageReply { object }) => {
                    tracing::info!(
                        peer_id = %object.peer_id,
                        from_id = %object.from_id,
                        text_len = %object.text.len(),
                        conversation_message_id = %object.conversation_message_id,
                        attachments_count = %object.attachments.len(),
                        has_voice = %object.voice_message().is_some(),
                        "Received reply message"
                    );
                }
                UpdateKind::Known(KnownUpdate::MessageTypingState { object }) => {
                    tracing::debug!(
                        from_id = %object.from_id,
                        to_id = %object.to_id,
                        state = %object.state,
                        "Received typing state update"
                    );
                }
                UpdateKind::Known(KnownUpdate::MessageRead { object }) => {
                    tracing::debug!(
                        from_id = %object.from_id,
                        peer_id = %object.peer_id,
                        read_message_id = %object.read_message_id,
                        "Received message read update"
                    );
                }
                UpdateKind::Known(KnownUpdate::MessageEvent { object }) => {
                    tracing::info!(
                        user_id = %object.user_id,
                        peer_id = %object.peer_id,
                        event_id = %object.event_id,
                        "Received message event"
                    );
                }
                UpdateKind::Unknown(value) => {
                    tracing::warn!(
                        raw_value = ?value,
                        "Received unknown update event"
                    );
                }
            }

            match handler.dispatch(deps).await {
                ControlFlow::Break(Ok(_)) => {
                    tracing::debug!("Update processed successfully");
                }
                ControlFlow::Break(Err(e)) => {
                    tracing::error!("Handler error: {}", e);
                }
                ControlFlow::Continue(_) => {
                    tracing::debug!("Update was not handled by any branch");
                }
            }
        };

        use tracing::Instrument;
        process_update.instrument(span).await;
    }

    pub async fn dispatch(mut self) -> Result<(), VkError> {
        let mut server = self.server().await?;
        tracing::info!("LongPoll server: {}", server.server);

        let mut ts = server.ts.clone();
        let mut shutdown_rx = self.shutdown_rx.take();
        let mut retry_delay = 1;

        loop {
            let Some(response) = self.poll_once(&server, &ts, &mut shutdown_rx, &mut retry_delay).await else {
                break;
            };

            if let Some(failed_code) = response.failed {
                tracing::warn!(failed_code = failed_code, "LongPoll returned failed code");
                if self.handle_longpoll_failure(failed_code, response.ts, &mut server, &mut ts, &mut shutdown_rx, &mut retry_delay).await?.is_break() {
                    break;
                }
                continue;
            }

            let Some(response_ts) = response.ts else {
                tracing::error!("LongPoll response missing ts");
                continue;
            };
            ts = response_ts;

            let updates = response.updates.unwrap_or_default();
            tracing::debug!(updates_count = updates.len(), "LongPoll returned updates");
            retry_delay = 1;

            for update in updates {
                self.process_update(update).await;
            }
        }

        Ok(())
    }
}

impl DispatcherBuilder {
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            deps: DependencyMap::new(),
            handler: None,
            shutdown: None,
        }
    }

    pub fn inject<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        self.deps.insert(data);
        self
    }

    pub fn shutdown_token(&mut self) -> ShutdownToken {
        let (tx, rx) = mpsc::unbounded_channel();
        self.shutdown = Some((tx.clone(), rx));
        ShutdownToken { tx }
    }

    pub fn handler(mut self, handler: dptree::Handler<'static, HandlerResult>) -> Self {
        self.handler = Some(Arc::new(handler));
        self
    }

    pub fn build(self) -> Dispatcher {
        Dispatcher {
            bot: self.bot,
            deps: self.deps,
            handler: self
                .handler
                .unwrap_or_else(|| Arc::new(dptree::entry().endpoint(|| async { Ok(()) }))),
            shutdown_rx: self.shutdown.map(|(_, rx)| rx),
        }
    }
}
