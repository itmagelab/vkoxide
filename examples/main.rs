use std::sync::atomic::{AtomicUsize, Ordering};
use vkoxide::{Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind};

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

                    let answer = format!(
                        "Вы сказали: {}\nВы - {}-й написавший мне человек!",
                        object.message.text, current_count
                    );

                    println!(
                        "Новое сообщение от {}, это уже {} сообщение...",
                        object.message.from_id, current_count
                    );

                    ctx.bot
                        .send_message(object.message.from_id, &answer)
                        .await?;
                }
                Ok(())
            },
        )
        .build();

    println!("Запускаем long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
}
