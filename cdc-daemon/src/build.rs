fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)   // Explicitly force client generation
        .build_server(false)  // BFF doesn't need to host a gRPC server
        .compile_protos(
            &["../proto/cdc_management.proto"], 
            &["../proto/"]
        )?;
    Ok(())
}