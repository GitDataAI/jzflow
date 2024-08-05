pub trait DBConfig {
    fn connection_string(&self) -> &str;
}
