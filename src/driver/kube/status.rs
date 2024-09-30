use super::*;
use k8s_openapi::api::core::v1::Pod;

impl<T> KubeHandler<T>
where
    T: Repo,
{
    pub async fn inner_status(&self) -> Result<NodeStatus> {
        let statefulset_api: Api<StatefulSet> =
            Api::namespaced(self.client.clone(), &self.namespace);
        let claim_api: Api<PersistentVolumeClaim> =
            Api::namespaced(self.client.clone(), &self.namespace);
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let metrics_api: Api<metricsv1::PodMetrics> =
            Api::<metricsv1::PodMetrics>::namespaced(self.client.clone(), &self.namespace);

        let statefulset = statefulset_api.get(&self.stateset_name).await.anyhow()?;
        let selector = statefulset
            .spec
            .as_ref()
            .unwrap()
            .selector
            .match_labels
            .as_ref()
            .expect("set in template")
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join(",");

        let list_params = ListParams::default().labels(&selector);
        let pods = pods_api.list(&list_params).await.anyhow()?;

        let pvc = claim_api.get(&self.claim_name).await.anyhow()?;
        let cap = pvc
            .status
            .unwrap()
            .capacity
            .unwrap()
            .get("storage")
            .map(|cap| cap.0.clone())
            .unwrap_or_default();

        let db_node = self.db_repo.get_node_by_name(&self.node_name).await?;
        let data_count = self
            .db_repo
            .count(&self.node_name, &Vec::new(), None)
            .await?;
        let mut node_status = NodeStatus {
            name: self.node_name.clone(),
            state: db_node.state,
            data_count,
            replicas: statefulset
                .spec
                .as_ref()
                .and_then(|spec| spec.replicas)
                .unwrap_or_default() as u32,
            storage: cap,
            pods: HashMap::new(),
        };

        for pod in pods {
            let pod_name = pod.metadata.name.as_ref().expect("set in template");
            let phase = get_pod_status(&pod);

            let metrics = metrics_api
                .get(pod_name)
                .await
                .anyhow()
                .map_err(|err| {
                    warn!("get metrics fail {err} {}", pod_name);
                    err
                })
                .unwrap_or_default();

            let mut cpu_sum = 0.0;
            let mut memory_sum = 0;
            for container in metrics.containers.iter() {
                cpu_sum += container
                    .usage
                    .cpu()
                    .map_err(|err| {
                        error!("cpu not exit {err}");
                        err
                    })
                    .unwrap_or(0.0);
                memory_sum += container
                    .usage
                    .memory()
                    .map_err(|err| {
                        error!("cpu not exit {err}");
                        err
                    })
                    .unwrap_or(0);
            }
            let pod_status = PodStauts {
                state: phase,
                cpu_usage: cpu_sum,
                memory_usage: memory_sum,
            };
            node_status.pods.insert(pod_name.clone(), pod_status);
        }
        Ok(node_status)
    }
}
