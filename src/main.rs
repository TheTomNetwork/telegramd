use telegramd::chat_data::ChatData;
use telegramd::command::Command;
use telegramd::files::send_file;
use telegramd::message::send_message;
use telegramd::UPLOADED_FILES;

use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use dotenv::dotenv;
use std::env;
use teloxide::prelude::*;
use tokio::{fs, join};

#[tokio::main]
async fn main() -> Result<()> {
    let status_dotenv = dotenv();
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    if let Err(e) = status_dotenv {
        log::warn!("Failed to load .env file: {:?}", &e);
    }

    if env::var("TELOXIDE_TOKEN").is_err() {
        log::error!("The telegram bot token isn't set with $TELOXIDE_TOKEN");
        return Ok(());
    }

    let cwd = env::current_dir().context("Failed to get current working directory")?;
    let upload_dir = cwd.join(UPLOADED_FILES);
    log::info!("Using upload directory: {}", upload_dir.to_string_lossy());

    // Create the upload directory if it doesn't exist
    fs::create_dir_all(&upload_dir)
        .await
        .with_context(|| format!("Failed to create upload directory at {:?}", upload_dir))?;

    log::info!("Started telegramd");

    // Telegram bot command listener
    let command_listener = Command::repl(Bot::from_env(), Command::answer);

    let upload_dir_clone = upload_dir.clone();
    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Bot::from_env())) // Pass bot to handlers
            .app_data(web::Data::new(upload_dir_clone.clone())) // Pass upload_dir to handlers
            .route(
                "/send-message",
                web::get().to(send_message::<web::Query<ChatData<String>>>),
            )
            .route(
                "/send-message",
                web::post().to(send_message::<web::Json<ChatData<String>>>),
            )
            .route("/send-file", web::put().to(send_file))
    })
    .bind("127.0.0.1:5005")
    .with_context(|| "Failed to bind HTTP server to 127.0.0.1:5005")?
    .run();

    // Run both the command listener and HTTP server concurrently
    join!(command_listener, http_server).1?;

    Ok(())
}
