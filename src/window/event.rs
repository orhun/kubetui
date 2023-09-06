use std::{cell::RefCell, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::clipboard_wrapper::Clipboard;

use crate::action::view_id;

use crate::ui::widget::config::WidgetTheme;
use crate::ui::{
    tab::WidgetChunk,
    widget::{config::WidgetConfig, Text, WidgetTrait},
    Tab,
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct EventTheme {
    #[serde(flatten)]
    pub widget: WidgetTheme,
}

pub struct EventsTabBuilder<'a> {
    title: &'a str,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    theme: EventTheme,
}

pub struct EventsTab {
    pub tab: Tab<'static>,
}

impl<'a> EventsTabBuilder<'a> {
    pub fn new(
        title: &'a str,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        theme: EventTheme,
    ) -> Self {
        Self {
            title,
            clipboard,
            theme,
        }
    }

    pub fn build(self) -> EventsTab {
        let event = self.event();

        EventsTab {
            tab: Tab::new(view_id::tab_event, self.title, [WidgetChunk::new(event)]),
        }
    }

    fn event(&self) -> Text {
        let builder = Text::builder()
            .id(view_id::tab_event_widget_event)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Event")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .wrap()
            .follow()
            .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

                config.render_block(text.can_activate() && is_active, is_mouse_over)
            });

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }
}
