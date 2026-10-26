fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Client will be in flowr-tasks
        .compile_protos(&["../../proto/clawdb.proto"], &["../../proto"])?;
    Ok(())
}
