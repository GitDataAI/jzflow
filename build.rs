use tonic_build::compile_protos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let protos = [
        "src/network/protos/helloworld.proto",
        "src/network/protos/common.proto",
        "src/network/protos/datatransfer.proto",
        "src/network/protos/nodecontroller.proto",
    ];

    // fix use of deprecated method `tonic_build::Builder::compile`: renamed to `compile_protos()`
    //
    // let proto_dir = "src/network/protos";
    // tonic_build::configure()
    //     .compile(&protos, &[proto_dir])?;
    compile_protos(protos[0])?;
    compile_protos(protos[1])?;
    compile_protos(protos[2])?;
    compile_protos(protos[3])?;
    Ok(())
}
