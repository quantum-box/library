//! `library repo` — repositories.

use anyhow::Result;
use clap::Subcommand;
use serde_json::{json, Value};

use crate::client::LibraryClient;
use crate::commands::org::visibility;
use crate::commands::parse_repo_ref;
use crate::output::{array, field, print_json, short_field, Format, Table};

#[derive(Subcommand)]
pub enum RepoCommand {
    /// List the repositories an organization owns
    List {
        /// Organization username
        org: String,
    },
    /// Search repositories by name
    Search {
        /// Organization username to search within
        #[arg(long)]
        org: Option<String>,
        /// Repository name to match
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// Show one repository
    Get {
        /// Repository as `org/repo`
        repo: String,
    },
    /// Create a repository
    Create {
        /// Repository as `org/repo`, where `repo` becomes its username
        repo: String,
        /// Display name. Defaults to the repository username.
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        /// Make the repository readable without signing in
        #[arg(long)]
        public: bool,
    },
    /// Update a repository
    Update {
        /// Repository as `org/repo`
        repo: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        /// Make the repository public
        #[arg(long, conflicts_with = "private")]
        public: bool,
        /// Make the repository private
        #[arg(long, conflicts_with = "public")]
        private: bool,
        /// Replace the tag list. Repeat the flag for several tags.
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// Rename a repository
    Rename {
        /// Repository as `org/repo`
        repo: String,
        /// New repository username
        new_username: String,
    },
    /// Delete a repository and everything in it
    Delete {
        /// Repository as `org/repo`
        repo: String,
        /// Delete without the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(
    command: RepoCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        RepoCommand::List { org } => {
            // The organization endpoint carries the org in its path, so
            // an API key authenticates there. `/v1beta/repos` does not,
            // which is why listing and searching use different routes.
            let response =
                client.get(&format!("/v1beta/orgs/{org}"), &[]).await?;
            let repos = array(&response, "repos").to_vec();
            render_repos(&Value::Array(repos), format);
            Ok(())
        }
        RepoCommand::Search { org, name, limit } => {
            let mut query: Vec<(&str, String)> =
                vec![("limit", limit.to_string())];
            if let Some(org) = org {
                query.push(("org", org));
            }
            if let Some(name) = name {
                query.push(("name", name));
            }
            let response = client.get("/v1beta/repos", &query).await?;
            render_repos(&response, format);
            Ok(())
        }
        RepoCommand::Get { repo } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(&format!("/v1beta/repos/{org}/{repo}"), &[])
                .await?;
            render_repo(&response, format);
            Ok(())
        }
        RepoCommand::Create {
            repo,
            name,
            description,
            public,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let body = json!({
                "name": name.unwrap_or_else(|| repo.clone()),
                "username": repo,
                "description": description,
                "is_public": public,
                "database_id": Value::Null,
            });
            let response =
                client.post(&format!("/v1beta/repos/{org}"), body).await?;
            render_repo(&response, format);
            Ok(())
        }
        RepoCommand::Update {
            repo,
            name,
            description,
            public,
            private,
            tags,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            // Absent means "leave alone", so only send what was asked
            // for. `--tag` with no occurrences is absent, not empty.
            let mut body = json!({});
            if let Some(name) = name {
                body["name"] = json!(name);
            }
            if let Some(description) = description {
                body["description"] = json!(description);
            }
            if public || private {
                body["is_public"] = json!(public);
            }
            if !tags.is_empty() {
                body["tags"] = json!(tags);
            }

            let response = client
                .put(&format!("/v1beta/repos/{org}/{repo}"), body)
                .await?;
            render_repo(&response, format);
            Ok(())
        }
        RepoCommand::Rename { repo, new_username } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .put(
                    &format!("/v1beta/repos/{org}/{repo}/change-username"),
                    json!({ "new_username": new_username }),
                )
                .await?;
            render_repo(&response, format);
            Ok(())
        }
        RepoCommand::Delete { repo, yes } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            crate::commands::confirm(
                yes,
                &format!("Delete {org}/{repo} and every record in it?"),
            )?;
            client
                .delete(&format!("/v1beta/repos/{org}/{repo}"))
                .await?;
            crate::commands::report_deleted(
                format,
                "repo",
                &format!("{org}/{repo}"),
            );
            Ok(())
        }
    }
}

fn render_repos(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    let repos = response.as_array().cloned().unwrap_or_default();
    let mut table = Table::new(&[
        "ORG",
        "USERNAME",
        "NAME",
        "VISIBILITY",
        "DESCRIPTION",
    ]);
    for repo in &repos {
        table.push(vec![
            field(repo, "org_username"),
            field(repo, "username"),
            field(repo, "name"),
            visibility(repo),
            short_field(repo, "description", 48),
        ]);
    }
    table.print();
}

fn render_repo(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!(
        "{}/{}",
        field(response, "org_username"),
        field(response, "username")
    );
    println!("  name:        {}", field(response, "name"));
    println!("  id:          {}", field(response, "id"));
    println!("  visibility:  {}", visibility(response));
    println!("  description: {}", field(response, "description"));
}
