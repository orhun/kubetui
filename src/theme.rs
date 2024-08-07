mod style;
mod tab;

use serde::{Deserialize, Serialize};

pub use self::tab::TabTheme;
pub use style::ThemeStyle;

#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Theme {
    #[serde(default)]
    pub tab: TabTheme,
}

#[cfg(test)]
mod tests {
    use super::*;
}
