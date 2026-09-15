use std::error::Error;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        id::GuildId,
        prelude::{application_command::ApplicationCommandInteraction, command::CommandOptionType},
    },
};

use crate::{
    command::{get_new_config_contents_blacklist, Command, CommandContext},
    config::PACEMANBOT_BLACKLIST_CHANNEL,
};

pub struct Blacklist;

#[async_trait]
impl Command for Blacklist {
    fn name(&self) -> &str {
        "blacklist"
    }

    fn description(&self) -> &str {
        "Blacklist new players or remove old players' configurations in the server based on uuid."
    }

    fn create_options<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command
            .create_option(|option| {
                option
                    .name("action")
                    .description("Action to perform out of 'add' or 'remove'.")
                    .required(true)
                    .kind(CommandOptionType::String)
                    .add_string_choice("Add", "add")
                    .add_string_choice("Remove", "remove")
            })
            .create_option(|option| {
                option
                    .name("uuid")
                    .description("UUID (with hyphens '-') of the runner that you want to add.")
                    .required(true)
                    .kind(CommandOptionType::String)
            })
    }

    async fn execute(&self, context: CommandContext<'_>) -> Result<(), Box<dyn Error>> {
        let response_content =
            update_blacklist(context.ctx, context.guild_id, context.interaction, true).await?;
        context
            .interaction
            .edit_original_interaction_response(&context.ctx.http, |data| {
                data.content(response_content)
            })
            .await?;
        Ok(())
    }
}

pub(super) async fn update_blacklist(
    ctx: &Context,
    guild_id: GuildId,
    command: &ApplicationCommandInteraction,
    use_uuid: bool,
) -> Result<String, Box<dyn Error>> {
    let channels = match ctx.cache.guild_channels(guild_id) {
        Some(channels) => channels,
        None => {
            return Err(format!("failed to get channels for guild id: {}", guild_id).into());
        }
    };
    let mut action = String::new();
    let mut ign = String::new();
    let mut uuid = String::new();

    for option in command.data.options.iter() {
        match option.name.as_str() {
            "action" => {
                action = match option.value.to_owned() {
                    Some(value) => match value.as_str() {
                        Some(str) => str.to_owned(),
                        None => {
                            return Err(
                                String::from("failed to parse string for action option.").into()
                            )
                        }
                    },
                    None => {
                        return Err(String::from("failed to get value for action option.").into())
                    }
                }
            }
            "ign" => match option.value.to_owned() {
                Some(value) => {
                    ign = match value.as_str() {
                        Some(str) => str.to_owned(),
                        None => {
                            return Err(
                                String::from("failed to parse string for ign option.").into()
                            )
                        }
                    }
                }
                None => return Err(String::from("failed to get value for ign option.").into()),
            },
            "uuid" => match option.value.to_owned() {
                Some(value) => {
                    uuid = match value.as_str() {
                        Some(str) => str.to_owned(),
                        None => {
                            return Err(
                                String::from("failed to parse string for uuid option.").into()
                            )
                        }
                    }
                }
                None => return Err(String::from("failed to get value for uuid option.").into()),
            },
            _ => return Err(format!("unrecognized command option: '{}'", option.name).into()),
        };
    }

    let channel = channels
        .iter()
        .filter(|c| c.name == PACEMANBOT_BLACKLIST_CHANNEL)
        .collect::<Vec<_>>();
    let channel = match channel.first() {
        Some(channel) => channel,
        None => {
            return Err(format!(
                "failed to find #{} in guild id: {}",
                PACEMANBOT_BLACKLIST_CHANNEL, guild_id
            )
            .into())
        }
    };
    let message = channel.messages(&ctx.http, |m| m.limit(1)).await?;
    let mut players: Vec<String> = vec![];
    match message.last() {
        Some(message) => {
            if !message.author.bot {
                return Err(format!(
                    "failed as the first message in #{} is not from the bot.",
                    PACEMANBOT_BLACKLIST_CHANNEL
                )
                .into());
            }
            for line in message.content.split("\n") {
                if line == "```" || line == "" {
                    continue;
                }
                players.push(line.to_string());
            }
            if action == "remove" {
                if use_uuid {
                    players.iter().position(|p| p == &uuid);
                } else {
                    players.iter().position(|p| p == &ign);
                }
            } else {
                if use_uuid {
                    players.push(uuid);
                } else {
                    players.push(ign);
                }
            }
            let new_config = get_new_config_contents_blacklist(&players);
            message
                .to_owned()
                .edit(&ctx.http, |m| {
                    m.content(format!("```\n{}\n```", new_config))
                })
                .await?;
        }
        None => {
            if action == "remove" {
                return Err(
                    format!("failed to remove names from in guild id: {}", guild_id).into(),
                );
            }
            if use_uuid {
                players.push(uuid);
            } else {
                players.push(ign);
            }
            let new_config = get_new_config_contents_blacklist(&players);
            channel
                .send_message(&ctx.http, |m| {
                    m.content(format!("```\n{}\n```", new_config))
                })
                .await?;
        }
    };
    Ok("Updated config!".to_string())
}

pub const BLACKLIST: Blacklist = Blacklist {};
