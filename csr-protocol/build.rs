use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // setup the protobuf compiler from source
    env::set_var("PROTOC", protobuf_src::protoc());

    println!("cargo:rerun-if-changed=protos/csr.proto");

    // build our grpc service
    tonic_build::compile_protos("protos/csr.proto")?;
    Ok(())
}
