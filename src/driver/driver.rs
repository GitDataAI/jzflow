use crate::core::GID;
use crate::Dag;
use anyhow::Result;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
pub trait UnitHandler<ID>
where
    ID: GID,
{
    //pause graph running for now
    fn pause(&mut self) -> impl Future<Output = Result<()>> + Send;

    //restart paused graph
    fn restart(&mut self) -> impl Future<Output = Result<()>> + Send;

    //stop resource about this graph
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;

    //return a channel handler
    fn channel_handler(
        &self,
    ) -> impl Future<Output = Result<Option<Arc<Mutex<impl ChannelHandler<ID>>>>>> + Send;
}

pub trait ChannelHandler<ID>
where
    ID: GID,
{
    //pause graph running for now
    fn pause(&mut self) -> impl Future<Output = Result<()>> + Send;

    //restart paused graph
    fn restart(&mut self) -> impl Future<Output = Result<()>> + Send;

    //stop resource about this graph
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub trait PipelineController<ID>
where
    ID: GID,
{
    fn get_node<'a>(
        &'a self,
        id: &'a ID,
    ) -> impl std::future::Future<Output = Result<&'a impl UnitHandler<ID>>> + Send;

    fn get_node_mut<'a>(
        &'a mut self,
        id: &'a ID,
    ) -> impl std::future::Future<Output = Result<&'a mut impl UnitHandler<ID>>> + Send;
}

pub trait Driver<ID>
where
    ID: GID,
{
    //deploy graph to cluster
    fn deploy(
        &self,
        namespace: &str,
        graph: &Dag<ID>,
    ) -> impl Future<Output = Result<impl PipelineController<ID>>> + Send;

    //attach cluster in cloud with graph
    fn attach(
        &self,
        namespace: &str,
        graph: &Dag<ID>,
    ) -> impl Future<Output = Result<impl PipelineController<ID>>> + Send;

    //clean all resource about this graph
    fn clean(&self, namespace: &str) -> impl Future<Output = Result<()>> + Send;
}
