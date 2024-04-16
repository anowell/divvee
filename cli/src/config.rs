use anyhow::{format_err, Result};
use divvee::repo::Identity;
use serde::Deserialize;
use std::{env, fs, sync::OnceLock};

static CONFIG: OnceLock<Config> = OnceLock::new();
static IDENTITY: OnceLock<Identity> = OnceLock::new();
const IDENTITY_NAME: &str = "default";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    defaults: Defaults,
    // aliases: BtreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Defaults {
    team: Option<String>,
    // assignee: Option<String>,
}

impl Config {
    pub fn init() -> Result<()> {
        let cfg_path = dirs::config_dir()
            .ok_or_else(|| format_err!("No config dir"))?
            .join("divvee/config.toml");
        let mut config = match cfg_path.exists() {
            true => toml::from_str(&fs::read_to_string(cfg_path)?)?,
            false => Config::default(),
        };
        if let Ok(val) = env::var("DIVVEE_TEAM") {
            config.defaults.team = Some(val);
        }

        CONFIG.set(config).unwrap();

        Ok(())
    }

    // Panics if init was never called
    pub fn get() -> &'static Config {
        CONFIG.get().unwrap()
    }

    pub fn defaults() -> &'static Defaults {
        &Config::get().defaults
    }
}

pub fn default_team() -> Result<String> {
    Config::defaults()
        .team
        .clone()
        .ok_or_else(|| format_err!("Default team not specified"))
}

pub fn me() -> &'static Identity {
    IDENTITY.get_or_init(|| Identity::load_global(IDENTITY_NAME).unwrap())
}
