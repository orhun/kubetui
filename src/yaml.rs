use crate::ui::theme::UIStyle;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ColorizedYamlTheme {
    key: UIStyle,
    value: UIStyle,
    colon: UIStyle,
}

#[derive(Debug, Clone)]
pub struct ColorizedYaml {
    key_val_regex: Regex,
    key_regex: Regex,
    key_color: UIStyle,
    value_color: UIStyle,
    colon_color: UIStyle,
}

impl ColorizedYaml {
    pub fn new(theme: ColorizedYamlTheme) -> Self {
        Self {
            key_val_regex: Regex::new(r#"\A(\s*)([\w|\-|\.|\/|\s]+|".*"):\s(.+)\z"#).unwrap(),
            key_regex: Regex::new(r#"\A(\s*)([\w|\-|\.|\/|\s]+|".*"):\s*\z"#).unwrap(),
            key_color: theme.key,
            value_color: theme.value,
            colon_color: theme.colon,
        }
    }

    pub fn colorize(&self, s: &str) -> String {
        if let Some((_, [indent, key, value])) =
            self.key_val_regex.captures(s).map(|cap| cap.extract())
        {
            return format!(
                "{indent}{key_color}{key}{colon_color}: {value_color}{value}",
                indent = indent,
                key_color = self.key_color.to_ansi_escape_sequence(),
                key = key,
                colon_color = self.colon_color.to_ansi_escape_sequence(),
                value_color = self.value_color.to_ansi_escape_sequence(),
                value = value
            );
        }

        if let Some((_, [indent, key])) = self.key_regex.captures(s).map(|cap| cap.extract()) {
            return format!(
                "{indent}{key_color}{key}{colon_color}:",
                indent = indent,
                key_color = self.key_color.to_ansi_escape_sequence(),
                key = key,
                colon_color = self.colon_color.to_ansi_escape_sequence(),
            );
        }

        format!(
            "{value_color}{value}",
            value_color = self.value_color.to_ansi_escape_sequence(),
            value = s,
        )
    }
}
