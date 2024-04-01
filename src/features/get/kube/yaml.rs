use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use crossbeam::channel::Sender;
use k8s_openapi::{
    api::{
        core::v1::{ConfigMap, Pod, Secret, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    NamespaceResourceScope, Resource as _,
};
use kube::{Api, Resource};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    features::get::message::{GetRequest, GetResponse},
    kube::{apis::networking::gateway::v1::Gateway, KubeClient},
    logger,
    message::Message,
    workers::kube::AbortWorker,
};

#[derive(Debug, Clone)]
pub enum GetYamlKind {
    Pod,
    ConfigMap,
    Secret,
    Ingress,
    Service,
    NetworkPolicy,
    Gateway,
}

impl std::fmt::Display for GetYamlKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pod => write!(f, "{}", Pod::URL_PATH_SEGMENT),
            Self::ConfigMap => write!(f, "{}", ConfigMap::URL_PATH_SEGMENT),
            Self::Secret => write!(f, "{}", Secret::URL_PATH_SEGMENT),
            Self::Ingress => write!(f, "{}", Ingress::URL_PATH_SEGMENT),
            Self::Service => write!(f, "{}", Service::URL_PATH_SEGMENT),
            Self::NetworkPolicy => write!(f, "{}", NetworkPolicy::URL_PATH_SEGMENT),
            Self::Gateway => write!(f, "{}", Gateway::URL_PATH_SEGMENT),
        }
    }
}

#[derive(Clone)]
pub struct GetYamlWorker {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Message>,
    client: KubeClient,
    req: GetRequest,
}

impl GetYamlWorker {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Message>,
        client: KubeClient,
        req: GetRequest,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            client,
            req,
        }
    }
}

#[async_trait::async_trait]
impl AbortWorker for GetYamlWorker {
    async fn run(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

        let GetRequest {
            kind,
            name,
            namespace,
        } = &self.req;

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let yaml = match kind {
                GetYamlKind::Pod => fetch_resource_yaml::<Pod>(&self.client, name, namespace).await,
                GetYamlKind::ConfigMap => {
                    fetch_resource_yaml::<ConfigMap>(&self.client, name, namespace).await
                }
                GetYamlKind::Secret => {
                    fetch_resource_yaml::<Secret>(&self.client, name, namespace).await
                }
                GetYamlKind::Ingress => {
                    fetch_resource_yaml::<Ingress>(&self.client, name, namespace).await
                }
                GetYamlKind::Service => {
                    fetch_resource_yaml::<Service>(&self.client, name, namespace).await
                }
                GetYamlKind::NetworkPolicy => {
                    fetch_resource_yaml::<NetworkPolicy>(&self.client, name, namespace).await
                }
                GetYamlKind::Gateway => {
                    fetch_resource_yaml::<Gateway>(&self.client, name, namespace).await
                }
            };

            self.tx
                .send(
                    GetResponse {
                        yaml,
                        kind: kind.to_string(),
                        name: name.to_string(),
                    }
                    .into(),
                )
                .expect("Failed to send YamlResponse::Yaml");
        }
    }
}

/// 選択されているリソースのyamlを取得する
async fn fetch_resource_yaml<K>(client: &KubeClient, name: &str, ns: &str) -> Result<Vec<String>>
where
    K: Resource<Scope = NamespaceResourceScope> + k8s_openapi::Resource,
    <K as kube::Resource>::DynamicType: Default,
    K: DeserializeOwned + Clone + std::fmt::Debug,
    K: Serialize,
{
    logger!(
        info,
        "Fetching resource target [kind={} ns={} name={}]",
        K::KIND,
        ns,
        name
    );

    let api: Api<K> = Api::namespaced(client.to_client(), ns);

    let mut data = api.get(name).await?;

    let metadata = data.meta_mut();
    metadata.managed_fields = None;

    let yaml_string = serde_yaml::to_string(&data)?
        .lines()
        .map(ToString::to_string)
        .collect();

    Ok(yaml_string)
}
