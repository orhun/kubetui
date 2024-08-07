use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let base_dir = xdg::BaseDirectories::with_prefix("kubetui")?;

        let config_file = if let Some(path) = path {
            path
        } else {
            base_dir.get_config_file("config.yaml")
        };

        let config = Figment::new()
            .merge(Serialized::defaults(Self::default()))
            .merge(YamlExtended::file(config_file))
            .extract()?;

        dbg!(&config);

        Ok(config)
    }
}
