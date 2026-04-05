use vkoxide::{
    Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind, filters,
    keyboard::{Action, ButtonColor, Keyboard, KeyboardButton},
};

pub enum State {
    Start,
    Idle,
    Flow(String),
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    let app_state = State::Idle;

    let mut builder = Dispatcher::builder(bot);
    let shutdown_token = builder.shutdown_token();

    let dispatcher = builder
        .state(app_state)
        .add_handler(
            filters::any_message(),
            |update: Update, ctx: Context<State>| async move {
                if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
                    // Fetch user info directly via API
                    let user = ctx.bot.get_user(object.message.from_id).await?;

                    // Fetch current conversation info
                    let conv = ctx.bot.get_conversation(object.message.peer_id).await?;
                    let chat_title = conv
                        .chat_settings
                        .map_or("Direct messages".to_string(), |s| s.title);

                    let answer = format!(
                        "{}, you said: {}\nChat: {}\n",
                        user.first_name, object.message.text, chat_title
                    );

                    println!(
                        "New message from {} {} in chat '{}'",
                        user.first_name, user.last_name, chat_title
                    );

                    let keyboard = Keyboard::new(false, true).add_row(vec![
                        KeyboardButton {
                            action: Action::Callback {
                                label: "Answer with Callback".to_string(),
                                payload: Some("{\"btn\": 1}".to_string()),
                            },
                            color: Some(ButtonColor::Primary),
                        },
                        KeyboardButton {
                            action: Action::Text {
                                label: "Plain Text".to_string(),
                                payload: None,
                            },
                            color: Some(ButtonColor::Negative),
                        },
                    ]);

                    ctx.bot
                        .send_message(object.message.from_id, &answer, Some(&keyboard))
                        .await?;
                }
                Ok(())
            },
        )
        .add_handler(
            filters::is_callback(),
            |update: Update, ctx: Context<State>| async move {
                if let UpdateKind::Known(KnownUpdate::MessageEvent { object }) = update.kind {
                    println!(
                        "Received callback from user {} with payload {:?}",
                        object.user_id, object.payload
                    );

                    let event_data = serde_json::json!({
                        "type": "show_snackbar",
                        "text": "Sent via Callback Event!"
                    });

                    ctx.bot
                        .send_message_event_answer(
                            &object.event_id,
                            object.user_id,
                            object.peer_id,
                            Some(event_data),
                        )
                        .await?;
                }
                Ok(())
            },
        )
        .build();

    // Graceful shutdown on Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("\nShutdown signal received. Gracefully stopping...");
        shutdown_token.shutdown().unwrap();
    });

    println!("Starting long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
    println!("Dispatcher stopped.");
}
