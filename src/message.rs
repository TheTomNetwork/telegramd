use actix_web::{web, HttpResponse, Responder};
use std::ops::Deref;
use teloxide::{prelude::*, types::ParseMode};

use crate::chat_data::ChatData;

pub async fn send_message<T: Deref<Target = ChatData<String>>>(
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
