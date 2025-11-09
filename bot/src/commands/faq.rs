use anyhow::Context;
use async_trait::async_trait;
use builders::command_option::CommandOptionBuilder;
use twilight_model::application::command::{Command, CommandOptionType, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{
    Interaction, InteractionContextType, InteractionData,
};
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::CommandBuilder;

use crate::commands::CommandHandler;

pub(super) const FAQ_CMD_NAME: &str = "faq";

const QUERY_OPTION_NAME: &str = "query";
const MENTION_OPTION_NAME: &str = "mention";

#[allow(dead_code)]
pub(crate) struct Faq<'a> {
    pub(crate) cmd: &'a Interaction,
}

#[async_trait]
impl CommandHandler for Faq<'_> {
    fn model(ctx: Option<crate::Context>) -> anyhow::Result<Command> {
        let ctx = ctx.expect("ctx is required");
        let query_option = CommandOptionBuilder::new(
            QUERY_OPTION_NAME,
            "The response to send.",
            CommandOptionType::String,
        )
        .choices(ctx.cfg.faq_option_choices())
        .required(true)
        .build()?;

        let mention_option = CommandOptionBuilder::new(
            MENTION_OPTION_NAME,
            "The user to mention in the response.",
            CommandOptionType::User,
        )
        .build()?;

        Ok(CommandBuilder::new(
            "faq",
            "Send quick responses to common questions/queries.",
            CommandType::ChatInput,
        )
        .contexts([InteractionContextType::Guild])
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .default_member_permissions(Permissions::MANAGE_CHANNELS)
        .option(query_option)
        .option(mention_option)
        .validate()
        .context("validate faq command")?
        .build())
    }

    async fn exec(&self, ctx: crate::Context) -> anyhow::Result<()> {
        let Some(InteractionData::ApplicationCommand(data)) = &self.cmd.data else {
            anyhow::bail!("expected application command interaction");
        };
        // Get the query option from the command data
        let query = data
            .options
            .iter()
            .find(|opt| opt.name == QUERY_OPTION_NAME)
            .context("missing query option")?;
        let CommandOptionValue::String(query) = &query.value else {
            anyhow::bail!("expected string query option");
        };
        let Some(mut response) = ctx.cfg.faq_option_response(query) else {
            anyhow::bail!("unknown query option: {}", query);
        };

        // Add mention if provided
        let mention = data
            .options
            .iter()
            .find(|opt| opt.name == MENTION_OPTION_NAME);

        if let Some(mention) = mention {
            let CommandOptionValue::User(u_id) = mention.value else {
                anyhow::bail!("expected user option");
            };
            // Either set the content to the response or prepend it to the response
            response.content = response
                .content
                .map_or(Some(format!("<@{u_id}>")), |content| {
                    Some(format!("<@{u_id}> {content}"))
                });
        }

        // Send the response
        ctx.http
            .interaction(self.cmd.application_id)
            .create_response(self.cmd.id, &self.cmd.token, &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(response),
            })
            .await?;

        Ok(())
    }
}
