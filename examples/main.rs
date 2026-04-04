use vkoxide::{Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    let dispatcher = Dispatcher::builder(bot)
        .add_handler(
            |update: &Update| -> bool {
                matches!(
                    update.kind,
                    UpdateKind::Known(KnownUpdate::MessageNew { .. })
                )
            },
            |update: Update, _ctx: Context<()>| {
                Box::pin(async move {
                    if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
                        println!(
                            "Новое сообщение от {}: {}",
                            object.message.from_id, object.message.text
                        );
                    }
                    Ok(())
                })
            },
        )
        .build();

    println!("Запускаем long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
}
