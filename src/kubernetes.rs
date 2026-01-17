use crate::models::Expectation;
use kube::{
    api::{Api, Patch, PatchParams},
    Client,
};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::runtime::{watcher, WatchStreamExt};
use futures::stream::StreamExt;
use std::sync::Arc;
use arc_swap::ArcSwap;
use serde_json::json;

pub async fn load_from_configmap(client: &Client, name: &str, ns: &str) -> Result<Vec<Expectation>, Box<dyn std::error::Error + Send + Sync>> {
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
    let cm = cms.get(name).await?;
    if let Some(data) = cm.data {
        if let Some(mocks_yaml) = data.get("mocks.yaml") {
            return Ok(serde_yaml::from_str(mocks_yaml)?);
        }
    }
    Ok(vec![])
}

pub async fn run_configmap_watcher(
    client: Client,
    namespace: String,
    config_map_name: String,
    expectations: Arc<ArcSwap<Vec<Expectation>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    
    let wc = watcher::Config::default().fields(&format!("metadata.name={}", config_map_name));
    let mut w = watcher(cms, wc).applied_objects().boxed();

    tracing::info!("Started ConfigMap watcher for {} in namespace {}", config_map_name, namespace);

    while let Some(cm_res) = w.next().await {
        if let Ok(cm) = cm_res {
            if let Some(data) = cm.data {
                if let Some(mocks_yaml) = data.get("mocks.yaml") {
                    if let Ok(new_expectations) = serde_yaml::from_str::<Vec<Expectation>>(mocks_yaml) {
                        expectations.store(Arc::new(new_expectations));
                        tracing::info!("State synchronized from ConfigMap (YAML)");
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn sync_to_configmap(
    client: &Client,
    namespace: &str,
    config_map_name: &str,
    mocks: &[Expectation],
) {
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let mocks_yaml = serde_yaml::to_string(mocks).unwrap();
    
    let patch = json!({
        "data": {
            "mocks.yaml": mocks_yaml
        }
    });

    let pp = PatchParams::apply("mimicrab");
    if let Err(e) = cms.patch(config_map_name, &pp, &Patch::Merge(&patch)).await {
        tracing::error!("Failed to patch ConfigMap: {}", e);
    }
}
