use std::str::FromStr;

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default, Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct UIStyle {
    #[serde(deserialize_with = "deserialize_color")]
    pub fg: Option<Color>,
    #[serde(deserialize_with = "deserialize_color")]
    pub bg: Option<Color>,

    #[serde(
        deserialize_with = "deserialize_modifier",
        serialize_with = "serialize_modifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub modifier: Option<Modifier>,
}

impl UIStyle {
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = Some(modifier);
        self
    }

    pub fn to_ansi_escape_sequence(&self) -> String {
        let mut buf = String::new();

        if let Some(fg) = self.fg {
            buf += "\x1b[";
            match fg {
                Color::Black => {
                    buf += "30";
                }
                Color::Red => {
                    buf += "31";
                }
                Color::Green => {
                    buf += "32";
                }
                Color::Yellow => {
                    buf += "33";
                }
                Color::Blue => {
                    buf += "34";
                }
                Color::Magenta => {
                    buf += "35";
                }
                Color::Cyan => {
                    buf += "36";
                }
                Color::White => {
                    buf += "37";
                }
                Color::Rgb(r, g, b) => {
                    buf += &format!("38;2;{r};{g};{b}", r = r, g = g, b = b);
                }
                Color::Indexed(n) => {
                    buf += &format!("38;5;{}", n);
                }
                Color::Reset => {
                    buf += "39";
                }
                Color::DarkGray => {
                    buf += "90";
                }
                Color::LightRed => {
                    buf += "91";
                }
                Color::LightGreen => {
                    buf += "92";
                }
                Color::LightYellow => {
                    buf += "93";
                }
                Color::LightBlue => {
                    buf += "94";
                }
                Color::LightMagenta => {
                    buf += "95";
                }
                Color::LightCyan => {
                    buf += "96";
                }
                Color::Gray => {
                    buf += "97";
                }
            };
        }

        if let Some(bg) = self.bg {
            if buf.is_empty() {
                buf += "\x1b[";
            } else {
                buf += ";";
            }
            match bg {
                Color::Black => {
                    buf += "40";
                }
                Color::Red => {
                    buf += "41";
                }
                Color::Green => {
                    buf += "42";
                }
                Color::Yellow => {
                    buf += "43";
                }
                Color::Blue => {
                    buf += "44";
                }
                Color::Magenta => {
                    buf += "45";
                }
                Color::Cyan => {
                    buf += "46";
                }
                Color::White => {
                    buf += "47";
                }
                Color::Rgb(r, g, b) => {
                    buf += &format!("48;2;{r};{g};{b}", r = r, g = g, b = b);
                }
                Color::Indexed(n) => {
                    buf += &format!("48;5;{}", n);
                }
                Color::Reset => {
                    buf += "49";
                }
                Color::DarkGray => {
                    buf += "100";
                }
                Color::LightRed => {
                    buf += "101";
                }
                Color::LightGreen => {
                    buf += "102";
                }
                Color::LightYellow => {
                    buf += "103";
                }
                Color::LightBlue => {
                    buf += "104";
                }
                Color::LightMagenta => {
                    buf += "105";
                }
                Color::LightCyan => {
                    buf += "106";
                }
                Color::Gray => {
                    buf += "107";
                }
            };
        }

        if let Some(modifier) = self.modifier {
            if buf.is_empty() {
                buf += "\x1b[";
            } else {
                buf += ";";
            }
            for (i, m) in modifier.iter().enumerate() {
                if i != 0 {
                    buf += ";";
                }

                let s = match m {
                    Modifier::BOLD => "1",
                    Modifier::DIM => "2",
                    Modifier::ITALIC => "3",
                    Modifier::UNDERLINED => "4",
                    Modifier::SLOW_BLINK => "5",
                    Modifier::RAPID_BLINK => "6",
                    Modifier::REVERSED => "7",
                    Modifier::HIDDEN => "8",
                    Modifier::CROSSED_OUT => "9",
                    _ => {
                        unreachable!()
                    }
                };

                buf += s;
            }
        }

        if !buf.is_empty() {
            buf += "m";
        }

        buf
    }
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;

    if let Some(s) = s {
        if s.is_empty() {
            return Ok(None);
        }

        if s == "none" {
            return Ok(None);
        }

        Color::from_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom)
    } else {
        Ok(None)
    }
}

const MODIFIER_BOLD: &str = "bold";
const MODIFIER_DIM: &str = "dim";
const MODIFIER_ITALIC: &str = "italic";
const MODIFIER_UNDERLINED: &str = "underlined";
const MODIFIER_SLOW_BLINK: &str = "slow blink";
const MODIFIER_RAPID_BLINK: &str = "rapid blink";
const MODIFIER_REVERSED: &str = "reversed";
const MODIFIER_HIDDEN: &str = "hidden";
const MODIFIER_CROSSED_OUT: &str = "crossed out";
const MODIFIER_NONE: &str = "none";

fn serialize_modifier<S>(value: &Option<Modifier>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(value) = value {
        if value.is_empty() {
            serializer.serialize_str(MODIFIER_NONE)
        } else {
            let mut modifiers = Vec::new();

            for m in *value {
                let s = match m {
                    Modifier::BOLD => MODIFIER_BOLD,
                    Modifier::DIM => MODIFIER_DIM,
                    Modifier::ITALIC => MODIFIER_ITALIC,
                    Modifier::UNDERLINED => MODIFIER_UNDERLINED,
                    Modifier::SLOW_BLINK => MODIFIER_SLOW_BLINK,
                    Modifier::RAPID_BLINK => MODIFIER_RAPID_BLINK,
                    Modifier::REVERSED => MODIFIER_REVERSED,
                    Modifier::HIDDEN => MODIFIER_HIDDEN,
                    Modifier::CROSSED_OUT => MODIFIER_CROSSED_OUT,
                    _ => return Err(serde::ser::Error::custom("unknown modifier")),
                };

                modifiers.push(s);
            }

            let modifiers = modifiers.join(" | ");

            serializer.serialize_str(&modifiers)
        }
    } else {
        serializer.serialize_none()
    }
}

fn deserialize_modifier<'de, D>(deserializer: D) -> Result<Option<Modifier>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.is_empty() {
        return Ok(Some(Modifier::empty()));
    }

    if s.to_ascii_lowercase() == "none" {
        return Ok(Some(Modifier::empty()));
    }

    let modifiers = s.split('|');

    let mut result = Modifier::empty();

    for m in modifiers {
        result |= match m.to_lowercase().as_str().trim() {
                    MODIFIER_BOLD => Modifier::BOLD,
                    MODIFIER_DIM => Modifier::DIM,
                    MODIFIER_ITALIC => Modifier::ITALIC,
                    MODIFIER_UNDERLINED => Modifier::UNDERLINED,
                    MODIFIER_SLOW_BLINK => Modifier::SLOW_BLINK,
                    MODIFIER_RAPID_BLINK => Modifier::RAPID_BLINK,
                    MODIFIER_REVERSED => Modifier::REVERSED,
                    MODIFIER_HIDDEN => Modifier::HIDDEN,
                    MODIFIER_CROSSED_OUT => Modifier::CROSSED_OUT,
                    MODIFIER_NONE => {
                        return Ok(Some(Modifier::empty()));
                    },
                    _ => return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(m), &"bold | dim | italic | underlined | slow blink | rapid blink | reversed | crossed out")),
                };
    }

    Ok(Some(result))
}

impl UIStyle {
    pub fn to_style(&self) -> Style {
        let mut style = Style::default();

        if let Some(fg) = self.fg {
            style = style.fg(fg)
        }

        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }

        if let Some(modifier) = self.modifier {
            if modifier.is_empty() {
                style = style.remove_modifier(Modifier::all());
            } else {
                style = style
                    .remove_modifier(Modifier::all())
                    .add_modifier(modifier)
            }
        }

        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde_yaml_0_9 as serde_yaml;

    #[test]
    fn deserialize_hex() {
        let s = indoc! {r##"
                    fg: "#000000"
                    bg: "#FFFFFF"
                "##};

        let actual: UIStyle = serde_yaml::from_str(s).unwrap();

        let expected = UIStyle {
            fg: Some(Color::Rgb(0, 0, 0)),
            bg: Some(Color::Rgb(255, 255, 255)),
            modifier: None,
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn deserialize_empty() {
        let actual: UIStyle = serde_yaml::from_str("").unwrap();

        let expected = UIStyle::default();

        assert_eq!(actual, expected)
    }

    #[test]
    fn deserialize_name() {
        let s = indoc! {r#"
                    fg: black
                    bg: white
                "#};

        let actual: UIStyle = serde_yaml::from_str(s).unwrap();

        let expected = UIStyle {
            fg: Some(Color::Black),
            bg: Some(Color::White),
            modifier: None,
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn deserialize_invalid_value() {
        let s = indoc! {r#"
                    fg: hoge
                    bg: fuga
		    modifier: hoge
                "#};

        assert!(serde_yaml::from_str::<UIStyle>(s).is_err())
    }

    #[test]
    fn deserialize_bold() {
        let s = indoc! {r#"
                    modifier: bold
                "#};

        let actual: UIStyle = serde_yaml::from_str(s).unwrap();

        let expected = UIStyle {
            modifier: Some(Modifier::BOLD),
            ..Default::default()
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn deserialize_all() {
        let s = indoc! {r#"
                    modifier: bold | dim | italic | underline | slow blink | rapid blink | reversed | hidden | crossed out
                "#};

        let actual: UIStyle = serde_yaml::from_str(s).unwrap();

        let expected = UIStyle {
            modifier: Some(Modifier::all()),
            ..Default::default()
        };

        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case(UIStyle::default(), "")]
    #[case(UIStyle::default().fg(Color::Red), "\x1b[31m")]
    #[case(UIStyle::default().bg(Color::Red), "\x1b[41m")]
    #[case(UIStyle::default().modifier(Modifier::BOLD), "\x1b[1m")]
    #[case(UIStyle::default().fg(Color::Red).bg(Color::Green), "\x1b[31;42m")]
    #[case(UIStyle::default().fg(Color::Green).modifier(Modifier::BOLD), "\x1b[32;1m")]
    #[case(UIStyle::default().bg(Color::Green).modifier(Modifier::BOLD), "\x1b[42;1m")]
    #[case(UIStyle::default().fg(Color::Red).bg(Color::Green).modifier(Modifier::BOLD), "\x1b[31;42;1m")]
    fn to_ansi_escape_sequence(#[case] ui_style: UIStyle, #[case] expected: &str) {
        assert_eq!(ui_style.to_ansi_escape_sequence(), expected);
    }
}
