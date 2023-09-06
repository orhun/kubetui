use crossbeam::channel::Sender;
use serde::{Deserialize, Serialize};

use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::kubernetes::api_resources::ApiRequest,
    event::Event,
    ui::{
        event::EventResult,
        tab::WidgetChunk,
        widget::{
            config::{WidgetConfig, WidgetTheme},
            MultipleSelect, SelectedItem, Text, Widget, WidgetTrait,
        },
        Tab, Window,
    },
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ListTheme {
    #[serde(flatten)]
    pub widget: WidgetTheme,
}

pub struct ListTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    theme: ListTheme,
}

pub struct ListTab {
    pub tab: Tab<'static>,
    pub popup: Widget<'static>,
}

impl<'a> ListTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        theme: ListTheme,
    ) -> Self {
        Self {
            title,
            tx,
            clipboard,
            theme,
        }
    }

    pub fn build(self) -> ListTab {
        let list = self.list();

        ListTab {
            tab: Tab::new(view_id::tab_list, self.title, [WidgetChunk::new(list)]),
            popup: self.popup().into(),
        }
    }

    fn list(&self) -> Text {
        let tx = self.tx.clone();

        let open_subwin = move |w: &mut Window| {
            tx.send(ApiRequest::Get.into())
                .expect("Failed to send ApiRequest::Get");
            w.open_popup(view_id::popup_list);
            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_list_widget_list)
            .widget_config(
                &WidgetConfig::builder()
                    .title("List")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

                config.render_block(text.can_activate() && is_active, is_mouse_over)
            })
            .action('f', open_subwin);

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }

    fn popup(&self) -> MultipleSelect<'static> {
        let tx = self.tx.clone();

        MultipleSelect::builder()
            .id(view_id::popup_list)
            .widget_config(
                &WidgetConfig::builder()
                    .title("List")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_list)
                    .as_mut_multiple_select();

                if let Some(SelectedItem::Array(items)) = widget.widget_item() {
                    let list = items
                        .iter()
                        .map(|item| {
                            let Some(metadata) = &item.metadata else { unreachable!() };

                            let Some(key) = metadata.get("key") else { unreachable!() };

                            let Ok(key) = serde_json::from_str(key) else { unreachable!() };

                            key
                        })
                        .collect();

                    tx.send(ApiRequest::Set(list).into())
                        .expect("Failed to send ApiRequest::Set");
                }

                if widget.selected_items().is_empty() {
                    w.widget_clear(view_id::tab_list_widget_list)
                }

                EventResult::Nop
            })
            .build()
    }
}
