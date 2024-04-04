mod description;
mod related_resource;

use anyhow::{Context as _, Result};
use kube::Api;

use crate::{
    features::api_resources::kube::SharedApiResources,
    kube::{apis::networking::gateway::v1::Gateway, KubeClientRequest},
};

use self::{description::Description, related_resource::discover_releated_resources};

use super::{Fetch, FetchedData};

pub(super) struct GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String, _: SharedApiResources) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let api = Api::<Gateway>::namespaced(self.client.client().clone(), &self.namespace);

        let gateway = api.get(&self.name).await.context(format!(
            "Failed to fetch Gateway: namespace={}, name={}",
            self.namespace, self.name
        ))?;

        let description = Description::new(gateway.clone());

        let mut related_resources = discover_releated_resources(
            self.client.client().clone(),
            &self.name,
            &self.namespace,
            &gateway,
        )
        .await?;

        let mut yaml = serde_yaml::to_string(&description)?
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        if !related_resources.is_empty() {
            yaml.push("".into());

            yaml.append(&mut related_resources);
        }

        Ok(yaml)
    }
}
