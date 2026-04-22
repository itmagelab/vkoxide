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
        .branch(filter::is_start().endpoint(handle_message))
        .branch(filter::any_message().endpoint(handle_message))
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
