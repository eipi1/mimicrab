use crate::models::Expectation;
use arc_swap::ArcSwap;
use futures::stream::StreamExt;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::runtime::{WatchStreamExt, watcher};
use kube::{
    Client,
    api::{Api, Patch, PatchParams},
};
use serde_json::json;
use std::sync::Arc;

pub async fn load_from_configmap(
    client: &Client,
    name: &str,
    ns: &str,
) -> Result<Vec<Expectation>, Box<dyn std::error::Error + Send + Sync>> {
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
    let cm = cms.get(name).await?;
    if let Some(data) = cm.data {
        if let Some(mocks_json) = data.get("mocks.json") {
            return Ok(serde_json::from_str(mocks_json)?);
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

    tracing::info!(
        "Started ConfigMap watcher for {} in namespace {}",
        config_map_name,
        namespace
    );

    while let Some(cm_res) = w.next().await {
        if let Ok(cm) = cm_res {
            if let Some(data) = cm.data {
                if let Some(mocks_json) = data.get("mocks.json") {
                    if let Ok(new_expectations) =
                        serde_json::from_str::<Vec<Expectation>>(mocks_json)
                    {
                        expectations.store(Arc::new(new_expectations));
                        tracing::info!("State synchronized from ConfigMap (JSON)");
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
    let mocks_json = serde_json::to_string(mocks).unwrap();

    let patch = json!({
        "data": {
            "mocks.json": mocks_json
        }
    });

    let pp = PatchParams::apply("mimicrab");
    if let Err(e) = cms.patch(config_map_name, &pp, &Patch::Merge(&patch)).await {
        tracing::error!("Failed to patch ConfigMap: {}", e);
    }
}
