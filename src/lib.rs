pub mod bot;
pub mod dispatcher;
pub mod filters;
pub mod keyboard;
pub mod prelude;
pub mod types;
pub mod utils;

pub use bot::Bot;
pub use dispatcher::{Context, Dispatcher};
pub use types::{KnownUpdate, Update, UpdateKind, VkError};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        dotenvy::dotenv().ok();

        let token = std::env::var("VKOXIDE_TOKEN").unwrap();
        let group_id = std::env::var("VKOXIDE_GROUP_ID").unwrap();
        let bot = Bot::new(token, group_id);

        Dispatcher::builder(bot).build().dispatch().await.unwrap();
    }
}
