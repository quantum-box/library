//! `library source` — the external references attached to a repository.

use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Value};

use crate::client::LibraryClient;
use crate::commands::parse_repo_ref;
use crate::output::{field, print_json, Format, Table};

#[derive(Subcommand)]
pub enum SourceCommand {
    /// List the sources attached to a repository
    List {
        /// Repository as `org/repo`
        repo: String,
    },
    /// Show one source
    Get {
        /// Repository as `org/repo`
        repo: String,
        /// Source id
        source_id: String,
    },
    /// Attach a source to a repository
    Create {
        /// Repository as `org/repo`
        repo: String,
        /// Source name
        name: String,
        #[arg(long)]
        url: Option<String>,
    },
    /// Update a source
    Update {
        /// Repository as `org/repo`
        repo: String,
        /// Source id
        source_id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, conflicts_with = "clear_url")]
        url: Option<String>,
        /// Remove the URL, leaving the source with only a name
        #[arg(long, conflicts_with = "url")]
        clear_url: bool,
    },
    /// Delete a source
    Delete {
        /// Repository as `org/repo`
        repo: String,
        /// Source id
        source_id: String,
        /// Delete without the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(
    command: SourceCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        SourceCommand::List { repo } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(&format!("/v1beta/repos/{org}/{repo}/sources"), &[])
                .await?;
            render_sources(&response, format);
            Ok(())
        }
        SourceCommand::Get { repo, source_id } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(
                    &format!(
                        "/v1beta/repos/{org}/{repo}/sources/{source_id}"
                    ),
                    &[],
                )
                .await?;
            render_source(&response, format);
            Ok(())
        }
        SourceCommand::Create { repo, name, url } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .post(
                    &format!("/v1beta/repos/{org}/{repo}/sources"),
                    json!({ "name": name, "url": url }),
                )
                .await?;
            render_source(&response, format);
            Ok(())
        }
        SourceCommand::Update {
            repo,
            source_id,
            name,
            url,
            clear_url,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            // The API distinguishes an absent `url` from an explicit
            // null: absent leaves it alone, null unsets it.
            let mut body = json!({});
            if let Some(name) = name {
                body["name"] = json!(name);
            }
            if clear_url {
                body["url"] = Value::Null;
            } else if let Some(url) = url {
                body["url"] = json!(url);
            }

            let response = client
                .put(
                    &format!(
                        "/v1beta/repos/{org}/{repo}/sources/{source_id}"
                    ),
                    body,
                )
                .await?;
            render_source(&response, format);
            Ok(())
        }
        SourceCommand::Delete {
            repo,
            source_id,
            yes,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            crate::commands::confirm(
                yes,
                &format!("Delete source {source_id} from {org}/{repo}?"),
            )?;
            client
                .delete(&format!(
                    "/v1beta/repos/{org}/{repo}/sources/{source_id}"
                ))
                .await?;
            crate::commands::report_deleted(format, "source", &source_id);
            Ok(())
        }
    }
}

fn render_sources(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    let sources = response.as_array().cloned().unwrap_or_default();
    let mut table = Table::new(&["ID", "NAME", "URL"]);
    for source in &sources {
        table.push(vec![
            field(source, "id"),
            field(source, "name"),
            field(source, "url"),
        ]);
    }
    table.print();
}

fn render_source(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!("{}", field(response, "name"));
    println!("  id:      {}", field(response, "id"));
    println!("  repo_id: {}", field(response, "repo_id"));
    println!("  url:     {}", field(response, "url"));
}
