use ratatui::{style::Color, style::Modifier, symbols};
use serde::{Deserialize, Serialize};

use super::ThemeStyle;

/// タブのテーマ
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TabTheme {
    #[serde(default)]
    pub divider: String,

    #[serde(default)]
    pub base: ThemeStyle,

    #[serde(default)]
    pub active: ThemeStyle,

    #[serde(default = "default_mouse_over")]
    pub mouse_over: ThemeStyle,
}

impl Default for TabTheme {
    fn default() -> Self {
        Self {
            divider: symbols::line::VERTICAL.to_string(),
            base: ThemeStyle::default(),
            active: default_active(),
            mouse_over: default_mouse_over(),
        }
    }
}

fn default_active() -> ThemeStyle {
    ThemeStyle {
        modifier: Modifier::REVERSED,
        ..Default::default()
    }
}

fn default_mouse_over() -> ThemeStyle {
    ThemeStyle {
        modifier: Modifier::REVERSED,
        fg_color: Color::DarkGray,
        ..Default::default()
    }
}
