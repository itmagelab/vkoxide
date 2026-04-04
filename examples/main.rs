use vkoxide::{Bot, Context, Dispatcher, Event};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    let dispatcher = Dispatcher::builder(bot)
        .add_handler(
            |event: &Event| -> bool {
                // Простой фильтр: реагируем только на новые сообщения
                matches!(event, Event::MessageNew(_))
            },
            |event: Event, _ctx: Context<()>| {
                Box::pin(async move {
                    if let Event::MessageNew(msg) = event {
                        println!("Новое сообщение от {}: {}", msg.user_id, msg.text);
                    }
                    Ok(())
                })
            },
        )
        .build();

    println!("Запускаем long-poll dispatcher...");
    dispatcher.dispatch().await.unwrap();
}
