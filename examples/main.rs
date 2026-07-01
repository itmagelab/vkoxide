use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use vkoxide::prelude::*;

#[derive(Clone, Copy)]
pub enum State {
    Idle,
}

// Mock database to demonstrate multiple dependencies
pub struct Database {
    pub call_count: AtomicU32,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vkoxide=debug,info".into()),
        )
        .init();

    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    let db = Arc::new(Database {
        call_count: AtomicU32::new(0),
    });

    let mut builder = Dispatcher::builder(bot);
    let shutdown_token = builder.shutdown_token();

    let dispatcher = builder
        .inject(State::Idle)
        .inject(db)
        .handler(schema())
        .build();

    // Graceful shutdown on Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        tracing::info!("Shutdown signal received. Gracefully stopping...");
        shutdown_token.shutdown().unwrap();
    });

    tracing::info!("Starting long-poll dispatcher with dptree branching...");
    dispatcher.dispatch().await.unwrap();
    tracing::info!("Dispatcher stopped.");
}

fn schema() -> dptree::Handler<'static, HandlerResult> {
    dptree::entry()
        // Handle the initial "Start" button press (payload command)
        .branch(filter::is_start().endpoint(handle_start))
        // Handle "/help" command
        .branch(filter::command("/help").endpoint(handle_help))
        // Handle "/persistent" command to show a persistent reply keyboard
        .branch(filter::command("/persistent").endpoint(handle_persistent))
        // Handle "/close" command or "Close Keyboard" button press to hide it
        .branch(filter::command("/close").endpoint(handle_close))
        .branch(filter::is_text("Close Keyboard").endpoint(handle_close))
        // Handle exact text "hello"
        .branch(filter::is_text("hello").endpoint(handle_hello))
        // Handle inline callback button presses
        .branch(filter::is_callback().endpoint(handle_callback))
        // Handle voice messages (audiomsg)
        .branch(filter::voice_message().endpoint(handle_voice))
        // Fallback: handle any other message
        .branch(filter::any_message().endpoint(handle_message))
}

/// Build an inline keyboard with a "Show info" callback button
fn info_keyboard() -> Keyboard {
    Keyboard::new(false, true).add_row(vec![KeyboardButton {
        action: Action::Callback {
            label: "Show info".to_string(),
            payload: Some(r#"{"action":"info"}"#.to_string()),
        },
        color: None,
    }])
}

/// Build a reply keyboard with text buttons (one-time keyboard)
#[allow(dead_code)]
fn menu_keyboard() -> Keyboard {
    Keyboard::new(true, false).add_row(vec![
        KeyboardButton {
            action: Action::Text {
                label: "Help".to_string(),
                payload: Some(r#"{"command":"help"}"#.to_string()),
            },
            color: Some(ButtonColor::Primary),
        },
        KeyboardButton {
            action: Action::Text {
                label: "Hello".to_string(),
                payload: None,
            },
            color: Some(ButtonColor::Secondary),
        },
    ])
}

/// Build a persistent reply keyboard with text buttons (will stay under the input field)
fn persistent_keyboard() -> Keyboard {
    Keyboard::new(false, false).add_row(vec![
        KeyboardButton {
            action: Action::Text {
                label: "Hello".to_string(),
                payload: None,
            },
            color: Some(ButtonColor::Primary),
        },
        KeyboardButton {
            action: Action::Text {
                label: "Close Keyboard".to_string(),
                payload: None,
            },
            color: Some(ButtonColor::Negative),
        },
    ])
}

async fn handle_start(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    let kb = persistent_keyboard();
    bot.send_message(
        obj.message.peer_id,
        "Welcome! Use the buttons below or type /help.",
        Some(&kb),
    )
    .await?;
    Ok(())
}

async fn handle_help(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    let kb = info_keyboard();
    bot.send_message(
        obj.message.peer_id,
        "Available commands:\n/help — show this message\nhello — greet the bot\n/persistent — show a persistent keyboard\n/close — remove the keyboard\n\nOr press the button below.",
        Some(&kb),
    )
    .await?;
    Ok(())
}

async fn handle_persistent(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    let kb = persistent_keyboard();
    bot.send_message(
        obj.message.peer_id,
        "This is a persistent keyboard. It will stay visible under the input field until you close it.",
        Some(&kb),
    )
    .await?;
    Ok(())
}

async fn handle_close(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    // To remove the reply keyboard, we send a Keyboard with empty buttons
    let kb = Keyboard::new(false, false);
    bot.send_message(
        obj.message.peer_id,
        "Removing the keyboard...",
        Some(&kb),
    )
    .await?;
    Ok(())
}

async fn handle_hello(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    bot.send_message(obj.message.peer_id, "Hello there! 👋", None)
        .await?;
    Ok(())
}

async fn handle_callback(bot: Bot, obj: MessageEventObject) -> HandlerResult {
    // Respond to the inline callback with a notification
    bot.send_message_event_answer(
        &obj.event_id,
        obj.user_id,
        obj.peer_id,
        Some(&EventData::show_snackbar("Here is your info!")),
    )
    .await?;

    // Also send a follow-up message
    bot.send_message(obj.peer_id, "You requested info. Here it is!", None)
        .await?;
    Ok(())
}

async fn handle_voice(bot: Bot, obj: MessageNewObject, voice: AudioMessage) -> HandlerResult {
    tracing::info!(
        "Received voice message with ID {} from owner {}, duration {} seconds.",
        voice.id,
        voice.owner_id,
        voice.duration
    );

    match bot.download_file(&voice.link_ogg).await {
        Ok(bytes) => {
            let msg = format!(
                "Получено голосовое сообщение!\nПродолжительность: {} сек.\nУспешно скачан файл: {} байт (OGG).",
                voice.duration,
                bytes.len()
            );
            bot.send_message(obj.message.peer_id, &msg, None).await?;
        }
        Err(err) => {
            tracing::error!("Failed to download voice file: {:?}", err);
            bot.send_message(
                obj.message.peer_id,
                "Голосовое сообщение получено, но не удалось загрузить аудиофайл.",
                None,
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_message(
    bot: Bot,
    db: Arc<Database>,
    state: State,
    obj: MessageNewObject,
) -> HandlerResult {
    db.call_count.fetch_add(1, Ordering::SeqCst);
    let count = db.call_count.load(Ordering::SeqCst);

    // Fetch user info directly via API
    let user = bot.get_user(obj.message.from_id).await?;

    let msg = match state {
        State::Idle => {
            format!("Hello {}, you are message #{}!", user.first_name, count)
        }
    };

    bot.send_message(obj.message.peer_id, &msg, None).await?;

    Ok(())
}
