fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../cdc-daemon/proto/cdc_management.proto")?;
    Ok(())
}