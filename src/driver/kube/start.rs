use super::*;
use k8s_openapi::api::core::v1::Pod;
use tracing::info;

impl<T> KubeHandler<T>
where
    T: Repo,
{
    pub async fn inner_start(&self) -> Result<()> {
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let pods = pods_api.list(&ListParams::default()).await?;

        for pod in pods {
            let pod_name = pod.metadata.name.as_ref().unwrap();
            if let Some(phase) = pod.status.unwrap().phase.as_ref() {
                if phase != "Running" {
                    pods_api.delete(pod_name, &Default::default()).await?;
                    info!("Pod {} is not running. It will be restarted.", pod_name);
                }
            } else {
                pods_api.delete(pod_name, &Default::default()).await?;
                info!("Pod {} has no phase info. It will be restarted.", pod_name);
            }
        }

        Ok(())
    }
}
