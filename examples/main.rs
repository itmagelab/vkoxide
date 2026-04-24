use dptree::di::DependencyMap;
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
        println!("\nShutdown signal received. Gracefully stopping...");
        shutdown_token.shutdown().unwrap();
    });

    println!("Starting long-poll dispatcher with dptree branching...");
    dispatcher.dispatch().await.unwrap();
    println!("Dispatcher stopped.");
}

fn schema() -> dptree::Handler<'static, DependencyMap, HandlerResult> {
    dptree::entry()
        // Handle the initial "Start" button press (payload command)
        .branch(filter::is_start().endpoint(handle_start))
        // Handle "/help" command
        .branch(filter::command("/help").endpoint(handle_help))
        // Handle exact text "hello"
        .branch(filter::is_text("hello").endpoint(handle_hello))
        // Handle inline callback button presses
        .branch(filter::is_callback().endpoint(handle_callback))
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

/// Build a reply keyboard with text buttons
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

async fn handle_start(bot: Bot, obj: MessageNewObject) -> HandlerResult {
    let kb = menu_keyboard();
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
        "Available commands:\n/help — show this message\nhello — greet the bot\n\nOr press the button below.",
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
        Some(serde_json::json!({ "type": "show_snackbar", "text": "Here is your info!" })),
    )
    .await?;

    // Also send a follow-up message
    bot.send_message(obj.peer_id, "You requested info. Here it is!", None)
        .await?;
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
