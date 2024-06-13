use crate::core::GID;
use crate::Dag;
use anyhow::Result;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
pub trait NHandler {
    //pause graph running for now
    fn pause(&mut self) -> impl Future<Output = Result<()>> + Send;

    //restart paused graph
    fn restart(&mut self) -> impl Future<Output = Result<()>> + Send;

    //stop resource about this graph
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub trait PipelineController {
    fn get_node(&self, name: &str) -> impl Future<Output = Result<impl NHandler>> + Send;
}

pub trait Driver<ID>
where
    ID: GID,
{
    //deploy graph to cluster
    fn deploy(
        &self,
        graph: &Dag<ID>,
    ) -> impl Future<Output = Result<Arc<Mutex<impl PipelineController>>>> + Send;

    //clean all resource about this graph
    fn clean(&self) -> impl Future<Output = Result<()>> + Send;
}
