fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile proto files when grpc feature is enabled
    #[cfg(feature = "grpc")]
    {
        use std::env;
        use std::path::PathBuf;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("grpc_health_v1.bin"))
            .build_server(true)
            .compile(&["proto/health.proto"], &["proto/"])?;

        tonic_build::configure()
            .file_descriptor_set_path(
                out_dir.join("helloworld_descriptor.bin"),
            )
            .compile(&["proto/helloworld.proto"], &["proto"])
            .unwrap();

        // auth_service
        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("auth_descriptor.bin"))
            .compile(&["../proto/auth.proto"], &["../proto"])
            .unwrap();
    }
    Ok(())
}
