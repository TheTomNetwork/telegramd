use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use anyhow::{Context, Result};
use futures::StreamExt;
use futures::TryStreamExt;
use std::fs;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use teloxide::{
    prelude::*,
    types::{InputFile, ParseMode},
};

use crate::chat_data::ChatData;

pub async fn upload_file_worker(mut payload: Multipart, upload_dir: &Path) -> Result<Vec<PathBuf>> {
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

pub async fn send_file(
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
