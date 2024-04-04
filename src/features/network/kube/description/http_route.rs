use anyhow::{Context as _, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::{
    features::api_resources::kube::{ApiResource, ApiResources, SharedApiResources},
    kube::{
        apis::networking::gateway::v1::{
            Gateway, HTTPRoute, HTTPRouteSpec, HTTPRouteStatus, ParentReference, RouteParentStatus,
        },
        KubeClientRequest,
    },
};

use super::{Fetch, FetchedData};

pub(super) struct HTTPRouteDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for HTTPRouteDescriptionWorker<'a, C>
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
        let api = Api::<HTTPRoute>::namespaced(self.client.client().clone(), &self.namespace);

        let http_route = api.get(&self.name).await.context(format!(
            "Failed to fetch HTTPRoute: namespace={}, name={}",
            self.namespace, self.name
        ))?;

        let description = Description::new(http_route.clone());

        let yaml = serde_yaml::to_string(&description)?
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        Ok(yaml)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Description {
    http_route: DescriptionHTTPRoute,
}

impl Description {
    fn new(http_route: HTTPRoute) -> Self {
        Self {
            http_route: DescriptionHTTPRoute::new(http_route),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataName {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DescriptionHTTPRoute {
    metadata: MetadataName,

    spec: HTTPRouteSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<HTTPRouteStatusWrapper>,
}

impl DescriptionHTTPRoute {
    fn new(http_route: HTTPRoute) -> Self {
        let name = http_route.name_any();

        let HTTPRoute {
            metadata: _,
            spec,
            status,
        } = http_route;

        let status_wrapper = status.map(HTTPRouteStatusWrapper::new);

        Self {
            metadata: MetadataName { name },
            spec,
            status: status_wrapper,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HTTPRouteStatusWrapper {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parents: Vec<RouteParentStatusWrapper>,
}

impl HTTPRouteStatusWrapper {
    fn new(status: HTTPRouteStatus) -> Self {
        let parents = status
            .parents
            .into_iter()
            .map(RouteParentStatusWrapper::new)
            .collect();

        Self { parents }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteParentStatusWrapper {
    #[serde(flatten)]
    #[serde(with = "RouteParentStatusDef")]
    status: RouteParentStatus,
}

impl RouteParentStatusWrapper {
    fn new(status: RouteParentStatus) -> Self {
        Self { status }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RouteParentStatus")]
#[serde(rename_all = "camelCase")]
struct RouteParentStatusDef {
    #[serde(skip)]
    conditions: Option<Vec<Condition>>,

    controller_name: String,

    parent_ref: ParentReference,
}

fn fetch_related_resources() -> Result<Vec<String>> {
    todo!()
}

fn fetch_gateways(client: &Client, http_route: &HTTPRoute) -> Result<Vec<Gateway>> {
    // collect all gateways that reference the HTTPRoute
    let Some(parent_refs) = http_route.spec.parent_refs.as_ref() else {
        return Ok(vec![]);
    };

    let gateway_refs: Vec<(String, String)> = parent_refs
        .iter()
        .map(|parent_ref| {
            let mut namespace = http_route
                .namespace()
                .unwrap_or_else(|| "default".to_string());

            if let Some(parent_namespace) = &parent_ref.namespace {
                namespace = parent_namespace.clone();
            }

            (namespace, parent_ref.name.clone())
        })
        .collect();

    todo!()
}

fn fetch_backends() -> () {}

fn fetch_pods() -> () {}

// fn compare_parent_ref_section_name(section_name: Option<&str>, gateway: &Gateway) -> bool {
//     section_name.unwrap_or(gateway.name_any().as_str()) == gateway.name_any()
// }

/// groupとkindが一致するAPIリソースを取得する
///   * 一致するリソースが複数ある場合は、preferredVersionを優先して取得する
///   * preferredVersionがない場合は、最初に見つかったリソースを取得する
///   * 一致するリソースがない場合はNoneを返す
fn find_api_resource<'a>(
    api_resources: &'a ApiResources,
    group: &str,
    kind: &str,
) -> Option<&'a ApiResource> {
    let mut apis_for_find = api_resources
        .iter()
        .filter(|api| api.group() == group && api.name() == kind);

    let mut apis_for_first = apis_for_find.clone();

    if let Some(api) = apis_for_find.find(|api| api.is_preferred_version()) {
        Some(api)
    } else {
        apis_for_first.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod find_api_resource {
        use super::*;

        use pretty_assertions::assert_eq;

        #[test]
        fn with_preferred_version() {
            let api_resources = ApiResources::from([
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v1".to_string(),
                    preferred_version: false,
                    namespaced: true,
                },
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v2".to_string(),
                    preferred_version: true,
                    namespaced: true,
                },
            ]);

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            let expected = ApiResource::Apis {
                group: "group1".to_string(),
                name: "kind1".to_string(),
                version: "v2".to_string(),
                preferred_version: true,
                namespaced: true,
            };

            assert_eq!(actual, Some(&expected));
        }

        #[test]
        fn without_preferred_version() {
            let api_resources = ApiResources::from([
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v2".to_string(),
                    preferred_version: false,
                    namespaced: true,
                },
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v1".to_string(),
                    preferred_version: false,
                    namespaced: true,
                },
            ]);

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            let expected = ApiResource::Apis {
                group: "group1".to_string(),
                name: "kind1".to_string(),
                version: "v2".to_string(),
                preferred_version: false,
                namespaced: true,
            };

            assert_eq!(actual, Some(&expected));
        }

        #[test]
        fn no_matching_resources() {
            let api_resources = ApiResources::default();

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            assert_eq!(actual, None);
        }
    }
}
