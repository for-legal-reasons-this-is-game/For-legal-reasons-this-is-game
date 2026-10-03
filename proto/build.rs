use std::{env, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let protos = ["common.proto", "events.proto", "service.proto"];

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("trading_service.bin"))
        .compile_protos(&protos, &["."])?;

    Ok(())
}
