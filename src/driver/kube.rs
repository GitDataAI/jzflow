use super::{Driver, NHandler, PipelineController};
use crate::core::GID;
use crate::Dag;
use anyhow::{Error, Result};
use std::sync::{Arc, Mutex};
pub struct KubeHandler {}

impl NHandler for KubeHandler {
    async fn pause(&mut self) -> Result<()> {
        todo!()
    }

    async fn restart(&mut self) -> Result<()> {
        todo!()
    }

    async fn stop(&mut self) -> Result<()> {
        todo!()
    }
}
pub struct KubePipelineController {}

impl PipelineController for KubePipelineController {
    #[allow(refining_impl_trait)]
    async fn get_node(&self, name: &str) -> Result<KubeHandler> {
        todo!()
    }
}

pub struct KubeDriver<ID>
where
    ID: GID,
{
    _id: std::marker::PhantomData<ID>,
}

impl<ID> Driver<ID> for KubeDriver<ID>
where
    ID: GID,
{
    #[allow(refining_impl_trait)]
    async fn deploy(&self, graph: &Dag<ID>) -> Result<Arc<Mutex<KubePipelineController>>> {
        todo!()
    }

    async fn clean(&self) -> Result<()> {
        todo!()
    }
}
