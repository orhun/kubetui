use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};
/// Theme用のスタイル
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ThemeStyle {
    #[serde(with = "serde_color", default)]
    pub fg_color: Color,

    #[serde(with = "serde_color", default)]
    pub bg_color: Color,

    #[serde(with = "serde_modifier", default)]
    pub modifier: Modifier,
}

impl ThemeStyle {
    pub fn to_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::new()
            .fg(self.fg_color)
            .bg(self.bg_color)
            .add_modifier(self.modifier)
    }
}

/// Modifierに対して大文字・小文字を区別せずにパースできるように拡張する
mod serde_modifier {
    use serde::Deserialize as _;

    use super::Modifier;

    pub fn serialize<S>(modifier: &Modifier, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if modifier.is_empty() {
            return serializer.serialize_str("none");
        }

        let s = serde_yaml::to_string(modifier)
            .map_err(serde::ser::Error::custom)?
            .trim_end()
            .to_lowercase();

        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Modifier, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let s = s.to_uppercase();

        match s.as_str() {
            "NONE" => Ok(Modifier::empty()),
            _ => serde_yaml::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}

mod serde_color {
    use serde::Deserialize as _;

    use super::Color;

    /// Colorをシリアライズした結果を小文字に変換する
    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if *color == Color::default() {
            return serializer.serialize_str("default");
        }

        let s = serde_yaml::to_string(color)
            .map_err(serde::ser::Error::custom)?
            .trim_end()
            .to_lowercase();

        serializer.serialize_str(&s)
    }

    /// Colorをデシリアライズする際に"default"文字列をサポートし、Color::Defaultに変換する
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_str() {
            "default" => Ok(Color::default()),
            _ => serde_yaml::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod style {
        use super::*;

        use indoc::indoc;
        use pretty_assertions::assert_eq;

        #[test]
        fn default_theme_style() {
            let actual = ThemeStyle::default();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }

        #[test]
        fn serialize_theme_style() {
            let theme = ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD | Modifier::ITALIC,
            };

            let actual = serde_yaml::to_string(&theme).unwrap();

            let expected = indoc! { "
                fg_color: red
                bg_color: blue
                modifier: bold | italic
            " };

            assert_eq!(actual, expected);
        }

        #[test]
        fn deserialize_theme_style() {
            let yaml = indoc! { "
                fg_color: red
                bg_color: blue
                modifier: bold | italic
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD | Modifier::ITALIC,
            };

            assert_eq!(actual, expected);
        }

        /// 空文字を与えたときにDefault値が返ることを確認する
        #[test]
        fn deserialize_theme_style_empty_string() {
            let yaml = "";

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle::default();

            assert_eq!(actual, expected);
        }

        /// "default"を与えたときにColor::default()が返ることを確認する
        #[test]
        fn deserialize_color_default_string() {
            let yaml = indoc! { "
                fg_color: default
                bg_color: default
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }

        /// "none"を与えたときにModifier::empty()が返ることを確認する
        #[test]
        fn deserialize_modifier_none_string() {
            let yaml = indoc! { "
                modifier: none
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }
    }
}
