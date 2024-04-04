use std::collections::BTreeSet;

use anyhow::Result;
use derivative::Derivative;
use k8s_openapi::{api::core::v1::Service, Resource};
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::{kube::apis::networking::gateway::v1::HTTPBackendRef, logger};

use super::{http_route::RelatedHTTPRoute, ExtractNamespace};

pub type RelatedServices = Vec<RelatedService>;

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, Eq, PartialOrd, Ord)]
pub struct RelatedService {
    /// Service Name
    pub name: String,

    /// Service Namespace
    pub namespace: String,

    // TODO: 名前を変えたい
    /// HTTPRoute Name
    pub httproute_name: String,

    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    #[serde(skip)]
    pub resource: Service,
}

struct BackendRefs<'a> {
    httproute_name: &'a str,
    httproute_namespace: &'a str,
    refs: Vec<&'a HTTPBackendRef>,
}

impl<'a> From<&'a RelatedHTTPRoute> for Option<BackendRefs<'a>> {
    fn from(value: &'a RelatedHTTPRoute) -> Self {
        let rules = value.resource.spec.rules.as_ref();

        let refs: Vec<&HTTPBackendRef> = rules.map(|rules| {
            rules
                .iter()
                .filter_map(|rule| rule.backend_refs.as_ref())
                .flat_map(|backend_refs| backend_refs.iter())
                .collect::<Vec<_>>()
        })?;

        Some(BackendRefs {
            httproute_name: value.name.as_ref(),
            httproute_namespace: value.namespace.as_ref(),
            refs,
        })
    }
}

pub async fn discover_services(
    client: Client,
    httproutes: &[RelatedHTTPRoute],
) -> Result<Option<RelatedServices>> {
    let backend_refs: Vec<BackendRefs> = httproutes.iter().filter_map(Option::from).collect();

    let mut result: BTreeSet<RelatedService> = BTreeSet::new();

    for BackendRefs {
        httproute_name,
        httproute_namespace,
        refs,
    } in backend_refs
    {
        for r in refs {
            if r.group.as_ref().is_some_and(|g| g != "")
                || r.kind.as_ref().is_some_and(|k| k != Service::KIND)
            {
                continue;
            }

            let namespace = if let Some(namespace) = r.namespace.as_ref() {
                namespace
            } else {
                httproute_namespace
            };

            let api = Api::<Service>::namespaced(client.clone(), &namespace);

            let Ok(service) = api.get(&r.name).await else {
                logger!(error, "failed to get service {namespace}/{{r.name}}");
                continue;
            };

            result.insert(RelatedService {
                name: service.name_any(),
                namespace: service.extract_namespace(),
                httproute_name: httproute_name.to_string(),
                resource: service,
            });
        }
    }

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result.into_iter().collect()))
    }
}
