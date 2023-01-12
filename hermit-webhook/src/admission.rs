use std::collections::BTreeMap;

use actix_web::{get, http, post, web, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use k8s_openapi::{
    api::core::v1::{ConfigMap, Container, Pod, Volume},
    Metadata,
};
use kube::{
    api::{Api, Patch, PatchParams},
    core::{
        admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
        DynamicObject, ObjectMeta,
    },
    Client, Config, ResourceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

const HERMIT_CONFIG_KEY: &str = "hermit.jan0ski.net/config";

#[derive(Serialize, Deserialize, Debug)]
struct HermitConfig {
    #[serde(skip)]
    context_id: u32,
    blocked_ips: Vec<String>,
    blocked_user_agents: Vec<String>,
}

#[get("/healthz")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .json(json!({"message": "ok"}))
}

#[post("/mutate")]
pub async fn handle_mutate(
    reqst: HttpRequest,
    body: web::Json<AdmissionReview<DynamicObject>>,
) -> impl Responder {
    info!(
        "request recieved: method={:?}, uri={}",
        reqst.method(),
        reqst.uri(),
    );

    let req: AdmissionRequest<_> = match body.into_inner().try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return HttpResponse::InternalServerError()
                .json(&AdmissionResponse::invalid(err.to_string()).into_review());
        }
    };

    let mut res = AdmissionResponse::from(&req);

    if let Some(obj) = req.object {
        let name = obj.name_any();
        info!("injecting envoy sidecar in pod {}", name);
        res = match inject_sidecar(res.clone(), &obj).await {
            Ok(res) => {
                info!("accepted: {:?} on Pod {}", req.operation, name);
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {} ({})", req.operation, name, err);
                res.deny(err.to_string())
            }
        };
    };

    HttpResponse::Ok().json(&res.into_review())
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
    Ok(())
}

#[cfg(test)]
mod test {}
