use std::sync::atomic::{AtomicUsize, Ordering};
use vkoxide::{
    Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind,
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
            |update: &Update| -> bool {
                matches!(
                    update.kind,
                    UpdateKind::Known(KnownUpdate::MessageNew { .. })
                )
            },
            |update: Update, ctx: Context<MyState>| async move {
                if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
                    let current_count = ctx.state.message_count.fetch_add(1, Ordering::Relaxed) + 1;

                    let user = ctx.bot.get_user(object.message.from_id).await?;

                    let conv = ctx.bot.get_conversation(object.message.peer_id).await?;
                    let chat_title = conv
                        .chat_settings
                        .map_or("Ваш диалог".to_string(), |s| s.title);

                    let answer = format!(
                        "{}, вы сказали: {}\nЧат: {}\nВы - {}-й написавший мне человек!",
                        user.first_name, object.message.text, chat_title, current_count
                    );

                    println!(
                        "Новое сообщение от {} {} в чате '{}'",
                        user.first_name, user.last_name, chat_title
                    );

                    let keyboard = Keyboard::new(false, true).add_row(vec![
                        KeyboardButton {
                            action: Action::Callback {
                                label: "Ответить Callback'ом".to_string(),
                                payload: Some("{\"btn\": 1}".to_string()),
                            },
                            color: Some(ButtonColor::Primary),
                        },
                        KeyboardButton {
                            action: Action::Text {
                                label: "Просто текст".to_string(),
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
            |update: &Update| -> bool {
                matches!(
                    update.kind,
                    UpdateKind::Known(KnownUpdate::MessageEvent { .. })
                )
            },
            |update: Update, ctx: Context<MyState>| async move {
                if let UpdateKind::Known(KnownUpdate::MessageEvent { object }) = update.kind {
                    println!(
                        "Получен callback от юзера {} с payload {:?}",
                        object.user_id, object.payload
                    );

                    let event_data = serde_json::json!({
                        "type": "show_snackbar",
                        "text": "Отправлено через Callback Event!"
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

    println!("Запускаем long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
}
