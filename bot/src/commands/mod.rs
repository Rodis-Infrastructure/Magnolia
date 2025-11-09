use async_trait::async_trait;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::Interaction;

mod config;
mod devforum_self_role;
mod faq;
mod highlight;

/// Get all application command models.
pub(crate) fn models(ctx: crate::Context) -> anyhow::Result<Vec<Command>> {
    Ok(vec![
        devforum_self_role::DevForumSelfRole::model(None)?,
        config::Config::model(None)?,
        faq::Faq::model(Some(ctx))?,
        highlight::Highlight::model(None)?,
    ])
}

/// Trait for implementing application commands.
#[async_trait]
pub(crate) trait CommandHandler: Send {
    fn model(ctx: Option<crate::Context>) -> anyhow::Result<Command>
    where
        Self: Sized;
    async fn exec(&self, ctx: crate::Context) -> anyhow::Result<()>;
}

pub(crate) async fn handle_command(
    cmd: &Interaction,
    cmd_name: &str,
    ctx: crate::Context,
) -> anyhow::Result<()> {
    let handler: Box<dyn CommandHandler> = match cmd_name {
        devforum_self_role::DEVFORUM_SELF_ROLE_CMD_NAME => {
            Box::new(devforum_self_role::DevForumSelfRole { cmd })
        },
        config::CONFIG_CMD_NAME => Box::new(config::Config { cmd }),
        faq::FAQ_CMD_NAME => Box::new(faq::Faq { cmd }),
        highlight::HIGHLIGHT_CMD_NAME => Box::new(highlight::Highlight { cmd }),
        unknown => anyhow::bail!("unknown command name: {}", unknown),
    };
    handler.exec(ctx).await
}
