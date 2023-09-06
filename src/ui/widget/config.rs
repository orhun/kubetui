use std::{borrow::Cow, fmt::Display};

use ratatui::{
    style::{Color, Modifier},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};
use serde::{Deserialize, Serialize};

use crate::ui::theme::UIStyle;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct WidgetConfigBuilder(WidgetConfig);

#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BorderTypeDef {
    Plain,
    Rounded,
    Double,
    Thick,
}

impl From<BorderTypeDef> for BorderType {
    fn from(value: BorderTypeDef) -> Self {
        match value {
            BorderTypeDef::Plain => BorderType::Plain,
            BorderTypeDef::Rounded => BorderType::Rounded,
            BorderTypeDef::Double => BorderType::Double,
            BorderTypeDef::Thick => BorderType::Thick,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct WidgetBorderStyle {
    style: Option<UIStyle>,
    border_style: Option<UIStyle>,
    title_style: Option<UIStyle>,
    r#type: Option<BorderTypeDef>,
}

impl WidgetBorderStyle {
    fn patch_style<'a>(&self, block: Block<'a>) -> Block<'a> {
        let mut block = block;

        if let Some(style) = &self.style {
            block = block.style(style.to_style());
        }

        if let Some(style) = &self.border_style {
            block = block.border_style(style.to_style());
        }

        if let Some(style) = &self.title_style {
            block = block.title_style(style.to_style());
        }

        if let Some(ty) = self.r#type {
            block = block.border_type(ty.into());
        }

        block
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct WidgetTheme {
    active: WidgetBorderStyle,
    inactive: WidgetBorderStyle,
    mouse_over: WidgetBorderStyle,
}

impl Default for WidgetTheme {
    fn default() -> Self {
        Self {
            active: WidgetBorderStyle {
                title_style: Some(UIStyle {
                    modifier: Some(Modifier::BOLD),
                    ..Default::default()
                }),
                ..Default::default()
            },
            inactive: WidgetBorderStyle {
                title_style: Some(UIStyle {
                    fg: Some(Color::DarkGray),
                    bg: None,
                    modifier: Some(Modifier::empty()),
                }),
                border_style: Some(UIStyle {
                    fg: Some(Color::DarkGray),
                    bg: None,
                    modifier: Some(Modifier::empty()),
                }),
                ..Default::default()
            },
            mouse_over: WidgetBorderStyle {
                border_style: Some(UIStyle {
                    fg: Some(Color::Gray),
                    bg: None,
                    modifier: Some(Modifier::empty()),
                }),
                ..Default::default()
            },
        }
    }
}

/// widgets::Block and Title wrapper
#[derive(Debug, PartialEq, Clone)]
pub struct WidgetConfig {
    title: Title,
    append_title: Option<Title>,
    block: Block<'static>,
    can_activate: bool,
    theme: WidgetTheme,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            title: Default::default(),
            append_title: Default::default(),
            block: Block::default()
                .border_type(BorderType::Plain)
                .borders(Borders::ALL),
            can_activate: true,
            theme: WidgetTheme::default(),
        }
    }
}

/// builder
impl WidgetConfigBuilder {
    pub fn title(mut self, title: impl Into<Title>) -> Self {
        self.0.title = title.into();
        self
    }

    pub fn append_title(mut self, append: impl Into<Title>) -> Self {
        self.0.append_title = Some(append.into());
        self
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.0.block = block;
        self
    }

    /// Border style and title style are default style
    pub fn disable_activation(mut self) -> Self {
        self.0.can_activate = false;
        self
    }

    pub fn theme(mut self, theme: WidgetTheme) -> Self {
        self.0.theme = theme;
        self
    }

    pub fn build(self) -> WidgetConfig {
        self.0
    }
}

impl WidgetConfig {
    pub fn builder() -> WidgetConfigBuilder {
        WidgetConfigBuilder::default()
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn theme(&self) -> &WidgetTheme {
        &self.theme
    }

    pub fn block_mut(&mut self) -> &mut Block<'static> {
        &mut self.block
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn title_mut(&mut self) -> &mut Title {
        &mut self.title
    }

    pub fn append_title(&self) -> &Option<Title> {
        &self.append_title
    }

    pub fn append_title_mut(&mut self) -> &mut Option<Title> {
        &mut self.append_title
    }

    pub fn render_title(&self, is_active: bool) -> Vec<Span<'static>> {
        if self.title.to_string() == "" {
            return Vec::new();
        }

        let mut title = self.title.spans().spans;

        if let Some(append) = &self.append_title {
            title.append(&mut append.spans().spans);
        }

        title.push(" ".into());

        if self.can_activate {
            if is_active {
                title.insert(0, " + ".into());

                // title.iter_mut().for_each(|span| {
                //     // span.style = span.style.add_modifier(Modifier::BOLD);
                // });
            } else {
                title.insert(0, " ".into());

                // title.iter_mut().for_each(|span| {
                //     // span.style = span.style.fg(Color::DarkGray);
                // });
            }
        } else {
            title.insert(0, " ".into());
        }

        title
    }

    /// Render Block
    ///
    /// Active:   ─ + Title ───  (BOLD)
    /// Inactive: ─── Title ───  (DarkGray: title is Raw)
    pub fn render_block(&self, is_active: bool, is_mouse_over: bool) -> Block<'static> {
        let block = if self.can_activate {
            if is_active {
                self.theme.active.patch_style(self.block.clone())
            } else if is_mouse_over {
                self.theme.mouse_over.patch_style(self.block.clone())
            } else {
                self.theme.inactive.patch_style(self.block.clone())
            }
        } else {
            self.theme.active.patch_style(self.block.clone())
        };

        let title = self.render_title(is_active);
        if title.is_empty() {
            block
        } else {
            block.title(title)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Title {
    Raw(String),
    Line(Line<'static>),
    Span(Span<'static>),
}

impl Title {
    pub fn spans(&self) -> Line<'static> {
        match self {
            Title::Raw(title) => Line::from(title.to_string()),
            Title::Line(title) => title.clone(),
            Title::Span(title) => Line::from(title.clone()),
        }
    }
}

impl Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Title::Raw(title) => write!(f, "{}", title),
            Title::Line(title) => write!(
                f,
                "{}",
                title
                    .spans
                    .iter()
                    .cloned()
                    .map(|span| span.content)
                    .collect::<Vec<Cow<str>>>()
                    .concat()
            ),
            Title::Span(title) => write!(f, "{}", title.content),
        }
    }
}

impl Default for Title {
    fn default() -> Self {
        Self::Raw(Default::default())
    }
}

impl From<&str> for Title {
    fn from(title: &str) -> Self {
        Self::Raw(title.into())
    }
}

impl From<String> for Title {
    fn from(title: String) -> Self {
        Self::Raw(title)
    }
}

impl From<&String> for Title {
    fn from(title: &String) -> Self {
        Self::Raw(title.to_string())
    }
}

impl From<Span<'static>> for Title {
    fn from(title: Span<'static>) -> Self {
        Self::Line(title.into())
    }
}

impl From<Line<'static>> for Title {
    fn from(title: Line<'static>) -> Self {
        Self::Line(title)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn render_title() {
        let wc = WidgetConfig::builder()
            .title("Title")
            .disable_activation()
            .build();

        let title = wc.render_title(false);

        assert_eq!(
            vec![Span::raw(" "), Span::raw("Title"), Span::raw(" "),],
            title
        )
    }

    #[test]
    fn render_title_with_append() {
        let wc = WidgetConfig::builder()
            .title("Title")
            .append_title(" append")
            .disable_activation()
            .build();

        let title = wc.render_title(false);

        assert_eq!(
            vec![
                Span::raw(" "),
                Span::raw("Title"),
                Span::raw(" append"),
                Span::raw(" "),
            ],
            title
        )
    }
}
