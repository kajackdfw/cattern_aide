use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_general")]
    pub general:  GeneralConfig,
    #[serde(default)]
    pub provider: ProviderMap,
    #[serde(default)]
    pub project:  Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    #[serde(default = "default_tick_ms")]
    pub tick_ms: u64,
}

fn default_tick_ms() -> u64 { 50 }
fn default_general() -> GeneralConfig { GeneralConfig { tick_ms: 50 } }

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProviderMap {
    pub anthropic: Option<AnthropicConfig>,
    pub opencode:  Option<OpenCodeConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AnthropicConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_anthropic_model() -> String { "claude-opus-4-5".into() }
fn default_mode()             -> String { "api".into() }

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenCodeConfig {
    #[serde(default = "default_opencode_base")]
    pub api_base: String,
    #[serde(default)]
    pub api_key:  String,
    #[serde(default = "default_opencode_model")]
    pub model:    String,
    #[serde(default = "default_mode")]
    pub mode:     String,
}

fn default_opencode_base()  -> String { "http://localhost:4096/v1".into() }
fn default_opencode_model() -> String { "anthropic/claude-sonnet-4-5".into() }

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name:     String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_path")]
    pub path:     String,
}

fn default_provider() -> String { "anthropic".into() }
fn default_path()     -> String { ".".into() }

pub fn load(path: &str) -> Result<Config> {
    let s = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    toml::from_str(&s).context("parsing config.toml")
}

pub fn save(cfg: &Config, path: &str) -> Result<()> {
    let s = toml::to_string_pretty(cfg).context("serializing config")?;
    std::fs::write(path, s).with_context(|| format!("writing {path}"))
}

pub fn default_config() -> Config {
    Config {
        general:  GeneralConfig { tick_ms: 50 },
        provider: ProviderMap::default(),
        project:  vec![
            ProjectConfig { name: "Project Alpha".into(), provider: "anthropic".into(), path: ".".into() },
            ProjectConfig { name: "Project Beta".into(),  provider: "anthropic".into(), path: ".".into() },
        ],
    }
}
