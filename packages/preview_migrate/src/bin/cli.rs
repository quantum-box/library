//! Preview migration CLI.
//!
//! Used by CI replays and local runs. The deploy hook itself invokes the
//! Lambda entry point instead (see `bin/lambda.rs`), because the per-PR
//! database is only reachable from inside the app's VPC.

use std::env;
use std::process::ExitCode;

use library_api_preview_migrate::run_preview_migrations;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let raw_database_url =
        env::var(&args.database_url_env).map_err(|_| {
            format!(
                "{} is required for preview migrations",
                args.database_url_env
            )
        })?;
    run_preview_migrations(
        env::var("TACHYON_ENV").ok().as_deref(),
        &raw_database_url,
        &args.database_url_env,
    )
    .await
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    database_url_env: String,
}

impl Args {
    fn parse(
        args: impl IntoIterator<Item = String>,
    ) -> Result<Self, String> {
        let mut database_url_env = "DATABASE_URL".to_string();
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--database-url-env" => {
                    database_url_env = iter.next().ok_or_else(|| {
                        "--database-url-env requires a value".to_string()
                    })?;
                }
                other => {
                    return Err(format!("unexpected argument: {other}"))
                }
            }
        }
        Ok(Self { database_url_env })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_database_url() {
        assert_eq!(
            Args::parse(Vec::<String>::new()).unwrap(),
            Args {
                database_url_env: "DATABASE_URL".to_string()
            }
        );
    }

    #[test]
    fn reads_the_named_environment_variable() {
        assert_eq!(
            Args::parse([
                "--database-url-env".to_string(),
                "PREVIEW_DATABASE_URL".to_string(),
            ])
            .unwrap(),
            Args {
                database_url_env: "PREVIEW_DATABASE_URL".to_string()
            }
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(Args::parse(["--nope".to_string()]).is_err());
    }
}
