use crate::Dag;
use anyhow::Result;

pub trait NHandler {
    //pause graph running for now
    fn pause(&mut self) -> Result<()>;

    //restart paused graph
    fn restart(&mut self) -> Result<()>;

    //stop resource about this graph
    fn stop(&mut self) -> Result<()>;
}

pub trait PipelineController {
    fn get_node(&self, name: &str) -> Option<Box<dyn NHandler>>;
}

pub trait Driver {
    //deploy graph to cluster
    fn deploy(&mut self, graph: &Dag) -> Result<Box<dyn PipelineController>>;

    //clean all resource about this graph
    fn clean(&mut self) -> Result<()>;
}
