use std::time::Duration;

use anyhow::{
    anyhow,
    Result,
};
use tokio::{
    task::JoinSet,
    time::sleep,
};
use tracing::{
    error,
    info,
};

pub async fn monitor_tasks(join_set: &mut JoinSet<Result<()>>) -> Result<()> {
    let mut has_err = false;
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok(Err(err)) => {
                has_err = true;
                error!("Task exited with error: {err}");
            }
            Err(join_err) => {
                has_err = true;
                error!("Failed to join task: {join_err}");
            }
            _ => {}
        }
    }

    if !has_err {
        info!("Gracefully shutting down...");
        // Prevent StatefulSet from restarting by sleeping for a long duration
        sleep(Duration::from_days(364 * 100)).await;
        Ok(())
    } else {
        Err(anyhow!("One or more tasks exited with an error"))
    }
}
