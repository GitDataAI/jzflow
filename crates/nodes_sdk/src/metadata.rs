pub fn is_metadata(id: &str) -> bool {
    id.ends_with("metadata")
}

pub fn to_metadata(node_name: &str) -> String {
    node_name.to_owned() + "metadata"
}
