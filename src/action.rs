use std::collections::BTreeMap;

use crossbeam::channel::Receiver;

use crate::{
    context::{Context, Namespace},
    error::Result,
    message::Message,
    ui::{
        event::{exec_to_window_event, EventResult},
        util::chars::convert_tabs_to_spaces,
        widget::{Item, LiteralItem, TableItem, WidgetTrait},
        Window, WindowEvent,
    },
    workers::kube::{
        api_resources::{ApiMessage, ApiResponse},
        config::ConfigMessage,
        context_message::{ContextMessage, ContextResponse},
        namespace_message::{NamespaceMessage, NamespaceResponse},
        network::{NetworkMessage, NetworkResponse},
        pod::LogMessage,
        yaml::{YamlMessage, YamlResourceListItem, YamlResponse},
        Kube, KubeTable, KubeTableRow,
    },
};

pub mod view_id {

    #![allow(non_upper_case_globals)]
    macro_rules! generate_id {
        ($id:ident) => {
            pub const $id: &str = stringify!($id);
        };
    }

    generate_id!(tab_pod);
    generate_id!(tab_pod_widget_pod);
    generate_id!(tab_pod_widget_log_query);
    generate_id!(tab_pod_widget_log_query_help);
    generate_id!(tab_pod_widget_log);
    generate_id!(tab_config);
    generate_id!(tab_config_widget_config);
    generate_id!(tab_config_widget_raw_data);
    generate_id!(tab_network);
    generate_id!(tab_network_widget_network);
    generate_id!(tab_network_widget_description);
    generate_id!(tab_event);
    generate_id!(tab_event_widget_event);
    generate_id!(tab_list);
    generate_id!(tab_list_widget_list);
    generate_id!(tab_yaml);
    generate_id!(tab_yaml_widget_yaml);

    generate_id!(popup_ctx);
    generate_id!(popup_ns);
    generate_id!(popup_list);
    generate_id!(popup_single_ns);

    generate_id!(popup_yaml_name);
    generate_id!(popup_yaml_kind);
    generate_id!(popup_yaml_return);

    generate_id!(popup_yaml);

    generate_id!(popup_help);
}

macro_rules! error_format {
    ($fmt:literal, $($arg:tt)*) => {
        format!(concat!("\x1b[31m[kubetui] ", $fmt,"\x1b[39m"), $($arg)*)
    };
}

macro_rules! error_lines {
    ($err:ident) => {
        format!("{:?}", $err)
            .lines()
            .map(|line| LiteralItem {
                item: error_format!("{}", line),
                metadata: None,
            })
            .collect::<Vec<_>>()
    };
}

pub fn window_action(window: &mut Window, rx: &Receiver<Message>) -> WindowEvent {
    match rx.recv().expect("Failed to recv") {
        Message::User(ev) => match window.on_event(ev) {
            EventResult::Nop => {}

            EventResult::Ignore => {
                if let Some(cb) = window.match_callback(ev) {
                    if let EventResult::Window(ev) = (cb)(window) {
                        return ev;
                    }
                }
            }
            ev @ EventResult::Callback(_) => {
                return exec_to_window_event(ev, window);
            }
            EventResult::Window(ev) => {
                return ev;
            }
        },

        Message::Tick => {}
        Message::Kube(k) => return WindowEvent::UpdateContents(k),
        Message::Error(_) => {}
    }
    WindowEvent::Continue
}

fn update_widget_item_for_table(window: &mut Window, id: &str, table: Result<KubeTable>) {
    let widget = window.find_widget_mut(id);
    let w = widget.as_mut_table();

    match table {
        Ok(table) => {
            if w.equal_header(table.header()) {
                w.update_widget_item(Item::Table(
                    table
                        .rows
                        .into_iter()
                        .map(
                            |KubeTableRow {
                                 namespace,
                                 name,
                                 metadata,
                                 row,
                             }| {
                                let mut item_metadata = BTreeMap::from([
                                    ("namespace".to_string(), namespace),
                                    ("name".to_string(), name),
                                ]);

                                if let Some(metadata) = metadata {
                                    item_metadata.extend(metadata);
                                }

                                TableItem {
                                    metadata: Some(item_metadata),
                                    item: row,
                                }
                            },
                        )
                        .collect(),
                ));
            } else {
                let rows: Vec<TableItem> = table
                    .rows
                    .into_iter()
                    .map(
                        |KubeTableRow {
                             namespace,
                             name,
                             metadata,
                             row,
                         }| {
                            let mut item_metadata = BTreeMap::from([
                                ("namespace".to_string(), namespace),
                                ("name".to_string(), name),
                            ]);

                            if let Some(metadata) = metadata {
                                item_metadata.extend(metadata);
                            }

                            TableItem {
                                metadata: Some(item_metadata),
                                item: row,
                            }
                        },
                    )
                    .collect();

                w.update_header_and_rows(&table.header, &rows);
            }
        }
        Err(e) => {
            let rows: Vec<TableItem> = vec![vec![error_format!("{:?}", e)].into()];
            w.update_header_and_rows(&["ERROR".to_string()], &rows);
        }
    }
}

fn update_widget_item_for_vec(window: &mut Window, id: &str, vec: Result<Vec<String>>) {
    let widget = window.find_widget_mut(id);
    match vec {
        Ok(i) => {
            widget.update_widget_item(Item::Array(i.into_iter().map(LiteralItem::from).collect()));
        }
        Err(e) => {
            widget.update_widget_item(Item::Array(error_lines!(e)));
        }
    }
}

pub fn update_contents(
    window: &mut Window,
    ev: Kube,
    context: &mut Context,
    namespace: &mut Namespace,
) {
    match ev {
        Kube::Pod(pods_table) => {
            update_widget_item_for_table(window, view_id::tab_pod_widget_pod, pods_table);
        }

        Kube::Log(LogMessage::Response(res)) => {
            let widget = window.find_widget_mut(view_id::tab_pod_widget_log);

            match res {
                Ok(i) => {
                    let array = i
                        .into_iter()
                        .map(|i| LiteralItem {
                            metadata: None,
                            item: convert_tabs_to_spaces(i),
                        })
                        .collect();

                    widget.append_widget_item(Item::Array(array));
                }
                Err(e) => {
                    widget.append_widget_item(Item::Array(error_lines!(e)));
                }
            }
        }

        Kube::Config(ConfigMessage::Response(res)) => {
            use crate::workers::kube::config::ConfigResponse::*;

            match res {
                Table(list) => {
                    update_widget_item_for_table(window, view_id::tab_config_widget_config, list);
                }
                Data(data) => {
                    update_widget_item_for_vec(window, view_id::tab_config_widget_raw_data, data);
                }
            }
        }

        Kube::Event(ev) => {
            update_widget_item_for_vec(window, view_id::tab_event_widget_event, ev);
        }

        Kube::Namespace(NamespaceMessage::Response(res)) => match res {
            NamespaceResponse::Get(res) => match res {
                Ok(namespaces) => {
                    window
                        .find_widget_mut(view_id::popup_ns)
                        .update_widget_item(Item::Array(
                            namespaces.iter().cloned().map(LiteralItem::from).collect(),
                        ));
                    window
                        .find_widget_mut(view_id::popup_single_ns)
                        .update_widget_item(Item::Array(
                            namespaces.iter().cloned().map(LiteralItem::from).collect(),
                        ));
                }
                Err(err) => {
                    let err = error_lines!(err);
                    window
                        .find_widget_mut(view_id::popup_ns)
                        .update_widget_item(Item::Array(err.to_vec()));

                    window
                        .find_widget_mut(view_id::popup_single_ns)
                        .update_widget_item(Item::Array(err));
                }
            },
            NamespaceResponse::Set(res) => {
                namespace.update(res);
            }
        },

        Kube::Context(ContextMessage::Response(res)) => match res {
            ContextResponse::Get(res) => {
                update_widget_item_for_vec(window, view_id::popup_ctx, Ok(res));
            }
        },

        Kube::RestoreContext {
            context: ctx,
            namespaces: ns,
        } => {
            context.update(ctx);
            namespace.update(ns.clone());

            window
                .find_widget_mut(view_id::popup_ns)
                .update_widget_item(Item::Array(
                    ns.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(view_id::popup_ns)
                .as_mut_multiple_select()
                .select_all();
        }

        Kube::RestoreAPIs(list) => {
            let w = window
                .find_widget_mut(view_id::popup_list)
                .as_mut_multiple_select();

            for key in list {
                let Ok(json) = serde_json::to_string(&key) else {
                    unreachable!()
                };

                let metadata = BTreeMap::from([("key".into(), json)]);

                let item = if key.is_api() || key.is_preferred_version() {
                    key.to_string()
                } else {
                    format!("\x1b[90m{}\x1b[39m", key)
                };

                let literal_item = LiteralItem::new(item, Some(metadata));

                w.select_item(&literal_item);
            }
        }

        Kube::API(ApiMessage::Response(res)) => {
            use ApiResponse::*;
            match res {
                Get(list) => {
                    let widget = window.find_widget_mut(view_id::popup_list);
                    match list {
                        Ok(i) => {
                            let items = i
                                .into_iter()
                                .map(|key| {
                                    let Ok(json) = serde_json::to_string(&key) else {
                                        unreachable!()
                                    };
                                    let metadata = BTreeMap::from([("key".into(), json)]);

                                    let item = if key.is_api() || key.is_preferred_version() {
                                        key.to_string()
                                    } else {
                                        format!("\x1b[90m{}\x1b[39m", key)
                                    };

                                    LiteralItem::new(item, Some(metadata))
                                })
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                        Err(e) => {
                            widget.update_widget_item(Item::Array(error_lines!(e)));
                        }
                    }
                }
                Set(_) => {}
                Poll(list) => {
                    update_widget_item_for_vec(window, view_id::tab_list_widget_list, list);
                }
            }
        }

        Kube::Yaml(YamlMessage::Response(ev)) => {
            use YamlResponse::*;
            match ev {
                APIs(res) => {
                    let widget = window.find_widget_mut(view_id::popup_yaml_kind);
                    match res {
                        Ok(vec) => {
                            let items = vec
                                .into_iter()
                                .map(|key| {
                                    let Ok(json) = serde_json::to_string(&key) else {
                                        unreachable!()
                                    };

                                    let metadata = BTreeMap::from([("key".into(), json)]);

                                    let item = if key.is_api() || key.is_preferred_version() {
                                        key.to_string()
                                    } else {
                                        format!("\x1b[90m{}\x1b[39m", key)
                                    };

                                    LiteralItem::new(item, Some(metadata))
                                })
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                        Err(e) => {
                            widget.update_widget_item(Item::Array(error_lines!(e)));
                        }
                    }
                }

                Resource(res) => match res {
                    Ok(list) => {
                        if list.items.is_empty() {
                            window.open_popup(view_id::popup_yaml_return);
                        } else {
                            window.open_popup(view_id::popup_yaml_name);

                            let widget = window.find_widget_mut(view_id::popup_yaml_name);

                            let items = list
                                .items
                                .into_iter()
                                .map(
                                    |YamlResourceListItem {
                                         namespace,
                                         name,
                                         kind,
                                         value,
                                     }| {
                                        let Ok(json) = serde_json::to_string(&kind) else {
                                            unreachable!()
                                        };

                                        let metadata = BTreeMap::from([
                                            ("namespace".to_string(), namespace),
                                            ("name".to_string(), name),
                                            ("key".into(), json),
                                        ]);

                                        LiteralItem {
                                            metadata: Some(metadata),
                                            item: value,
                                        }
                                    },
                                )
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                    }
                    Err(e) => {
                        let widget = window.find_widget_mut(view_id::popup_yaml_name);
                        widget.update_widget_item(Item::Array(error_lines!(e)));
                    }
                },
                SelectedYaml(res) => {
                    update_widget_item_for_vec(window, view_id::tab_yaml_widget_yaml, res);
                }
                DirectedYaml { kind, name, yaml } => {
                    let widget = window
                        .find_widget_mut(view_id::popup_yaml)
                        .widget_config_mut();
                    *(widget.append_title_mut()) = Some(format!(" : {}/{}", kind, name).into());

                    update_widget_item_for_vec(window, view_id::popup_yaml, yaml);
                }
            }
        }

        Kube::Network(NetworkMessage::Response(ev)) => {
            use NetworkResponse::*;

            match ev {
                List(res) => {
                    update_widget_item_for_table(window, view_id::tab_network_widget_network, res)
                }
                Yaml(res) => {
                    update_widget_item_for_vec(
                        window,
                        view_id::tab_network_widget_description,
                        res,
                    );
                }
            }
        }

        _ => unreachable!(),
    }
}
