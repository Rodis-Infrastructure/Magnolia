use std::fmt::Display;

use anyhow::Context;
use async_trait::async_trait;
use builders::command_option::CommandOptionBuilder;
use rusqlite::types::FromSqlError;
use rusqlite::Error as SqlError;
use twilight_model::application::command::{
    Command, CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType, CommandType,
};
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::application::interaction::{
    Interaction, InteractionContextType, InteractionData,
};
use twilight_model::channel::message::embed::{EmbedField, EmbedFooter};
use twilight_model::channel::message::{Embed, MessageFlags};
use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;
use twilight_model::oauth::ApplicationIntegrationType;
use twilight_model::user::User;
use twilight_util::builder::command::CommandBuilder;
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::commands::CommandHandler;

// Command name
pub(super) const HIGHLIGHT_CMD_NAME: &str = "highlight";

// Subcommand groups
const CHANNEL_SUB_GROUP: &str = "channel";
const PATTERN_SUB_GROUP: &str = "pattern";

// Subcommands of groups
const ADD_SUB_CMD: &str = "add";
const REMOVE_SUB_CMD: &str = "remove";
const CLEAR_SUB_CMD: &str = "clear";

// Subcommands
const LIST_SUB_CMD: &str = "list";
const ERASE_SUB_CMD: &str = "erase";

// Options
const CHANNEL_OPTION: &str = "channel";
const PATTERN_OPTION: &str = "pattern";
const SCOPE_OPTION: &str = "scope";
const USER_OPTION: &str = "user";

// Constraints
const PATTERN_MAX_LENGTH: usize = 100;
const PATTERN_LIMIT: usize = 20;
const CHANNEL_LIMIT: usize = 40;

#[allow(dead_code)]
pub(crate) struct Highlight<'a> {
    pub(crate) cmd: &'a Interaction,
}

#[async_trait]
impl CommandHandler for Highlight<'_> {
    fn model(_ctx: Option<crate::Context>) -> anyhow::Result<Command> {
        let channel_sub_group = CommandOptionBuilder::new(
            CHANNEL_SUB_GROUP,
            "Manage channel scoping.",
            CommandOptionType::SubCommandGroup,
        )
        .options([
            CommandOptionBuilder::new(
                ADD_SUB_CMD,
                "Add a channel to the scope.",
                CommandOptionType::SubCommand,
            )
            .options([
                CommandOptionBuilder::new(
                    CHANNEL_OPTION,
                    "The channel to add.",
                    CommandOptionType::Channel,
                )
                .channel_types([ChannelType::GuildText])
                .required(true)
                .build()?,
                CommandOptionBuilder::new(
                    SCOPE_OPTION,
                    "The scope of the channel.",
                    CommandOptionType::String,
                )
                .choices([Scope::Whitelist.as_choice(), Scope::Blacklist.as_choice()])
                .required(true)
                .build()?,
            ])
            .build()?,
            CommandOptionBuilder::new(
                REMOVE_SUB_CMD,
                "Remove a channel from the scope.",
                CommandOptionType::SubCommand,
            )
            .options([
                CommandOptionBuilder::new(
                    CHANNEL_OPTION,
                    "The channel to remove.",
                    CommandOptionType::Channel,
                )
                .channel_types([ChannelType::GuildText])
                .required(true)
                .build()?,
                CommandOptionBuilder::new(
                    SCOPE_OPTION,
                    "The scope of the channel.",
                    CommandOptionType::String,
                )
                .choices([Scope::Whitelist.as_choice(), Scope::Blacklist.as_choice()])
                .required(true)
                .build()?,
            ])
            .build()?,
            CommandOptionBuilder::new(
                CLEAR_SUB_CMD,
                "Clear all channels from the scope.",
                CommandOptionType::SubCommand,
            )
            .build()?,
        ])
        .build()?;

        let pattern_sub_group = CommandOptionBuilder::new(
            PATTERN_SUB_GROUP,
            "Manage pattern scoping.",
            CommandOptionType::SubCommandGroup,
        )
        .options([
            CommandOptionBuilder::new(
                ADD_SUB_CMD,
                "Add a pattern to the scope.",
                CommandOptionType::SubCommand,
            )
            .option(
                CommandOptionBuilder::new(
                    PATTERN_OPTION,
                    "The pattern to add.",
                    CommandOptionType::String,
                )
                .required(true)
                .build()?,
            )
            .build()?,
            CommandOptionBuilder::new(
                REMOVE_SUB_CMD,
                "Remove a pattern from the scope.",
                CommandOptionType::SubCommand,
            )
            .option(
                CommandOptionBuilder::new(
                    PATTERN_OPTION,
                    "The pattern to remove.",
                    CommandOptionType::String,
                )
                .required(true)
                .build()?,
            )
            .build()?,
            CommandOptionBuilder::new(
                CLEAR_SUB_CMD,
                "Clear all patterns from the scope.",
                CommandOptionType::SubCommand,
            )
            .build()?,
        ])
        .build()?;

        let list_sub_cmd = CommandOptionBuilder::new(
            LIST_SUB_CMD,
            "List all patterns.",
            CommandOptionType::SubCommand,
        )
        .option(
            CommandOptionBuilder::new(
                USER_OPTION,
                "The user to view the highlights of.",
                CommandOptionType::User,
            )
            .build()?,
        )
        .build()?;

        let erase_sub_cmd = CommandOptionBuilder::new(
            ERASE_SUB_CMD,
            "Erase a pattern.",
            CommandOptionType::SubCommand,
        )
        .option(
            CommandOptionBuilder::new(
                USER_OPTION,
                "The user to erase the highlights of.",
                CommandOptionType::User,
            )
            .build()?,
        )
        .build()?;

        Ok(CommandBuilder::new(
            HIGHLIGHT_CMD_NAME,
            "Manage highlights.",
            CommandType::ChatInput,
        )
        .contexts([InteractionContextType::Guild])
        .integration_types([ApplicationIntegrationType::GuildInstall])
        .default_member_permissions(Permissions::MANAGE_CHANNELS)
        .option(channel_sub_group)
        .option(pattern_sub_group)
        .option(list_sub_cmd)
        .option(erase_sub_cmd)
        .validate()
        .context("validate highlight command")?
        .build())
    }

    async fn exec(&self, ctx: crate::Context) -> anyhow::Result<()> {
        let Some(InteractionData::ApplicationCommand(data)) = &self.cmd.data else {
            anyhow::bail!("expected application command interaction");
        };
        let sub_cmd = data
            .options
            .get(0)
            .expect("command should have at least one option");

        // Filter action by subcommand
        let response = match sub_cmd.name.as_str() {
            CHANNEL_SUB_GROUP => manage_highlight_channels(&ctx, data),
            PATTERN_SUB_GROUP => manage_highlight_patterns(&ctx, data),
            LIST_SUB_CMD => list_highlights(&ctx, self.cmd, data),
            ERASE_SUB_CMD => erase_highlights(&ctx, data),
            _ => anyhow::bail!("unknown subcommand: {}", sub_cmd.name),
        };
        let response = response.unwrap_or_else(|e| {
            tracing::error!("error executing highlight command: {:?}", e);

            InteractionResponseDataBuilder::new()
                .content("An error occurred while processing your request.")
                .flags(MessageFlags::EPHEMERAL)
                .build()
        });

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

fn list_highlights(
    ctx: &crate::Context,
    cmd: &Interaction,
    _cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    let author = cmd.author().unwrap();
    let mut conn = ctx.conn.lock().unwrap();
    let tx = conn.transaction()?;

    // Get all the author's highlight patterns and channel scope configurations for this guild
    let mut channel_stmt =
        tx.prepare("SELECT * FROM highlight_channel_scoping WHERE user_id = ? AND guild_id = ?")?;
    let mut pattern_stmt =
        tx.prepare("SELECT * FROM highlight_pattern WHERE user_id = ? AND guild_id = ?")?;

    let hl_channels = channel_stmt
        .query_map((author.id.get(), cmd.guild_id.unwrap().get()), |row| {
            let channel_id: String = row.get(0)?;
            let channel_id: Id<ChannelMarker> = channel_id
                .parse()
                .map_err(|_| SqlError::from(FromSqlError::InvalidType))?;

            let scope: String = row.get(1)?;
            let scope = Scope::try_from(scope.as_str())
                .map_err(|_| SqlError::from(FromSqlError::InvalidType))?;

            Ok(HighlightChannel { channel_id, scope })
        })
        .context("fetch channel scopes from database")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let hl_patterns = pattern_stmt
        .query_map((author.id.get(), cmd.guild_id.unwrap().get()), |row| {
            let pattern: String = row.get(0)?;
            let pattern_type: String = row.get(1)?;
            let pattern_type = PatternType::try_from(pattern_type.as_str())
                .map_err(|_| SqlError::from(FromSqlError::InvalidType))?;

            Ok(HighlightPattern {
                pattern,
                pattern_type,
            })
        })
        .context("fetch patterns from database")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    // tx.commit()?;

    Ok(InteractionResponseDataBuilder::new()
        .embeds([build_highlight_list_embed(
            author,
            &hl_patterns,
            &hl_channels,
        )])
        .build())
}

// Build an embed listing the user's highlight patterns and channel scopes
fn build_highlight_list_embed(
    author: &User,
    patterns: &[HighlightPattern],
    channels: &[HighlightChannel],
) -> Embed {
    let whitelist_channels: Vec<&HighlightChannel> = channels
        .iter()
        .filter(|ch| matches!(ch.scope, Scope::Whitelist))
        .collect();
    let blacklist_channels: Vec<&HighlightChannel> = channels
        .iter()
        .filter(|ch| matches!(ch.scope, Scope::Blacklist))
        .collect();

    let pattern_field_value = if patterns.is_empty() {
        "No patterns set.".to_string()
    } else {
        patterns
            .iter()
            .map(|p| format!("`{}` ({})", p.pattern, p.pattern_type))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let whitelist_field_value = if whitelist_channels.is_empty() {
        "No whitelisted channels.".to_string()
    } else {
        whitelist_channels
            .iter()
            .map(|ch| format!("<#{}>", ch.channel_id))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let blacklist_field_value = if blacklist_channels.is_empty() {
        "No blacklisted channels.".to_string()
    } else {
        blacklist_channels
            .iter()
            .map(|ch| format!("<#{}>", ch.channel_id))
            .collect::<Vec<String>>()
            .join("\n")
    };

    EmbedBuilder::new()
        .field(EmbedField {
            name: format!("Patterns ({}/{})", patterns.len(), PATTERN_LIMIT),
            value: pattern_field_value,
            inline: false,
        })
        .field(EmbedField {
            name: format!(
                "Whitelisted Channels ({}/{})",
                whitelist_channels.len(),
                CHANNEL_LIMIT
            ),
            value: whitelist_field_value,
            inline: true,
        })
        .field(EmbedField {
            name: format!(
                "Blacklisted Channels ({}/{})",
                blacklist_channels.len(),
                CHANNEL_LIMIT
            ),
            value: blacklist_field_value,
            inline: true,
        })
        .footer(EmbedFooter {
            text: format!("@{}  •  {}", author.name, author.id),
            icon_url: None,
            proxy_icon_url: None,
        })
        .build()
}

fn erase_highlights(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn manage_highlight_channels(
    ctx: &crate::Context,
    cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    let sub_cmd = cmd_data
        .options
        .get(1)
        .expect("subcommand group must have at least one subcommand");

    // Handle the clear subcommand
    // This is a special case because it doesn't require a channel id or scope
    // All other options must have a channel id and a scope
    if sub_cmd.name.as_str() == CLEAR_SUB_CMD {
        return clear_highlight_channels(ctx, cmd_data);
    }

    // Parse the channel id
    let CommandOptionValue::Channel(channel_id) =
        cmd_data.options.get(2).expect("channel option").value
    else {
        anyhow::bail!("expected channel id");
    };

    // Parse the scope
    let CommandOptionValue::String(scope) = &cmd_data.options.get(3).expect("scope option").value
    else {
        anyhow::bail!("expected scope to be a string");
    };
    let scope = Scope::try_from(scope.as_str()).expect("valid scope");

    // Handle the add and remove subcommands
    match sub_cmd.name.as_str() {
        ADD_SUB_CMD => add_highlight_channel(ctx, cmd_data, channel_id, scope),
        REMOVE_SUB_CMD => remove_highlight_channel(ctx, cmd_data, channel_id, scope),
        _ => anyhow::bail!("unknown subcommand: {}", sub_cmd.name),
    }
}

fn add_highlight_channel(
    ctx: &crate::Context,
    cmd_data: &CommandData,
    channel_id: Id<ChannelMarker>,
    scope: Scope,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn remove_highlight_channel(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
    _channel_id: Id<ChannelMarker>,
    _scope: Scope,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn clear_highlight_channels(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn manage_highlight_patterns(
    ctx: &crate::Context,
    cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    let sub_cmd = cmd_data
        .options
        .get(1)
        .expect("subcommand group must have at least one subcommand");

    // Handle the clear subcommand
    // This is a special case because it doesn't require a pattern
    // All other options must have a pattern
    if sub_cmd.name.as_str() == CLEAR_SUB_CMD {
        return clear_highlight_patterns(ctx, cmd_data);
    }

    // Parse the pattern
    let CommandOptionValue::String(pattern) =
        &cmd_data.options.get(2).expect("pattern option").value
    else {
        anyhow::bail!("expected pattern to be a string");
    };

    // Handle the add and remove subcommands
    match sub_cmd.name.as_str() {
        ADD_SUB_CMD => add_highlight_pattern(ctx, cmd_data, pattern.to_string()),
        REMOVE_SUB_CMD => remove_highlight_pattern(ctx, cmd_data, pattern.to_string()),
        _ => anyhow::bail!("unknown subcommand: {}", sub_cmd.name),
    }
}

fn add_highlight_pattern(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
    _pattern: String,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn remove_highlight_pattern(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
    _pattern: String,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

fn clear_highlight_patterns(
    _ctx: &crate::Context,
    _cmd_data: &CommandData,
) -> anyhow::Result<InteractionResponseData> {
    unimplemented!()
}

struct HighlightPattern {
    pattern: String,
    pattern_type: PatternType,
}

struct HighlightChannel {
    channel_id: Id<ChannelMarker>,
    scope: Scope,
}

struct UserHighlights {
    patterns: Vec<HighlightPattern>,
    channels: Vec<HighlightChannel>,
}

/// Enum representing the scope of a highlight channel.
enum Scope {
    Whitelist,
    Blacklist,
}

/// Enum representing the type of pattern used for highlights.
enum PatternType {
    Exact,
    Wildcard,
    Regex,
}

impl TryFrom<&str> for Scope {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "whitelist" => Ok(Scope::Whitelist),
            "blacklist" => Ok(Scope::Blacklist),
            _ => anyhow::bail!("unknown scope: {}", value),
        }
    }
}

impl TryFrom<&str> for PatternType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "exact" => Ok(PatternType::Exact),
            "wildcard" => Ok(PatternType::Wildcard),
            "regex" => Ok(PatternType::Regex),
            _ => anyhow::bail!("unknown pattern type: {}", value),
        }
    }
}

impl Scope {
    /// Get the scope as a [`CommandOptionChoice`] (for the command option model).
    fn as_choice(&self) -> CommandOptionChoice {
        match self {
            Scope::Whitelist => CommandOptionChoice {
                name: "Whitelist".to_string(),
                name_localizations: None,
                value: CommandOptionChoiceValue::String("whitelist".to_string()),
            },
            Scope::Blacklist => CommandOptionChoice {
                name: "Blacklist".to_string(),
                name_localizations: None,
                value: CommandOptionChoiceValue::String("blacklist".to_string()),
            },
        }
    }
}

impl Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Whitelist => write!(f, "whitelist"),
            Scope::Blacklist => write!(f, "blacklist"),
        }
    }
}

impl Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::Exact => write!(f, "exact"),
            PatternType::Wildcard => write!(f, "wildcard"),
            PatternType::Regex => write!(f, "regex"),
        }
    }
}
