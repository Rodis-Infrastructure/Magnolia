use anyhow::Context;
use serde::Deserialize;
use twilight_model::application::command::{CommandOptionChoice, CommandOptionChoiceValue};
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::id::marker::RoleMarker;
use twilight_model::id::Id;

/// Configuration for the bot.
#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    /// A mapping of role IDs to their names.
    pub(crate) roles: RoleConfig,
    /// A list of options for the FAQ command.
    faq_options: Vec<FaqOption>,
}

/// Configuration for roles.
#[derive(Deserialize, Debug)]
pub(crate) struct RoleConfig {
    pub(crate) devforum_member: Id<RoleMarker>,
    pub(crate) roblox_verified: Option<Id<RoleMarker>>,
}

/// Configuration for an option of the FAQ command.
#[derive(Deserialize, Debug)]
pub(crate) struct FaqOption {
    /// The label of the option (displayed to the user).
    label: String,
    /// The value of the option (used as the identifier).
    value: String,
    /// The embed to be sent when this option is selected.
    // pub(crate) embed: Embed,
    /// The components to be included with the embed.
    // pub(crate) components: Option<Vec<Component>>,
    pub(crate) response: InteractionResponseData,
}

impl Config {
    /// Returns a vector of options for the FAQ command.
    pub(crate) fn faq_option_choices(&self) -> Vec<CommandOptionChoice> {
        self.faq_options
            .iter()
            .map(|opt| CommandOptionChoice {
                name: opt.label.clone(),
                value: CommandOptionChoiceValue::String(opt.value.clone()),
                name_localizations: None,
            })
            .collect()
    }

    /// Returns the FAQ option corresponding to the given value.
    pub(crate) fn faq_option_response<S>(&self, value: S) -> Option<InteractionResponseData>
    where
        S: AsRef<str>,
    {
        self.faq_options
            .iter()
            .find(|opt| opt.value == value.as_ref())
            .map(|opt| opt.response.clone())
    }
}

/// Loads the configuration from a YAML file.
pub(crate) fn load_config(path: String) -> Result<Config, anyhow::Error> {
    let cfg_yaml = std::fs::read(path).context("read config file")?;
    serde_yaml::from_slice(&cfg_yaml).context("parse config file")
}

/// Parses the config file path from command line arguments
/// or defaults to "magnolia.cfg.yml".
pub(crate) fn config_path() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| "magnolia.cfg.yml".to_string())
}
