use std::collections::BTreeMap;

use anyhow::Result;
use k8s_openapi::{
    api::core::v1::{ConfigMap, Container, Pod, Volume},
    Metadata,
};
use kube::{
    api::{Api, Patch, PatchParams},
    core::{admission::AdmissionResponse, DynamicObject, ObjectMeta},
    Client, Config, ResourceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

const HERMIT_CONFIG_KEY: &str = "hermit.jan0ski.net/config";

#[derive(Serialize, Deserialize, Debug)]
struct HermitConfig {
    #[serde(skip)]
    context_id: u32,
    blocked_ips: Vec<String>,
    blocked_user_agents: Vec<String>,
}

async fn get_config(name: &str, ns: &str) -> Result<HermitConfig> {
    let kube_client = Client::try_default().await?;
    let configs: Api<ConfigMap> = Api::namespaced(kube_client, ns);
    let data = configs.get(name).await?.data.unwrap();
    let config = data.get("hermit.json").unwrap().as_str();
    serde_json::from_str(config).map_err(|err| anyhow::Error::new(err))
}

fn get_containers(obj: &DynamicObject) -> Vec<Container> {
    serde_json::from_value::<Pod>(obj.to_owned().data)
        .unwrap()
        .spec
        .unwrap()
        .containers
}

fn get_volumes(obj: &DynamicObject) -> Vec<Volume> {
    serde_json::from_value::<Pod>(obj.to_owned().data)
        .unwrap()
        .spec
        .unwrap()
        .volumes
        .unwrap()
}

pub async fn inject_sidecar(
    res: AdmissionResponse,
    obj: &DynamicObject,
) -> Result<AdmissionResponse> {
    let mut patches = Vec::new();

    let config_volume: Volume = serde_json::from_value(json!({
        "name": "envoy-config",
        "configMap": {
            "name": "envoy-config"
        }
    }))?;
    let envoy: Container = serde_json::from_value(json!({
        "name": "hermit",
        "image": "envoyproxy/envoy:v1.24-latest",
        "volumeMounts": [
            {
                "name": config_volume.name,
                "mountPath": "/etc/envoy.yaml",
            },
        ]
    }))?;

    // Inject envoy sidecar if config annotation is present
    if obj.annotations().contains_key(HERMIT_CONFIG_KEY) {
        // Get Hermit config from configmap
        let ns = obj.namespace().unwrap_or("default".to_string()).clone();
        let config = get_config(
            obj.annotations()
                .get(HERMIT_CONFIG_KEY)
                .unwrap_or(&String::from("hermit-config")),
            ns.as_str(),
        )
        .await?;

        // Create configmap for envoy based on hermit config
        create_envoy_config(&config, ns).await?;

        // Add envoy sidecar
        patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
            path: "/spec/containers".into(),
            value: serde_json::json!([get_containers(obj), envoy]),
        }));

        // Add envoy and configmap volumes
        patches.push(json_patch::PatchOperation::Add(json_patch::AddOperation {
            path: "/spec/volumes".into(),
            value: serde_json::json!([get_volumes(obj), config_volume]),
        }));
    }
    Ok(res.with_patch(json_patch::Patch(patches))?)
}

async fn create_envoy_config(_config: &HermitConfig, ns: String) -> Result<()> {
    // TODO: Dynamically create envoy filters based on passed in hermit config
    let mut envoy_config = BTreeMap::new();
    envoy_config.insert("envoy.yaml".to_string(), r#"
        static_resources:
          listeners:
            - name: main
              address:
                socket_address:
                  address: 0.0.0.0
                  port_value: 80
              filter_chains:
                - filters:
                    - name: envoy.filters.network.wasm
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.filters.network.wasm.v3.Wasm
                        config:
                          name: "hermit"
                          root_id: "hermit"
                          configuration:
                            "@type": type.googleapis.com/google.protobuf.StringValue
                            value: |
                              { 
                                "blocked_ips": [
                                  "172.18.0.1",
                                  "172.19.0.1", 
                                  "172.20.0.1" 
                                ]
                                "blocked_user_agents": [
                                  "Fake User Agent",
                                  "curl/7.81.0"
                                ]
                              }
                          vm_config:
                            runtime: envoy.wasm.runtime.v8
                            code:
                              local:
                                filename: "/etc/hermit.wasm"
                            allow_precompiled: true
                    - name: envoy.tcp_proxy
                      typed_config:
                        "@type": type.googleapis.com/envoy.extensions.filters.network.tcp_proxy.v3.TcpProxy
                        stat_prefix: ingress_tcp
                        cluster: web_service
          clusters:
            - name: web_service
              connect_timeout: 0.25s
              type: STRICT_DNS
              lb_policy: round_robin
              load_assignment:
                cluster_name: web_service
                endpoints:
                  - lb_endpoints:
                      - endpoint:
                          address:
                            socket_address:
                              address: web_service
                              port_value: 5678
        admin:
          access_log_path: "/dev/null"
          address:
            socket_address:
              address: 0.0.0.0
              port_value: 8001
    "#.to_ascii_lowercase().to_string());

    // Construct configmap holding envoy config data
    let kube_client = Client::try_default().await?;
    let client: Api<ConfigMap> = Api::namespaced(kube_client, ns.as_str());
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some("envoy-config".to_string()),
            ..ObjectMeta::default()
        },
        data: Some(envoy_config),
        ..Default::default()
    };
    info!("creating envoy configmap {:?}", serde_yaml::to_string(&cm));

    // Create configmap in workload's namespace
    client
        .patch(
            cm.metadata().name.as_ref().unwrap(),
            &PatchParams::apply("hermit"),
            &Patch::Apply(&cm),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn test_inject() -> Result<()> {
        let pod: Pod = serde_json::from_value(serde_json::json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "test-pod",
            },
            "spec": {
                "containers": [{
                    "name": "empty",
                    "image": "alpine:latest"
                }],
                "restartPolicy": "Never",
            }
        }))?;
        Ok(())
    }
}
