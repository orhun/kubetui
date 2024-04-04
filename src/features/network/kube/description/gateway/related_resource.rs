mod http_route;
mod pod;
mod service;

use std::collections::BTreeMap;

use anyhow::{Context as _, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use kube::{Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::kube::apis::networking::gateway::v1::Gateway;

use self::{
    http_route::{discover_httproutes, RelatedHTTPRoutes},
    pod::{discover_pods, RelatedPods},
    service::{discover_services, RelatedServices},
};

trait ExtractNamespace {
    fn extract_namespace(&self) -> String;
}

impl<K> ExtractNamespace for K
where
    K: ResourceExt,
{
    fn extract_namespace(&self) -> String {
        self.namespace().unwrap_or_else(|| "default".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayRelatedResources {
    related_resources: GatewayRelatedResourceItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayRelatedResourceItems {
    #[serde(skip_serializing_if = "Option::is_none")]
    httproutes: Option<RelatedHTTPRoutes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<RelatedServices>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pods: Option<RelatedPods>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelatedResource {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
}

pub async fn discover_releated_resources(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
    gateway: &Gateway,
) -> Result<Vec<String>> {
    let httproutes = discover_httproutes(client.clone(), gateway_name, gateway_namespace, gateway)
        .await
        .with_context(|| "discover httproutes for gateway")?;

    let services = if let Some(httproutes) = httproutes.as_ref() {
        discover_services(client.clone(), httproutes)
            .await
            .with_context(|| "discover services for gateway")?
    } else {
        None
    };

    let pods = if let Some(services) = services.as_ref() {
        discover_pods(client.clone(), services)
            .await
            .with_context(|| "discover pods for gateway")?
    } else {
        None
    };

    let related_resources = GatewayRelatedResources {
        related_resources: GatewayRelatedResourceItems {
            httproutes,
            services,
            pods,
        },
    };

    let yaml = serde_yaml::to_string(&related_resources)?
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    Ok(yaml)
}

fn label_selector_to_query(selector: Option<LabelSelector>) -> String {
    let Some(LabelSelector {
        match_labels,
        match_expressions,
    }) = selector
    else {
        return "".into();
    };

    let mut query = Vec::new();

    if let Some(match_labels) = match_labels {
        query.append(&mut match_labels_to_query(match_labels));
    }

    if let Some(match_expressions) = match_expressions {
        query.append(&mut match_expressions_to_query(match_expressions));
    }

    query.join(",")
}

/// matchLabelsをクエリパラメーターに変換する
fn match_labels_to_query(match_labels: BTreeMap<String, String>) -> Vec<String> {
    match_labels
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
}

/// matchExpressionsをクエリパラメーターに変換する
fn match_expressions_to_query(match_expressions: Vec<LabelSelectorRequirement>) -> Vec<String> {
    match_expressions
        .into_iter()
        .map(|requirement| {
            let LabelSelectorRequirement {
                key,
                operator,
                values,
            } = requirement;

            // InとNotInのとき、valuesはかならずSomeである
            match operator.as_str() {
                "In" => {
                    format!(
                        "{} in ({})",
                        key,
                        values.map(|values| values.join(", ")).unwrap_or_default()
                    )
                }
                "NotIn" => {
                    format!(
                        "{} notin ({})",
                        key,
                        values.map(|values| values.join(", ")).unwrap_or_default()
                    )
                }
                "Exists" => {
                    format!("{}", key)
                }
                "DoesNotExist" => {
                    format!("!{}", key)
                }
                _ => {
                    unreachable!()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod match_labels_to_query {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn empty() {
            let match_labels: BTreeMap<String, String> = BTreeMap::new();
            let result = super::match_labels_to_query(match_labels);
            assert!(
                result.is_empty(),
                "Result should be empty for an empty input"
            );
        }

        #[test]
        fn single() {
            let mut match_labels: BTreeMap<String, String> = BTreeMap::new();
            match_labels.insert("key".to_string(), "value".to_string());
            let result = match_labels_to_query(match_labels);
            assert_eq!(
                result,
                vec!["key=value"],
                "Result should contain one key-value pair"
            );
        }

        #[test]
        fn multiple() {
            let mut match_labels: BTreeMap<String, String> = BTreeMap::new();
            match_labels.insert("key1".to_string(), "value1".to_string());
            match_labels.insert("key2".to_string(), "value2".to_string());
            let result = match_labels_to_query(match_labels);
            assert_eq!(
                result,
                vec!["key1=value1", "key2=value2"],
                "Result should contain two key-value pairs"
            );
        }
    }

    mod match_expressions_to_query {
        use super::*;

        #[test]
        fn empty() {
            let match_expressions: Vec<LabelSelectorRequirement> = Vec::new();
            let result = match_expressions_to_query(match_expressions);
            assert!(
                result.is_empty(),
                "Result should be empty for an empty input"
            );
        }

        #[test]
        fn in_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "In".to_string(),
                values: Some(vec!["value1".to_string(), "value2".to_string()]),
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key in (value1, value2)"],
                "Result should contain one 'In' expression"
            );
        }

        #[test]
        fn not_in_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "NotIn".to_string(),
                values: Some(vec!["value1".to_string(), "value2".to_string()]),
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key notin (value1, value2)"],
                "Result should contain one 'NotIn' expression"
            );
        }

        #[test]
        fn exists_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "Exists".to_string(),
                values: None,
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key"],
                "Result should contain one 'Exists' expression"
            );
        }

        #[test]
        fn does_not_exist_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "DoesNotExist".to_string(),
                values: None,
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["!key"],
                "Result should contain one 'DoesNotExist' expression"
            );
        }
    }
}
