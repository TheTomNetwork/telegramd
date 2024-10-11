use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use dotenv::dotenv;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{
    fs,
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

const UPLOADED_FILES: &'static str = "uploaded_files";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;
    let _ = fs::create_dir(UPLOADED_FILES);
    println!("Started telegramd");

    let command_listener = Command::repl(Bot::from_env(), Command::answer);

    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Bot::from_env()))
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
    .bind("127.0.0.1:5005")?
    .run();

    join!(command_listener, http_server).1?;
    Ok(())
}

async fn upload_file_worker(mut payload: Multipart) -> Result<Vec<PathBuf>> {
    let mut pathbufs = Vec::new();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let filename = content_disposition.get_filename().context("No filename")?;
        println!("filename: {}", &filename);
        let filepath = Path::new(&format!("./{}/{}", UPLOADED_FILES, filename)).to_path_buf();

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
) -> impl Responder {
    let paths = match upload_file_worker(payload).await {
        Ok(paths) => paths,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    if let Some(message) = &chat_data.message {
        let res = bot
            .send_message(chat_data.chatid.clone(), message)
            .parse_mode(ParseMode::Html)
            .await;

        if let Err(e) = res {
            return HttpResponse::InternalServerError().body(e.to_string());
        }
    }

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
        return HttpResponse::Accepted().finish();
    }

    let errors: Vec<_> = errors.iter().map(ToString::to_string).collect();
    HttpResponse::InternalServerError()
        .body(format!("Got the following errors: {}", errors.join(", ")))
}

async fn send_message<T: Deref<Target = ChatData<String>>>(
    bot: web::Data<Bot>,
    data: T,
) -> impl Responder {
    let res = bot
        .send_message(data.chatid.clone(), &data.message)
        .parse_mode(ParseMode::Html)
        .send()
        .await;

    match res {
        Ok(_) => HttpResponse::Ok().body("Message sent!"),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Failed to send message to telegram: {:?}", e)),
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
                    format!("The chatid is: <code>{}</code>", msg.chat.id),
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

#[derive(Serialize, Deserialize)]
struct ChatData<T> {
    chatid: String,
    message: T,
}
