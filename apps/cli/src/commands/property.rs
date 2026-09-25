//! `library property` — the column definitions of a repository.

use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};

use crate::client::LibraryClient;
use crate::commands::parse_repo_ref;
use crate::output::{field, print_json, Format, Table};

/// The property types `POST /properties` accepts. Keeping this as an
/// enum means an unsupported type is rejected with the list of valid
/// ones instead of a bare "Invalid property type" from the API.
#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum PropertyTypeArg {
    String,
    Integer,
    /// Deprecated by the API in favour of `rich_text`
    Html,
    Markdown,
    Relation,
    Select,
    MultiSelect,
    Id,
    Location,
    Date,
    Image,
    RichText,
    Boolean,
}

impl PropertyTypeArg {
    fn as_api_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Html => "html",
            Self::Markdown => "markdown",
            Self::Relation => "relation",
            Self::Select => "select",
            Self::MultiSelect => "multi_select",
            Self::Id => "id",
            Self::Location => "location",
            Self::Date => "date",
            Self::Image => "image",
            Self::RichText => "rich_text",
            Self::Boolean => "boolean",
        }
    }
}

#[derive(Subcommand)]
pub enum PropertyCommand {
    /// List the properties of a repository
    List {
        /// Repository as `org/repo`
        repo: String,
    },
    /// Show one property
    Get {
        /// Repository as `org/repo`
        repo: String,
        /// Property id
        property_id: String,
    },
    /// Create a property
    Create {
        /// Repository as `org/repo`
        repo: String,
        /// Stable Property key (ASCII letter first; then letters, digits, underscores, or hyphens; max 64 characters)
        name: String,
        /// Human-readable label; defaults to the key
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long = "type", value_name = "TYPE")]
        property_type: PropertyTypeArg,
        /// Whether an `id` property generates its own values. Required
        /// for `--type id` and rejected for every other type.
        #[arg(long)]
        auto_generate: Option<bool>,
    },
    /// Rename a property
    Update {
        /// Repository as `org/repo`
        repo: String,
        /// Property id
        property_id: String,
        /// New stable property key
        #[arg(long)]
        name: Option<String>,
        /// New human-readable label
        #[arg(long)]
        display_name: Option<String>,
    },
    /// Delete a property
    Delete {
        /// Repository as `org/repo`
        repo: String,
        /// Property id
        property_id: String,
        /// Delete without the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(
    command: PropertyCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        PropertyCommand::List { repo } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(&format!("/v1beta/repos/{org}/{repo}/properties"), &[])
                .await?;
            render_properties(&response, format);
            Ok(())
        }
        PropertyCommand::Get { repo, property_id } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(
                    &format!(
                        "/v1beta/repos/{org}/{repo}/properties/\
                         {property_id}"
                    ),
                    &[],
                )
                .await?;
            render_property(&response, format);
            Ok(())
        }
        PropertyCommand::Create {
            repo,
            name,
            display_name,
            property_type,
            auto_generate,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let mut body = json!({
                "name": name,
                "display_name": display_name,
                "property_type": property_type.as_api_str(),
            });
            if let Some(auto_generate) = auto_generate {
                body["auto_generate"] = json!(auto_generate);
            }
            let response = client
                .post(
                    &format!("/v1beta/repos/{org}/{repo}/properties"),
                    body,
                )
                .await?;
            render_property(&response, format);
            Ok(())
        }
        PropertyCommand::Update {
            repo,
            property_id,
            name,
            display_name,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            if name.is_none() && display_name.is_none() {
                anyhow::bail!("provide --name and/or --display-name");
            }
            let mut body = serde_json::Map::new();
            if let Some(name) = name {
                body.insert("name".to_string(), json!(name));
            }
            if let Some(display_name) = display_name {
                body.insert(
                    "display_name".to_string(),
                    json!(display_name),
                );
            }
            let response = client
                .put(
                    &format!(
                        "/v1beta/repos/{org}/{repo}/properties/\
                         {property_id}"
                    ),
                    Value::Object(body),
                )
                .await?;
            render_property(&response, format);
            Ok(())
        }
        PropertyCommand::Delete {
            repo,
            property_id,
            yes,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            crate::commands::confirm(
                yes,
                &format!(
                    "Delete property {property_id} from {org}/{repo}? \
                     Its values go with it."
                ),
            )?;
            client
                .delete(&format!(
                    "/v1beta/repos/{org}/{repo}/properties/{property_id}"
                ))
                .await?;
            crate::commands::report_deleted(
                format,
                "property",
                &property_id,
            );
            Ok(())
        }
    }
}

fn render_properties(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    let properties = response.as_array().cloned().unwrap_or_default();
    let mut table = Table::new(&["ID", "KEY", "DISPLAY NAME", "TYPE"]);
    for property in &properties {
        table.push(vec![
            field(property, "id"),
            field(property, "name"),
            field(property, "display_name"),
            field(property, "property_type"),
        ]);
    }
    table.print();
}

fn render_property(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!("{}", field(response, "display_name"));
    println!("  id:            {}", field(response, "id"));
    println!("  key:           {}", field(response, "name"));
    println!("  type:          {}", field(response, "property_type"));
    println!("  auto_generate: {}", field(response, "auto_generate"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_property_type_maps_to_the_string_the_api_parses() {
        // These are exactly the values `property_type_from_request` in
        // the API matches on; a mismatch here is a 400 at runtime.
        assert_eq!(
            PropertyTypeArg::MultiSelect.as_api_str(),
            "multi_select"
        );
        assert_eq!(PropertyTypeArg::RichText.as_api_str(), "rich_text");
        assert_eq!(PropertyTypeArg::String.as_api_str(), "string");
    }

    /// The CLI offers the same types the MCP `create_property` schema
    /// does. A type present there but missing here is one a caller can
    /// reach over MCP and not from the command line.
    #[test]
    fn the_offered_types_match_the_mcp_property_schema() {
        let offered = PropertyTypeArg::value_variants()
            .iter()
            .map(|variant| variant.as_api_str())
            .collect::<Vec<_>>();

        assert_eq!(
            offered,
            vec![
                "string",
                "integer",
                "html",
                "markdown",
                "relation",
                "select",
                "multi_select",
                "id",
                "location",
                "date",
                "image",
                "rich_text",
                "boolean",
            ]
        );
    }
}
