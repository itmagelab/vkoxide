use vkoxide::*;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let token = std::env::var("VKOXIDE_TOKEN").unwrap();
    let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
    let bot = Bot::new(token, group_id);

    Dispatcher::builder(bot).build().dispatch().await.unwrap();
}
