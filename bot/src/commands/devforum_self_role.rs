use anyhow::Context;
use async_trait::async_trait;
use builders::component::ActionRowBuilder;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::{Interaction, InteractionContextType};
use twilight_model::channel::message::MessageFlags;
use twilight_model::guild::Permissions;
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::CommandHandler;
use crate::components::verify_devforum_rank::VerifyDevForumRank;
use crate::components::ComponentHandler;

#[allow(dead_code)]
pub(crate) struct DevForumSelfRole<'a> {
    pub(crate) cmd: &'a Interaction,
}

#[async_trait]
impl CommandHandler for DevForumSelfRole<'_> {
    fn model(_ctx: Option<crate::Context>) -> anyhow::Result<Command> {
        Ok(CommandBuilder::new(
            "devforum-self-role",
            "Send an info embed with a button to self-update DevForum roles.",
            CommandType::ChatInput,
        )
        .contexts([InteractionContextType::Guild])
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .default_member_permissions(Permissions::MANAGE_CHANNELS)
        .validate()
        .context("validate devforum-self-role command")?
        .build())
    }

    async fn exec(&self, ctx: crate::Context) -> anyhow::Result<()> {
        let devforum_logo = Attachment::from_bytes(
            "devforum-logo.png".to_string(),
            Vec::from(include_bytes!("../../../assets/devforum-logo.png")),
            0,
        );
        let info_embed = EmbedBuilder::new()
            .title("Developer Forum Member Role(s)")
            .description(format!(
                "Click the `Update Roles` button below to claim your developer forum member role if you meet the eligibility criteria.

- <@&{}> - Your **trust level** on the Roblox developer forum is `Member` (not to be confused with `Visitor`)
- What is the developer forum? [**Learn more**](https://help.roblox.com/hc/articles/360000240223)
- How do I \"level up\"? [**Learn more**](https://devforum.roblox.com/t/3170997)",
                ctx.cfg.roles.devforum_member,
            ))
            .thumbnail(ImageSource::attachment(&devforum_logo.filename)?)
            .build();
        let action_row = ActionRowBuilder::new()
            .set_components([VerifyDevForumRank::model()?])
            .build()
            .context("build action row")?;

        // Send the embed to the channel
        let channel_id = self.cmd.channel.as_ref().context("get channel id")?.id;
        ctx.http
            .create_message(channel_id)
            .embeds(&[info_embed])
            .components(&[action_row])
            .attachments(&[devforum_logo])
            .await?;

        // Create the interaction response
        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .content("Successfully sent the self-role embed.")
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            ),
        };

        // Respond to the interaction ephemerally
        ctx.http
            .interaction(self.cmd.application_id)
            .create_response(self.cmd.id, &self.cmd.token, &response)
            .await
            .context("create response")?;

        Ok(())
    }
}
