use crossbeam::channel::Sender;
use serde::{Deserialize, Serialize};

use crate::{
    action::view_id,
    event::{
        kubernetes::{context_message::ContextRequest, namespace_message::NamespaceRequest},
        Event,
    },
    ui::{
        event::EventResult,
        widget::{
            config::{WidgetConfig, WidgetTheme},
            MultipleSelect, SingleSelect, Widget,
        },
        Window,
    },
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContextTheme {
    #[serde(flatten)]
    pub widget: WidgetTheme,
}

pub struct ContextPopupBuilder<'a> {
    tx: &'a Sender<Event>,
    theme: ContextTheme,
}

pub struct ContextPopup {
    pub context: Widget<'static>,
    pub single_namespace: Widget<'static>,
    pub multiple_namespaces: Widget<'static>,
}

impl<'a> ContextPopupBuilder<'a> {
    pub fn new(tx: &'a Sender<Event>, theme: ContextTheme) -> Self {
        Self { tx, theme }
    }

    pub fn build(self) -> ContextPopup {
        ContextPopup {
            context: self.context().into(),
            single_namespace: self.single_namespace().into(),
            multiple_namespaces: self.multiple_namespaces().into(),
        }
    }

    fn multiple_namespaces(&self) -> MultipleSelect<'static> {
        let tx = self.tx.clone();

        MultipleSelect::builder()
            .id(view_id::popup_ns)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Namespace")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w: &mut Window, _| {
                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                let mut items: Vec<String> = widget
                    .selected_items()
                    .iter()
                    .map(|i| i.item.to_string())
                    .collect();

                if items.is_empty() {
                    items = vec!["None".to_string()];
                }

                tx.send(NamespaceRequest::Set(items).into())
                    .expect("Failed to send NamespaceRequest::Set");

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_network_widget_description);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_list_widget_list);
                w.widget_clear(view_id::tab_yaml_widget_yaml);

                EventResult::Nop
            })
            .build()
    }

    fn context(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();
        SingleSelect::builder()
            .id(view_id::popup_ctx)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Context")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w: &mut Window, v| {
                let item = v.item.to_string();

                tx.send(ContextRequest::Set(item).into())
                    .expect("Failed to send ContextRequest::Set");

                w.close_popup();

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_network_widget_description);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_list_widget_list);
                w.widget_clear(view_id::tab_yaml_widget_yaml);

                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.unselect_all();

                let widget = w
                    .find_widget_mut(view_id::popup_list)
                    .as_mut_multiple_select();

                widget.unselect_all();

                EventResult::Nop
            })
            .build()
    }

    fn single_namespace(&self) -> SingleSelect<'static> {
        let tx = self.tx.clone();

        SingleSelect::builder()
            .id(view_id::popup_single_ns)
            .widget_config(
                &WidgetConfig::builder()
                    .title("Namespace")
                    .theme(self.theme.widget.clone())
                    .build(),
            )
            .on_select(move |w: &mut Window, v| {
                let items = vec![v.item.to_string()];
                tx.send(NamespaceRequest::Set(items).into())
                    .expect("Failed to send NamespaceRequest::Set");

                w.close_popup();

                w.widget_clear(view_id::tab_pod_widget_log);
                w.widget_clear(view_id::tab_config_widget_raw_data);
                w.widget_clear(view_id::tab_network_widget_description);
                w.widget_clear(view_id::tab_event_widget_event);
                w.widget_clear(view_id::tab_list_widget_list);
                w.widget_clear(view_id::tab_yaml_widget_yaml);

                let widget = w
                    .find_widget_mut(view_id::popup_ns)
                    .as_mut_multiple_select();

                widget.unselect_all();

                widget.select_item(v);

                EventResult::Nop
            })
            .build()
    }
}
