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
        /// Property name
        name: String,
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
        /// New property name
        #[arg(long)]
        name: String,
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
            property_type,
            auto_generate,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let mut body = json!({
                "name": name,
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
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .put(
                    &format!(
                        "/v1beta/repos/{org}/{repo}/properties/\
                         {property_id}"
                    ),
                    json!({ "name": name }),
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
    let mut table = Table::new(&["ID", "NAME", "TYPE", "DEPRECATION"]);
    for property in &properties {
        table.push(vec![
            field(property, "id"),
            field(property, "name"),
            field(property, "property_type"),
            field(property, "deprecation"),
        ]);
    }
    table.print();
}

fn render_property(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!("{}", field(response, "name"));
    println!("  id:            {}", field(response, "id"));
    println!("  type:          {}", field(response, "property_type"));
    println!("  auto_generate: {}", field(response, "auto_generate"));
    println!("  deprecation:   {}", field(response, "deprecation"));
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
            ]
        );
    }
}
