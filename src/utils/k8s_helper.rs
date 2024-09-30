use std::env;

use k8s_openapi::api::core::v1::Pod;

pub fn get_pod_status(pod: &Pod) -> String {
    if let Some(status) = &pod.status {
        if let Some(container_statuses) = &status.container_statuses {
            for container in container_statuses {
                if let Some(state) = &container.state {
                    if let Some(waiting) = &state.waiting {
                        return waiting
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string());
                    } else if let Some(terminated) = &state.terminated {
                        return terminated
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string());
                    }
                }
            }
        }

        return status
            .phase
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
    }
    "Unknown".to_string()
}

pub fn get_machine_name() -> String {
    env::var("MACHINE_NAME").unwrap_or_else(|_| {
        hostname::get()
            .expect("os provide host name")
            .to_string_lossy()
            .to_string()
    })
}
