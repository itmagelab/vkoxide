use std::sync::atomic::{AtomicUsize, Ordering};
use vkoxide::{
    Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind, filters,
    keyboard::{Action, ButtonColor, Keyboard, KeyboardButton},
};

struct MyState {
    pub message_count: AtomicUsize,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    let app_state = MyState {
        message_count: AtomicUsize::new(0),
    };

    let dispatcher = Dispatcher::builder(bot)
        .state(app_state)
        .add_handler(
            filters::any_message(),
            |update: Update, ctx: Context<MyState>| async move {
                if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
                    let current_count = ctx.state.message_count.fetch_add(1, Ordering::Relaxed) + 1;

                    // Fetch user info directly via API
                    let user = ctx.bot.get_user(object.message.from_id).await?;

                    // Fetch current conversation info
                    let conv = ctx.bot.get_conversation(object.message.peer_id).await?;
                    let chat_title = conv
                        .chat_settings
                        .map_or("Direct messages".to_string(), |s| s.title);

                    let answer = format!(
                        "{}, you said: {}\nChat: {}\nYou are the {}-th person writing to me!",
                        user.first_name, object.message.text, chat_title, current_count
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
            |update: Update, ctx: Context<MyState>| async move {
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

    println!("Starting long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
}
