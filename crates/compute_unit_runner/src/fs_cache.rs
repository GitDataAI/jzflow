use anyhow::{
    anyhow,
    Result,
};
use async_trait::async_trait;
use jz_action::{
    network::datatransfer::{
        MediaDataBatchResponse,
        MediaDataCell,
    },
    utils::StdIntoAnyhowResult,
};
use std::{
    collections::HashMap,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
    time::Instant,
};
use tokio::{
    fs,
    sync::Mutex,
};
use tracing::{
    debug,
    error,
    info,
};
use walkdir::WalkDir;

#[async_trait]
pub trait FileCache: Send + Sync + 'static {
    async fn write(&self, batch: MediaDataBatchResponse) -> Result<()>;
    async fn read(&self, id: &str) -> Result<MediaDataBatchResponse>;
    async fn remove(&self, id: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct FSCache {
    path: PathBuf,
}

impl FSCache {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

#[async_trait]
impl FileCache for FSCache {
    async fn write(&self, batch: MediaDataBatchResponse) -> Result<()> {
        let now = Instant::now();
        let tmp_in_path = self.path.join(&batch.id);
        debug!(
            "try to create directory {:?} {:?}",
            tmp_in_path,
            now.elapsed()
        );
        if let Err(err) = fs::create_dir_all(&tmp_in_path).await {
            error!("create input dir {:?} fail {}", tmp_in_path, err);
            return Err(anyhow!("write create directory fail"));
        }

        let mut is_write_err = false;
        for (entry_path, entry) in batch
            .cells
            .iter()
            .map(|entry| (tmp_in_path.join(entry.path.clone()), entry))
        {
            if let Err(err) = fs::write(entry_path.clone(), &entry.data).await {
                error!("write file {:?} fail {}", entry_path, err);
                is_write_err = true;
                break;
            }
        }
        if is_write_err {
            return Err(anyhow!("write files fail"));
        }
        info!("write files to disk in {} {:?}", &batch.id, now.elapsed());
        Ok(())
    }

    async fn read(&self, id: &str) -> Result<MediaDataBatchResponse> {
        let now = Instant::now();
        let tmp_out_path = self.path.join(id);
        let mut new_batch = MediaDataBatchResponse::default();
        for entry in WalkDir::new(&tmp_out_path) {
            match entry {
                std::result::Result::Ok(entry) => {
                    if entry.file_type().is_file() {
                        let mut path = entry.path();
                        match fs::read(path).await {
                            std::result::Result::Ok(content) => {
                                path = path
                                    .strip_prefix(&tmp_out_path)
                                    .expect("file is in the folder");
                                new_batch.cells.push(MediaDataCell {
                                    size: content.len() as i32,
                                    path: path.to_str().unwrap().to_string(),
                                    data: content,
                                });
                            }
                            Err(err) => return Err(anyhow!("read file({:?}) fail {err}", path)),
                        }
                    }
                }
                Err(err) => return Err(anyhow!("walk files {:?} {err}", tmp_out_path)),
            }
        }
        new_batch.size = new_batch.cells.len() as u32;
        new_batch.id = id.to_string();

        info!("read files from disk in {} {:?}", id, now.elapsed());
        Ok(new_batch)
    }

    async fn remove(&self, id: &str) -> Result<()> {
        let tmp_out_path = self.path.join(id);
        fs::remove_dir_all(&tmp_out_path).await.anyhow()
    }
}

#[derive(Clone)]
pub struct MemCache(Arc<Mutex<HashMap<String, MediaDataBatchResponse>>>);

impl MemCache {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

#[async_trait]
impl FileCache for MemCache {
    async fn write(&self, batch: MediaDataBatchResponse) -> Result<()> {
        let now = Instant::now();
        let mut store = self.0.lock().await;
        let id = batch.id.clone();
        let _ = store.insert(id.clone(), batch);
        debug!("write files to memcache in {} {:?}", id, now.elapsed());
        Ok(())
    }

    async fn read(&self, id: &str) -> Result<MediaDataBatchResponse> {
        let now = Instant::now();
        let store = self.0.lock().await;
        let result = match store.get(id) {
            Some(val) => Ok(val.clone()),
            None => Err(anyhow!("data {} not foud", id)),
        };

        info!("read files from memcache in {} {:?}", id, now.elapsed());
        result
    }

    async fn remove(&self, id: &str) -> Result<()> {
        let mut store = self.0.lock().await;
        store.remove(id);
        Ok(())
    }
}
