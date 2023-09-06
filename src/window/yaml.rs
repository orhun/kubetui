use crossbeam::channel::Sender;
use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::{kubernetes::yaml::YamlRequest, Event},
    logger,
    ui::{
        event::EventResult,
        tab::WidgetChunk,
        widget::Widget,
        widget::{
            config::{WidgetConfig, WidgetTheme},
            SingleSelect, Text, WidgetTrait,
        },
        Tab, Window,
    },
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct YamlTheme {
    #[serde(flatten)]
    pub widget: WidgetTheme,
}

pub struct YamlTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    theme: YamlTheme,
}

pub struct YamlTab {
    pub tab: Tab<'static>,
    pub popup_kind: Widget<'static>,
    pub popup_name: Widget<'static>,
    pub popup_return: Widget<'static>,
}

impl<'a> YamlTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        theme: YamlTheme,
    ) -> Self {
        Self {
            title,
            tx,
            clipboard,
            theme,
        }
    }

    pub fn build(self) -> YamlTab {
        let yaml = self.main();
        YamlTab {
            tab: Tab::new(view_id::tab_yaml, self.title, [WidgetChunk::new(yaml)]),
            popup_kind: self.subwin_kind().into(),
            popup_name: self.subwin_name().into(),
            popup_return: self.subwin_return().into(),
        }
    }

    fn main(&self) -> Text {
        let tx = self.tx.clone();

        let open_subwin = move |w: &mut Window| {
            tx.send(YamlRequest::APIs.into())
                .expect("YamlRequest::APIs");
            w.open_popup(view_id::popup_yaml_kind);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_yaml_widget_yaml)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Yaml")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

                config.render_block(text.can_activate() && is_active, is_mouse_over)
            })
            .action('f', open_subwin)
            .wrap();

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }

    fn subwin_kind(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();

        SingleSelect::builder()
            .id(view_id::popup_yaml_kind)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Kind")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w, v| {
                logger!(info, "Select Item: {:?}", v);

                w.close_popup();

                let Some(metadata) = v.metadata.as_ref() else {
                    unreachable!()
                };

                let Some(key) = metadata.get("key") else {
                    unreachable!()
                };

                let Ok(kind) = serde_json::from_str(key) else {
                    unreachable!()
                };

                tx.send(YamlRequest::Resource(kind).into())
                    .expect("Failed to send YamlRequest::Resource");

                EventResult::Nop
            })
            .build()
    }

    fn subwin_name(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();

        SingleSelect::builder()
            .id(view_id::popup_yaml_name)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Name")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w, v| {
                logger!(info, "Select Item: {:?}", v);

                w.close_popup();

                let Some(metadata) = v.metadata.as_ref() else {
                    unreachable!()
                };

                let Some(namespace) = metadata.get("namespace") else {
                    unreachable!()
                };

                let Some(name) = metadata.get("name") else {
                    unreachable!()
                };

                let Some(key) = metadata.get("key") else {
                    unreachable!()
                };

                let Ok(kind) = serde_json::from_str(key) else {
                    unreachable!()
                };

                tx.send(
                    YamlRequest::Yaml {
                        kind,
                        name: name.to_string(),
                        namespace: namespace.to_string(),
                    }
                    .into(),
                )
                .expect("Failed to send YamlRequest::Yaml");

                EventResult::Nop
            })
            .action(KeyCode::Esc, move |w| {
                w.open_popup(view_id::popup_yaml_kind);
                if let Widget::SingleSelect(w) = w.find_widget_mut(view_id::popup_yaml_kind) {
                    w.clear_filter();
                }
                EventResult::Nop
            })
            .build()
    }

    fn subwin_return(&self) -> Text {
        let return_kind = move |w: &mut Window| {
            w.open_popup(view_id::popup_yaml_kind);

            if let Widget::SingleSelect(w) = w.find_widget_mut(view_id::popup_yaml_kind) {
                w.clear_filter();
            }

            EventResult::Nop
        };

        Text::builder()
            .id(view_id::popup_yaml_return)
            .widget_config(&WidgetConfig::builder().title("Name").theme(self.theme.widget.clone()).build())
            .items(
                [
                    "No resources found.",
                    "",
                    "Press \x1b[1mEnter\x1b[0m or \x1b[1mEsc\x1b[0m to return to resource selection.",
                ]
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            )
            .wrap()
            .action(KeyCode::Enter, return_kind)
            .action(KeyCode::Esc, return_kind)
            .build()
    }
}
