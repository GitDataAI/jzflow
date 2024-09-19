use super::*;

impl<T> KubeHandler<T>
where
    T: Repo,
{
    pub async fn inner_pause(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
