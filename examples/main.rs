use vkoxide::*;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let bot = Bot::new(token);

    Dispatcher::builder(bot).build().dispatch().await.unwrap();
}
