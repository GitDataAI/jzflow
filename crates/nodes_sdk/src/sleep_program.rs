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
    while let Some(result) = join_set.join_next().await {
        if let Err(err) = result {
            has_err = true;
            error!("Task exited with error: {err}");
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
