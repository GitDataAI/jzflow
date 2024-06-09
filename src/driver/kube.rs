use super::Driver;
use crate::Dag;
use anyhow::Result;
pub struct KubeDriver {}

impl Driver for KubeDriver {
    fn deploy(&mut self, graph: &Dag) -> Result<Box<dyn super::PipelineController>> {
        todo!()
    }

    fn clean(&mut self) -> Result<()> {
        todo!()
    }
}
