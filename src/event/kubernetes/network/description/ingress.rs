use k8s_openapi::api::networking::v1::Ingress;

use crate::{error::Result, event::kubernetes::client::KubeClientRequest};

use super::{Fetch, FetchedData};

pub(super) struct IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: Ingress = serde_json::from_str(&res)?;

        value.metadata.managed_fields = None;

        let value = serde_yaml::to_string(&value)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use anyhow::bail;
    use indoc::indoc;
    use k8s_openapi::{
        api::{
            core::v1::{Pod, Service},
            networking::v1::Ingress,
        },
        List,
    };
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;

    use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

    use super::*;

    fn ingress() -> Ingress {
        serde_yaml::from_str(indoc! {
            r#"
            apiVersion: networking.k8s.io/v1
            kind: Ingress
            metadata:
              annotations:
                kubectl.kubernetes.io/last-applied-configuration: |
                  {"apiVersion":"networking.k8s.io/v1","kind":"Ingress","metadata":{"annotations":{},"name":"ingress","namespace":"kubetui"},"spec":{"rules":[{"host":"example-0.com","http":{"paths":[{"backend":{"service":{"name":"service-0","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}},{"host":"example-1.com","http":{"paths":[{"backend":{"service":{"name":"service-1","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}}],"tls":[{"hosts":["example.com"],"secretName":"secret-name"}]}}
              creationTimestamp: "2022-03-27T09:17:06Z"
              generation: 1
              name: ingress
              resourceVersion: "710"
              uid: 28a8cecd-8bbb-476f-8e34-eb86a8a8255f
            spec:
              rules:
              - host: example-0.com
                http:
                  paths:
                  - backend:
                      service:
                        name: service-1
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific
                  - backend:
                      service:
                        name: service-2
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific

              - host: example-1.com
                http:
                  paths:
                  - backend:
                      service:
                        name: service-3
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific
              tls:
                - hosts:
                  - example.com
                  secretName: secret-name
            status:
              loadBalancer: {}
            "#
        })
        .unwrap()
    }

    fn pods() -> List<Pod> {
        serde_yaml::from_str(indoc! {
            "
            items:
              - metadata:
                  name: pod-1
                  labels:
                    app: pod-1
                    version: v1
              - metadata:
                  name: pod-2
                  labels:
                    app: pod-2
                    version: v1
              - metadata:
                  name: pod-3
                  labels:
                    app: pod-3
                    version: v2
            "
        })
        .unwrap()
    }

    fn services() -> List<Service> {
        serde_yaml::from_str(indoc! {
            "
            items:
              - metadata:
                  name: service-1
                spec:
                  selector:
                    app: pod-1
                    version: v1
              - metadata:
                  name: service-2
                spec:
                   selector:
                    app: pod-2
                    version: v1
              - metadata:
                  name: service-3
                spec:
                   selector:
                    app: pod-3
                    version: v2
           "
        })
        .unwrap()
    }

    #[tokio::test]
    async fn yamlデータを返す() {
        let mut client = MockTestKubeClient::new();
        mock_expect!(
            client,
            request,
            [
                (
                    Ingress,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses/ingress"),
                    Ok(ingress())
                ),
                (
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(pods())
                )
            ]
        );

        let worker =
            IngressDescriptionWorker::new(&client, "default".to_string(), "ingress".to_string());

        let result = worker.fetch().await;

        let expected: Vec<String> = indoc! {
            "
            ingress:
              metadata:
                name: ingress
              spec:
                rules:
                  - host: example-0.com
                    http:
                      paths:
                        - backend:
                            service:
                              name: service-1
                              port:
                                number: 80
                          path: /path
                          pathType: ImplementationSpecific
                        - backend:
                            service:
                              name: service-2
                              port:
                                number: 80
                          path: /path
                          pathType: ImplementationSpecific
                  - host: example-1.com
                    http:
                      paths:
                        - backend:
                            service:
                              name: service-3
                              port:
                                number: 80
                          path: /path
                          pathType: ImplementationSpecific
                tls:
                  - hosts:
                      - example.com
                    secretName: secret-name
              status:
                loadBalancer: {}

            relatedResources:
              services:
                - service-1
                - service-2
                - service-3
              pods:
                - pod-1
                - pod-2
                - pod-3
            "
        }
        .lines()
        .map(ToString::to_string)
        .collect();

        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn エラーのときerrorを返す() {
        let mut client = MockTestKubeClient::new();
        mock_expect!(
            client,
            request,
            [
                (
                    Ingress,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses/test"),
                    bail!("error")
                ),
                (
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    bail!("error")
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    bail!("error")
                )

            ]
        );

        let worker =
            IngressDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        assert_eq!(result.is_err(), true);
    }
}

mod extract {
    use k8s_openapi::api::networking::v1::Ingress;
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for Ingress {
        fn extract(&self) -> Self {
            let annotations = if let Some(mut annotations) = self.metadata.annotations.clone() {
                annotations.remove("kubectl.kubernetes.io/last-applied-configuration");
                if annotations.is_empty() {
                    None
                } else {
                    Some(annotations)
                }
            } else {
                None
            };
            Ingress {
                metadata: ObjectMeta {
                    annotations,
                    labels: self.metadata.labels.clone(),
                    name: self.metadata.name.clone(),
                    ..Default::default()
                },
                spec: self.spec.clone(),
                status: self.status.clone(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        use super::*;

        fn ingress() -> Ingress {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  annotations:
                    kubectl.kubernetes.io/last-applied-configuration: |
                      {"apiVersion":"networking.k8s.io/v1","kind":"Ingress","metadata":{"annotations":{},"name":"ingress","namespace":"kubetui"},"spec":{"rules":[{"host":"example-0.com","http":{"paths":[{"backend":{"service":{"name":"service-0","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}},{"host":"example-1.com","http":{"paths":[{"backend":{"service":{"name":"service-1","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}}],"tls":[{"hosts":["example.com"],"secretName":"secret-name"}]}}
                  creationTimestamp: "2022-03-27T09:17:06Z"
                  generation: 1
                  name: ingress
                  namespace: kubetui
                  resourceVersion: "710"
                  uid: 28a8cecd-8bbb-476f-8e34-eb86a8a8255f
                spec:
                  rules:
                  - host: example-0.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-0
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  - host: example-1.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-1
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  tls:
                  - hosts:
                    - example.com
                    secretName: secret-name
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してserviceを返す() {
            let actual = ingress().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  annotations:
                  name: ingress
                spec:
                  rules:
                  - host: example-0.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-0
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  - host: example-1.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-1
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  tls:
                  - hosts:
                    - example.com
                    secretName: secret-name
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}
