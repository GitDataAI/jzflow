use crate::dag::Dag;
use anyhow::Result;
use std::{
    future::Future,
    sync::{
        Arc,
        Mutex,
    },
};
pub trait UnitHandler: Send {
    //pause graph running for now
    fn pause(&mut self) -> impl Future<Output = Result<()>> + Send;

    //restart paused graph
    fn restart(&mut self) -> impl Future<Output = Result<()>> + Send;

    //stop resource about this graph
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;

    //return a channel handler
    fn channel_handler(
        &self,
    ) -> impl Future<Output = Result<Option<Arc<Mutex<impl ChannelHandler>>>>> + Send;
}

pub trait ChannelHandler: Send {
    //pause graph running for now
    fn pause(&mut self) -> impl Future<Output = Result<()>> + Send;

    //restart paused graph
    fn restart(&mut self) -> impl Future<Output = Result<()>> + Send;

    //stop resource about this graph
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub trait PipelineController: Send {
    fn get_node<'a>(
        &'a self,
        id: &'a String,
    ) -> impl std::future::Future<Output = Result<&'a impl UnitHandler>> + Send;

    fn get_node_mut<'a>(
        &'a mut self,
        id: &'a String,
    ) -> impl std::future::Future<Output = Result<&'a mut impl UnitHandler>> + Send;
}

pub trait Driver: 'static + Clone + Send + Sync {
    //deploy graph to cluster
    fn deploy(
        &self,
        namespace: &str,
        graph: &Dag,
    ) -> impl Future<Output = Result<impl PipelineController>> + Send;

    //attach cluster in cloud with graph
    fn attach(
        &self,
        namespace: &str,
        graph: &Dag,
    ) -> impl Future<Output = Result<impl PipelineController>> + Send;

    //clean all resource about this graph
    fn clean(&self, namespace: &str) -> impl Future<Output = Result<()>> + Send;
}
