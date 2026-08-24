// `sqlx::migrate!` embeds the SQL at compile time, and only a build
// script can tell Cargo to rebuild when those files change.
fn main() {
    for source in [
        "../database-manager/migrations",
        "../../apps/api/migrations",
    ] {
        println!("cargo:rerun-if-changed={source}");
    }
}
