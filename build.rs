fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let protos = [
        "src/network/protos/helloworld.proto",
        "src/network/protos/common.proto",
        "src/network/protos/datatransfer.proto",
        "src/network/protos/nodecontroller.proto",
    ];

    let proto_dir = "src/network/protos";
    tonic_build::configure().compile(&protos, &[proto_dir])?;
    Ok(())
}
