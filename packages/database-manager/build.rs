fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(
            &["../../proto/v1/database_manager.proto"],
            &["../../proto/v1"],
        )
        .unwrap();
}
