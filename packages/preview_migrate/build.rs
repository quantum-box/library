fn main() {
    // `sqlx::migrate!` embeds migration files at compile time but does not
    // notice newly added files on its own; without these hints a new
    // migration could ship a preview-migrate Lambda that silently lacks it.
    for source in [
        "../database-manager/migrations",
        "../../apps/api/migrations",
    ] {
        println!("cargo:rerun-if-changed={source}");
    }
}
