use anyhow::Result;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use serde::{Deserialize, Serialize};

use self::theme::Theme;

pub mod core;
pub mod theme;

/// ```yaml`
/// theme:
///   tab:
///     active:
///       color:
///         fg: blue
///       modifier: none
///     mouse_over:
///       color:
///         fg: blue
///       modifier: reversed
///   header:
///     color:
///       fg: blue
/// ````
#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    pub fn load_config() -> Result<Self> {
        let base_dir = xdg::BaseDirectories::with_prefix("kubetui")?;

        eprintln!("{}", serde_yaml_0_9::to_string(&Config::default()).unwrap());

        let config_file = base_dir.get_config_file("config.yaml");
        let config = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(YamlExtended::file(config_file))
            .extract()?;

        dbg!(&config);

        Ok(config)
    }
}
