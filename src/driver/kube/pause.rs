use super::*;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{
    Api,
    ListParams,
    Patch,
    PatchParams,
};
use serde_json::json;
use tracing::info;

impl<T> KubeHandler<T>
where
    T: Repo,
{
    pub async fn inner_pause(&mut self) -> anyhow::Result<()> {
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let pods = pods_api.list(&ListParams::default()).await?;

        for pod in pods {
            let pod_name = pod.metadata.name.as_ref().expect("Pod name is set");

            let patch = Patch::Apply(json!({
                "spec": {
                    "unschedulable": true,
                },
            }));

            let patch_params = PatchParams::default();
            pods_api.patch(pod_name, &patch_params, &patch).await?;

            info!("Pod {} is paused.", pod_name);
        }
        Ok(())
    }
}
