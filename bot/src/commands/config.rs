use std::fmt::Display;
use std::str::FromStr;

use anyhow::Context;
use async_trait::async_trait;
use builders::command_option::CommandOptionBuilder;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{
    Interaction, InteractionContextType, InteractionData,
};
use twilight_model::guild::Permissions;
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::CommandHandler;
use crate::config::config_path;

pub(super) const CONFIG_CMD_NAME: &str = "config";

#[allow(dead_code)]
pub(crate) struct Config<'a> {
    pub(crate) cmd: &'a Interaction,
}

#[async_trait]
impl CommandHandler for Config<'_> {
    fn model(_ctx: Option<crate::Context>) -> anyhow::Result<Command> {
        let file_type_choices = [
            CommandOptionChoice {
                name: "Rust".to_string(),
                value: CommandOptionChoiceValue::String(FileType::Rust.to_string()),
                name_localizations: None,
            },
            CommandOptionChoice {
                name: "YAML".to_string(),
                value: CommandOptionChoiceValue::String(FileType::Yaml.to_string()),
                name_localizations: None,
            },
        ];
        let file_type_option = CommandOptionBuilder::new(
            "file_type",
            "The type of file to send",
            CommandOptionType::String,
        )
        .required(true)
        .choices(file_type_choices)
        .build()?;

        Ok(CommandBuilder::new(
            "config",
            "Send quick responses to common questions/queries.",
            CommandType::ChatInput,
        )
        .contexts([InteractionContextType::Guild])
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .default_member_permissions(Permissions::MANAGE_CHANNELS)
        .option(file_type_option)
        .validate()
        .context("validate config command")?
        .build())
    }

    async fn exec(&self, ctx: crate::Context) -> anyhow::Result<()> {
        // Parse the given file type from command options
        let Some(InteractionData::ApplicationCommand(data)) = &self.cmd.data else {
            anyhow::bail!("expected application command interaction");
        };
        let CommandOptionValue::String(file_type) = &data.options[0].value else {
            anyhow::bail!("expected string option value");
        };
        let file_type =
            FileType::from_str(file_type).context("failed to parse file type from string")?;

        // Create the config file based on the file type
        let cfg_file = match file_type {
            FileType::Rust => {
                // Get the stored config struct
                let content = format!("{:#?}", ctx.cfg).into_bytes();
                Attachment::from_bytes("magnolia.cfg.rs".to_string(), content, 0)
            },
            FileType::Yaml => {
                // Get the raw content of the config file
                let content = std::fs::read(config_path())?;
                Attachment::from_bytes("magnolia.cfg.yml".to_string(), content, 0)
            },
        };

        // Respond to the interaction with the config file
        ctx.http
            .interaction(self.cmd.application_id)
            .create_response(self.cmd.id, &self.cmd.token, &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    InteractionResponseDataBuilder::new()
                        .attachments([cfg_file])
                        .build(),
                ),
            })
            .await
            .context("failed to send config file")?;

        Ok(())
    }
}

/// FileType enum to represent the type of config file to send
enum FileType {
    Rust,
    Yaml,
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Rust => write!(f, "Rust"),
            FileType::Yaml => write!(f, "YAML"),
        }
    }
}

impl FromStr for FileType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Rust" => Ok(FileType::Rust),
            "YAML" => Ok(FileType::Yaml),
            _ => Err(anyhow::anyhow!("Invalid file type")),
        }
    }
}
