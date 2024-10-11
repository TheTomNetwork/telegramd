use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use dotenv::dotenv;
use futures::StreamExt;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
};
use teloxide::{
    macros,
    prelude::*,
    types::{InputFile, ParseMode},
    utils::command::BotCommands,
};
use tokio::join;

const UPLOADED_FILES: &str = "uploaded_files";

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

async fn upload_file_worker(mut payload: Multipart, upload_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut pathbufs = Vec::new();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let filename = content_disposition.get_filename().context("No filename")?;
        log::info!("Saving file: {}", &filename);
        let filepath = upload_dir.join(filename);

        let mut f = fs::File::create(&filepath)?;
        while let Some(Ok(bytes)) = field.next().await {
            f.write_all(&bytes)?;
        }
        pathbufs.push(filepath)
    }

    Ok(pathbufs)
}

async fn send_file(
    bot: web::Data<Bot>,
    chat_data: web::Query<ChatData<Option<String>>>,
    payload: Multipart,
    upload_dir: web::Data<PathBuf>,
) -> impl Responder {
    log::info!(
        "Sending file -> {}: {:?}",
        chat_data.chatid,
        chat_data.message
    );

    // Process the uploaded files
    let paths = match upload_file_worker(payload, &upload_dir).await {
        Ok(paths) => paths,
        Err(e) => {
            log::info!("Error uploading files: {:?}", e);
            return HttpResponse::InternalServerError().body(e.to_string());
        }
    };

    // Send a message if provided
    if let Some(message) = &chat_data.message {
        let res = bot
            .send_message(chat_data.chatid.clone(), message)
            .parse_mode(ParseMode::Html)
            .await;

        if let Err(e) = res {
            log::info!("Error sending message: {:?}", e);
            return HttpResponse::InternalServerError().body(e.to_string());
        }
    }

    // Send each uploaded file as a document
    let mut errors = Vec::new();
    for path in paths {
        let res = bot
            .send_document(chat_data.chatid.clone(), InputFile::file(&path))
            .await;
        if let Err(e) = res {
            errors.push(e);
        }
    }

    if errors.is_empty() {
        HttpResponse::Accepted().finish()
    } else {
        let errors: Vec<_> = errors.iter().map(ToString::to_string).collect();
        HttpResponse::InternalServerError()
            .body(format!("Got the following errors: {}", errors.join(", ")))
    }
}

async fn send_message<T: Deref<Target = ChatData<String>>>(
    bot: web::Data<Bot>,
    data: T,
) -> impl Responder {
    log::info!("Sending message -> {}: {}", data.chatid, data.message);
    let res = bot
        .send_message(data.chatid.clone(), &data.message)
        .parse_mode(ParseMode::Html)
        .send()
        .await;

    match res {
        Ok(_) => HttpResponse::Ok().body("Message sent!"),
        Err(e) => {
            log::info!("Error sending message: {:?}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to send message to Telegram: {:?}", e))
        }
    }
}

#[derive(Debug, macros::BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Help")]
    Help,
    #[command(description = "Get chat id")]
    GetId,
    #[command(description = "Pong!")]
    Ping,
    #[command(description = "Roll a dice")]
    Dice,
}

impl Command {
    async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .send()
                    .await?
            }
            Command::GetId => {
                bot.send_message(
                    msg.chat.id,
                    format!("The chat ID is: <code>{}</code>", msg.chat.id),
                )
                .parse_mode(ParseMode::Html)
                .send()
                .await?
            }
            Command::Ping => bot.send_message(msg.chat.id, "pong!").send().await?,
            Command::Dice => bot.send_dice(msg.chat.id).send().await?,
        };
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatData<T> {
    chatid: String,
    message: T,
}
