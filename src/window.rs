mod config;
mod context;
mod event;
mod help;
mod list;
mod network;
pub mod pod;
mod yaml;

use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Direction, text::Line, widgets::Paragraph};
use serde::{Deserialize, Serialize};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    context::{Context, Namespace},
    event::{
        kubernetes::{context_message::ContextRequest, namespace_message::NamespaceRequest},
        Event, UserEvent,
    },
    ui::{event::EventResult, popup::Popup, window::WindowTheme, Header, Tab, Window, WindowEvent},
};

use self::{
    config::{ConfigTab, ConfigTabBuilder, ConfigTheme},
    context::{ContextPopup, ContextPopupBuilder, ContextTheme},
    event::{EventTheme, EventsTab, EventsTabBuilder},
    help::{HelpPopup, HelpTheme},
    list::{ListTab, ListTabBuilder, ListTheme},
    network::{NetworkTab, NetworkTabBuilder, NetworkTheme},
    pod::{PodTabBuilder, PodTheme, PodsTab},
    yaml::{YamlTab, YamlTabBuilder, YamlTheme},
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ViewTheme {
    #[serde(flatten)]
    pub window: WindowTheme,
    pub pod: PodTheme,
    pub config: ConfigTheme,
    pub network: NetworkTheme,
    pub event: EventTheme,
    pub list: ListTheme,
    pub yaml: YamlTheme,
    pub context: ContextTheme,
    pub help: HelpTheme,
}

pub struct WindowInit {
    split_mode: Direction,
    tx: Sender<Event>,
    context: Rc<RefCell<Context>>,
    namespaces: Rc<RefCell<Namespace>>,
    theme: ViewTheme,
}

impl WindowInit {
    pub fn new(
        split_mode: Direction,
        tx: Sender<Event>,
        context: Rc<RefCell<Context>>,
        namespaces: Rc<RefCell<Namespace>>,
        theme: ViewTheme,
    ) -> Self {
        Self {
            split_mode,
            tx,
            context,
            namespaces,
            theme,
        }
    }

    pub fn build(self) -> Window<'static> {
        let (tabs, popups) = self.tabs_popups();

        let builder = Window::builder().tabs(tabs).popup(popups);

        // Configure Action
        let tx = self.tx.clone();
        let builder = builder.action(
            UserEvent::Key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT)),
            move |w| {
                tx.send(NamespaceRequest::Get.into())
                    .expect("Failed to send NamespaceRequest::Get");
                w.open_popup(view_id::popup_ns);
                EventResult::Nop
            },
        );

        let tx = self.tx.clone();
        let builder = builder.action('n', move |w| {
            tx.send(NamespaceRequest::Get.into())
                .expect("Failed to send NamespaceRequest::Get");
            w.open_popup(view_id::popup_single_ns);
            EventResult::Nop
        });

        let fn_close = |w: &mut Window| {
            if w.opening_popup() {
                w.close_popup();
                EventResult::Nop
            } else {
                EventResult::Window(WindowEvent::CloseWindow)
            }
        };

        let tx = self.tx.clone();
        let builder = builder.action('c', move |w| {
            tx.send(ContextRequest::Get.into())
                .expect("Failed to send ContextRequest::Get");
            w.open_popup(view_id::popup_ctx);
            EventResult::Nop
        });

        let open_help = move |w: &mut Window| {
            w.open_popup(view_id::popup_help);
            EventResult::Nop
        };

        let builder = builder.action('h', open_help).action('?', open_help);

        let builder = builder.action('q', fn_close).action(KeyCode::Esc, fn_close);

        let context = self.context.clone();
        let namespaces = self.namespaces.clone();

        let header = Header::new_callback(2, move || {
            let context = context.borrow();
            let namespaces = namespaces.borrow();
            Paragraph::new(vec![
                Line::from(format!(" ctx: {}", context)),
                Line::from(format!(" ns: {}", namespaces)),
            ])
        });

        let builder = builder.header(header);

        let builder = builder.theme(self.theme.window);

        builder.build()
    }

    fn tabs_popups(&self) -> (Vec<Tab<'static>>, Vec<Popup<'static>>) {
        let clipboard = Some(Rc::new(RefCell::new(Clipboard::new())));

        let PodsTab { tab: tab_pods } = PodTabBuilder::new(
            "Pod",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.theme.pod.clone(),
        )
        .build();

        let ConfigTab { tab: tab_configs } = ConfigTabBuilder::new(
            "Config",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.theme.config.clone(),
        )
        .build();

        let NetworkTab { tab: tab_network } = NetworkTabBuilder::new(
            "Network",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.theme.network.clone(),
        )
        .build();

        let EventsTab { tab: tab_events } =
            EventsTabBuilder::new("Event", &clipboard, self.theme.event.clone()).build();

        let ListTab {
            tab: tab_list,
            popup: popup_list,
        } = ListTabBuilder::new("List", &self.tx, &clipboard, self.theme.list.clone()).build();

        let YamlTab {
            tab: tab_yaml,
            popup_kind: popup_yaml_kind,
            popup_name: popup_yaml_name,
            popup_return: popup_yaml_return,
        } = YamlTabBuilder::new("Yaml", &self.tx, &clipboard, self.theme.yaml.clone()).build();

        let ContextPopup {
            context: popup_context,
            single_namespace: popup_single_namespace,
            multiple_namespaces: popup_multiple_namespaces,
        } = ContextPopupBuilder::new(&self.tx, self.theme.context.clone()).build();

        let HelpPopup {
            content: popup_help,
        } = HelpPopup::new(self.theme.help.clone());

        // Init Window
        let tabs = vec![
            tab_pods,
            tab_configs,
            tab_network,
            tab_events,
            tab_list,
            tab_yaml,
        ];

        let popups = vec![
            Popup::new(popup_context),
            Popup::new(popup_single_namespace),
            Popup::new(popup_multiple_namespaces),
            Popup::new(popup_list),
            Popup::new(popup_yaml_kind),
            Popup::new(popup_yaml_name),
            Popup::new(popup_yaml_return),
            Popup::new(popup_help),
        ];

        (tabs, popups)
    }
}
