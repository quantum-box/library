//! `library org` — organizations.

use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::client::LibraryClient;
use crate::output::{array, field, print_json, short_field, Format, Table};

#[derive(Subcommand)]
pub enum OrgCommand {
    /// Show an organization and the repositories it owns
    Get {
        /// Organization username
        org: String,
    },
    /// Create an organization
    Create {
        /// Username (slug) for the new organization
        username: String,
        /// Display name. Defaults to the username.
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        website: Option<String>,
    },
    /// Update an organization
    Update {
        /// Organization username
        org: String,
        /// Display name. Required by the API, so it must be given even
        /// when only the description is changing.
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        website: Option<String>,
    },
}

pub async fn run(
    command: OrgCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        OrgCommand::Get { org } => {
            let response =
                client.get(&format!("/v1beta/orgs/{org}"), &[]).await?;
            render_org(&response, format);
            Ok(())
        }
        OrgCommand::Create {
            username,
            name,
            description,
            website,
        } => {
            let body = json!({
                "name": name.unwrap_or_else(|| username.clone()),
                "username": username,
                "description": description,
                "website": website,
            });
            let response = client.post("/v1beta/orgs", body).await?;
            render_org(&response, format);
            Ok(())
        }
        OrgCommand::Update {
            org,
            name,
            description,
            website,
        } => {
            let body = json!({
                "name": name,
                "description": description,
                "website": website,
            });
            let response =
                client.put(&format!("/v1beta/orgs/{org}"), body).await?;
            render_org(&response, format);
            Ok(())
        }
    }
}

fn render_org(response: &serde_json::Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!(
        "{} ({})",
        field(response, "name"),
        field(response, "username")
    );
    println!("  id:          {}", field(response, "id"));
    println!("  description: {}", field(response, "description"));
    println!("  website:     {}", field(response, "website"));

    let repos = array(response, "repos");
    println!();
    println!("Repositories ({}):", repos.len());
    let mut table =
        Table::new(&["USERNAME", "NAME", "VISIBILITY", "DESCRIPTION"]);
    for repo in repos {
        table.push(vec![
            field(repo, "username"),
            field(repo, "name"),
            visibility(repo),
            short_field(repo, "description", 48),
        ]);
    }
    table.print();
}

pub fn visibility(repo: &serde_json::Value) -> String {
    match repo.get("is_public").and_then(serde_json::Value::as_bool) {
        Some(true) => "public".to_string(),
        Some(false) => "private".to_string(),
        None => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn visibility_reads_the_public_flag() {
        assert_eq!(visibility(&json!({ "is_public": true })), "public");
        assert_eq!(visibility(&json!({ "is_public": false })), "private");
        assert_eq!(visibility(&json!({})), "-");
    }
}
