use crate::core::{
    AccessMode,
    StorageOptions,
};

#[derive(Clone, Debug)]
pub struct KubeOptions {
    pub(crate) db_url: String,
    pub(crate) storage: StorageOptions,
}

impl Default for KubeOptions {
    fn default() -> Self {
        Self {
            db_url: "".to_string(),
            storage: StorageOptions {
                class_name: Some("jz-flow-fs".to_string()),
                capacity: Some("1Gi".to_string()),
                access_mode: Some(AccessMode::ReadWriteMany),
            },
        }
    }
}
pub fn merge_storage_options(opt1: &StorageOptions, opt2: &StorageOptions) -> StorageOptions {
    let mut opts = StorageOptions {
        class_name: opt1.class_name.clone(),
        capacity: opt1.capacity.clone(),
        access_mode: opt1.access_mode.clone(),
    };
    if let Some(class_name) = opt2.class_name.as_ref() {
        opts.class_name = Some(class_name.clone());
    }
    if let Some(capacity) = opt2.capacity.as_ref() {
        opts.capacity = Some(capacity.clone());
    }
    if let Some(access_mode) = opt2.access_mode.as_ref() {
        opts.access_mode = Some(access_mode.clone());
    }
    opts
}

impl KubeOptions {
    pub fn set_db_url(mut self, db_url: &str) -> Self {
        self.db_url = db_url.to_string();
        self
    }

    pub fn set_storage_class(mut self, class_name: &str) -> Self {
        self.storage.class_name = Some(class_name.to_string());
        self
    }

    pub fn set_capacity(mut self, capacity: &str) -> Self {
        self.storage.capacity = Some(capacity.to_string());
        self
    }

    pub fn set_access_mode(mut self, mode: AccessMode) -> Self {
        self.storage.access_mode = Some(mode);
        self
    }
}
