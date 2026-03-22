fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["proto/sn13/v1/sn13_validator.proto"],
            &["proto"],
        )?;
    Ok(())
}
