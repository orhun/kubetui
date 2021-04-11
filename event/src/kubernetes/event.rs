use super::{Event, Kube};
use crate::kubernetes::Handlers;
use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use futures::{StreamExt, TryStreamExt};

use std::sync::Arc;
use std::time;

use tokio::sync::RwLock;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Event as KEvent;

use kube::{
    api::{ListParams, Meta},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

pub async fn event_loop(tx: Sender<Event>, client: Client, namespace: Arc<RwLock<String>>) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let ns = namespace.read().await;

        let event_list = get_event_list(client.clone(), &ns).await;

        tx.send(Event::Kube(Kube::Event(event_list))).unwrap();
    }
}

async fn get_event_list(client: Client, ns: &str) -> Vec<String> {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default();

    let list = events.list(&lp).await.unwrap();

    let current_datetime: DateTime<Utc> = Utc::now();

    list.iter()
        .map(|ev| {
            let meta = Meta::meta(ev);

            let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
                Some(time) => time.0,
                None => current_datetime,
            };
            let duration: Duration = current_datetime - creation_timestamp;

            let obj = &ev.involved_object;

            let name = obj.name.as_ref().unwrap();
            let kind = obj.kind.as_ref().unwrap();
            let message = ev.message.as_ref().unwrap();
            let reason = ev.reason.as_ref().unwrap();
            format!(
                "{} {}  {} {}\n\x1b[90m> {}\x1b[0m\n ",
                kind,
                name,
                reason,
                age(duration),
                message
            )
        })
        .collect()
}

#[allow(dead_code)]
async fn watch(tx: Sender<Event>, client: Client, ns: String) -> Handlers {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default();

    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let watch_handle = tokio::spawn(async move {
        // タイムアウト時に再接続を試みる
        let current_datetime: DateTime<Utc> = Utc::now();
        let mut ew = try_flatten_applied(watcher(events, lp)).boxed();
        while let Some(event) = ew.try_next().await.unwrap() {
            let mut buf = buf_clone.write().await;

            let meta = Meta::meta(&event);
            let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
                Some(ref time) => time.0,
                None => current_datetime,
            };
            let duration: Duration = current_datetime - creation_timestamp;

            buf.push(format!("{:4} {}", age(duration), event.message.unwrap()));
        }
    });

    let buf_clone = Arc::clone(&buf);
    let event_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().await;
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::Event(buf.clone()))).unwrap();

                buf.clear();
            }
        }
    });

    Handlers(vec![watch_handle, event_handle])
}

#[allow(dead_code)]
pub async fn event_watch(
    tx: Sender<Event>,
    client: Client,
    ns: String,
    object_name: impl Into<String>,
    kind: impl Into<String>,
) -> Handlers {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default().fields(&format!(
        "involvedObject.kind={},involvedObject.name={}",
        kind.into(),
        object_name.into()
    ));

    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let watch_handle = tokio::spawn(async move {
        let mut ew = try_flatten_applied(watcher(events, lp)).boxed();
        while let Some(event) = ew.try_next().await.unwrap() {
            let mut buf = buf_clone.write().await;
            buf.push(format!(
                "{} {} {}",
                event.type_.unwrap(),
                event.reason.unwrap(),
                event.message.unwrap()
            ));
        }
    });

    let buf_clone = Arc::clone(&buf);
    let event_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().await;
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
                    .unwrap();

                buf.clear();
            }
        }
    });

    Handlers(vec![watch_handle, event_handle])
}
