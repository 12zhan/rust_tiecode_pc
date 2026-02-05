use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub activation_events: Vec<String>,
    #[serde(default)]
    pub contributes: Contributions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub keybindings: Vec<KeybindingContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    pub command: String,
    pub title: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContribution {
    pub command: String,
    pub key: String,
    pub when: Option<String>,
}

pub trait Plugin {
    fn activate(&self) -> anyhow::Result<()>;
    fn deactivate(&self) -> anyhow::Result<()>;
}
