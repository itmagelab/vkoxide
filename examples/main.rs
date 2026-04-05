use std::sync::atomic::{AtomicU32, Ordering};
use vkoxide::{Bot, Context, Dispatcher, KnownUpdate, Update, UpdateKind, filters};

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

    let db = Database {
        call_count: AtomicU32::new(0),
    };

    let mut builder = Dispatcher::builder(bot);
    let shutdown_token = builder.shutdown_token();

    let dispatcher = builder
        .inject(State::Idle)
        .inject(db)
        .add_handler(
            filters::any_message(),
            |update: Update, ctx: Context| async move {
                if let UpdateKind::Known(KnownUpdate::MessageNew { object }) = update.kind {
                    let db = ctx.get::<Database>().unwrap();
                    let state = ctx.get::<State>().unwrap();

                    db.call_count.fetch_add(1, Ordering::SeqCst);
                    let count = db.call_count.load(Ordering::SeqCst);

                    // Fetch user info directly via API
                    let user = ctx.bot.get_user(object.message.from_id).await?;

                    let msg = match *state {
                        State::Idle => {
                            format!("Hello {}, you are message #{}!", user.first_name, count)
                        }
                    };

                    ctx.bot
                        .send_message(object.message.peer_id, &msg, None)
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

    println!("Starting long-poll dispatcher with DI...");
    dispatcher.dispatch().await.unwrap();
    println!("Dispatcher stopped.");
}
