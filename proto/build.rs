fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = ["common.proto", "events.proto", "service.proto"];

    for p in &protos {
        println!("cargo::rerun-if-changed={p}");
    }
    println!("cargo::rerun-if-changed=build.rs");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&protos, &["."])?;

    Ok(())
}
