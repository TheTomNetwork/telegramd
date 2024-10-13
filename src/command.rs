use teloxide::{
    macros,
    payloads::SendMessageSetters,
    prelude::{Request, Requester, ResponseResult},
    types::{Message, ParseMode},
    utils::command::BotCommands,
    Bot,
};

#[derive(Debug, macros::BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
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
    pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
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
