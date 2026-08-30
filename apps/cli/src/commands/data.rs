//! `library data` — the records inside a repository.

use anyhow::{anyhow, bail, Result};
use clap::{Args, Subcommand};
use serde_json::{json, Map, Value};

use crate::client::LibraryClient;
use crate::commands::{parse_pair, parse_repo_ref, read_possible_file};
use crate::output::{array, field, print_json, Format, Table};

#[derive(Subcommand)]
pub enum DataCommand {
    /// List records in a repository
    List {
        /// Repository as `org/repo`
        repo: String,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        page_size: u32,
    },
    /// Find records by name
    Search {
        /// Repository as `org/repo`
        repo: String,
        /// Record name to match
        query: String,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        page_size: u32,
    },
    /// Show one record
    Get {
        /// Repository as `org/repo`
        repo: String,
        /// Record id
        data_id: String,
        /// Print the record as Markdown with YAML frontmatter
        #[arg(long)]
        markdown: bool,
    },
    /// Create a record
    Create {
        /// Repository as `org/repo`
        repo: String,
        /// Record name
        #[arg(long)]
        name: String,
        #[command(flatten)]
        values: PropertyValueArgs,
    },
    /// Replace a record. Properties left unset are cleared, so send every
    /// value the record should keep.
    Update {
        /// Repository as `org/repo`
        repo: String,
        /// Record id
        data_id: String,
        /// Record name
        #[arg(long)]
        name: String,
        #[command(flatten)]
        values: PropertyValueArgs,
    },
    /// Delete a record
    Delete {
        /// Repository as `org/repo`
        repo: String,
        /// Record id
        data_id: String,
        /// Delete without the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

/// How a caller supplies property values.
///
/// Each flag names a property by id or by name and differs only in how
/// the value is typed on the way in, because the API infers a property
/// value's kind from the JSON it receives. Every value may be given as
/// `@path` to read a file, or `@-` to read standard input.
#[derive(Args, Clone, Default)]
pub struct PropertyValueArgs {
    /// Set a property to a plain string: `--set body=hello`
    #[arg(long = "set", value_name = "PROPERTY=VALUE")]
    pub set: Vec<String>,

    /// Set a property to Markdown: `--set-markdown body=@notes.md`
    #[arg(long = "set-markdown", value_name = "PROPERTY=VALUE")]
    pub set_markdown: Vec<String>,

    /// Set a property to raw JSON, for numbers, lists, and relations:
    /// `--set-json count=42`
    #[arg(long = "set-json", value_name = "PROPERTY=JSON")]
    pub set_json: Vec<String>,
}

impl PropertyValueArgs {
    fn is_empty(&self) -> bool {
        self.set.is_empty()
            && self.set_markdown.is_empty()
            && self.set_json.is_empty()
    }
}

pub async fn run(
    command: DataCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        DataCommand::List {
            repo,
            page,
            page_size,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(
                    &format!("/v1beta/repos/{org}/{repo}/data-list"),
                    &[
                        ("page", page.to_string()),
                        ("page_size", page_size.to_string()),
                    ],
                )
                .await?;
            render_data_list(&response, format);
            Ok(())
        }
        DataCommand::Search {
            repo,
            query,
            page,
            page_size,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let response = client
                .get(
                    &format!("/v1beta/repos/{org}/{repo}/data"),
                    &[
                        ("name", query),
                        ("page", page.to_string()),
                        ("page_size", page_size.to_string()),
                    ],
                )
                .await?;
            render_data_list(&response, format);
            Ok(())
        }
        DataCommand::Get {
            repo,
            data_id,
            markdown,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            if markdown {
                let body = client
                    .get_text(&format!(
                        "/v1beta/repos/{org}/{repo}/data/{data_id}/md"
                    ))
                    .await?;
                match format {
                    Format::Json => print_json(&json!({
                        "id": data_id,
                        "markdown": body,
                    })),
                    Format::Text => print!("{body}"),
                }
                return Ok(());
            }

            let response = client
                .get(
                    &format!("/v1beta/repos/{org}/{repo}/data/{data_id}"),
                    &[],
                )
                .await?;
            render_data(&response, format);
            Ok(())
        }
        DataCommand::Create { repo, name, values } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let property_data =
                build_property_data(client, &org, &repo, &values).await?;
            let response = client
                .post(
                    &format!("/v1beta/repos/{org}/{repo}/data"),
                    json!({
                        "name": name,
                        "property_data": property_data,
                    }),
                )
                .await?;
            render_data(&response, format);
            Ok(())
        }
        DataCommand::Update {
            repo,
            data_id,
            name,
            values,
        } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            let property_data =
                build_property_data(client, &org, &repo, &values).await?;
            let response = client
                .put(
                    &format!("/v1beta/repos/{org}/{repo}/data/{data_id}"),
                    json!({
                        "name": name,
                        "property_data": property_data,
                    }),
                )
                .await?;
            render_data(&response, format);
            Ok(())
        }
        DataCommand::Delete { repo, data_id, yes } => {
            let (org, repo) = parse_repo_ref(&repo)?;
            crate::commands::confirm(
                yes,
                &format!("Delete record {data_id} from {org}/{repo}?"),
            )?;
            client
                .delete(&format!(
                    "/v1beta/repos/{org}/{repo}/data/{data_id}"
                ))
                .await?;
            crate::commands::report_deleted(format, "data", &data_id);
            Ok(())
        }
    }
}

/// Turn the `--set*` flags into the `property_data` the API expects.
///
/// Properties may be named rather than given by id, which means the
/// repository's property list has to be fetched first — but only when
/// something actually needs resolving.
async fn build_property_data(
    client: &LibraryClient,
    org: &str,
    repo: &str,
    values: &PropertyValueArgs,
) -> Result<Vec<Value>> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    let properties = client
        .get(&format!("/v1beta/repos/{org}/{repo}/properties"), &[])
        .await?;
    let index = PropertyIndex::from_response(&properties);

    let mut entries: Vec<Value> = Vec::new();
    for raw in &values.set {
        let (key, value) = parse_pair(raw)?;
        entries.push(json!({
            "property_id": index.resolve(&key)?,
            "value": Value::String(read_possible_file(&value)?),
        }));
    }
    for raw in &values.set_markdown {
        let (key, value) = parse_pair(raw)?;
        entries.push(json!({
            "property_id": index.resolve(&key)?,
            // The API reads `{"markdown": …}` as a Markdown value; a bare
            // string would land as plain text instead.
            "value": json!({ "markdown": read_possible_file(&value)? }),
        }));
    }
    for raw in &values.set_json {
        let (key, value) = parse_pair(raw)?;
        let text = read_possible_file(&value)?;
        let parsed: Value =
            serde_json::from_str(&text).map_err(|error| {
                anyhow!(
                    "--set-json {key}: value is not valid JSON: {error}"
                )
            })?;
        entries.push(json!({
            "property_id": index.resolve(&key)?,
            "value": parsed,
        }));
    }

    Ok(entries)
}

/// Maps whatever a caller typed — an id or a property name — onto a
/// property id.
struct PropertyIndex {
    by_id: Vec<String>,
    by_name: Map<String, Value>,
}

impl PropertyIndex {
    fn from_response(properties: &Value) -> Self {
        let list = properties.as_array().cloned().unwrap_or_default();
        let mut by_id = Vec::new();
        let mut by_name = Map::new();

        for property in &list {
            let Some(id) = property.get("id").and_then(Value::as_str)
            else {
                continue;
            };
            by_id.push(id.to_string());
            if let Some(name) = property.get("name").and_then(Value::as_str)
            {
                by_name.insert(name.to_string(), json!(id));
            }
        }

        Self { by_id, by_name }
    }

    fn resolve(&self, key: &str) -> Result<String> {
        if self.by_id.iter().any(|id| id == key) {
            return Ok(key.to_string());
        }
        if let Some(id) = self.by_name.get(key).and_then(Value::as_str) {
            return Ok(id.to_string());
        }

        let known =
            self.by_name.keys().cloned().collect::<Vec<_>>().join(", ");
        bail!(
            "no property named or identified by `{key}` in this \
             repository\nknown properties: {known}"
        );
    }
}

fn render_data_list(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    let records = array(response, "data");
    let mut table = Table::new(&["ID", "NAME"]);
    for record in records {
        table.push(vec![field(record, "id"), field(record, "name")]);
    }
    table.print();

    if let Some(paginator) = response.get("paginator") {
        println!();
        println!("paginator: {paginator}");
    }
}

fn render_data(response: &Value, format: Format) {
    if format == Format::Json {
        print_json(response);
        return;
    }

    println!("{}", field(response, "name"));
    println!("  id:  {}", field(response, "id"));
    println!("  url: {}", field(response, "url"));
    println!();

    let mut table = Table::new(&["PROPERTY", "TYPE", "VALUE"]);
    for item in array(response, "items") {
        let (kind, preview) = describe_value(item.get("value"));
        table.push(vec![field(item, "key"), kind, preview]);
    }
    table.print();
}

/// A property value arrives as a single-key object naming its kind, e.g.
/// `{"markdown": "# Title"}`. Render the kind and a one-line preview.
fn describe_value(value: Option<&Value>) -> (String, String) {
    let Some(Value::Object(map)) = value else {
        return ("-".to_string(), "-".to_string());
    };
    let Some((kind, inner)) = map.iter().next() else {
        return ("-".to_string(), "-".to_string());
    };

    let preview = match inner {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let preview = preview.replace(['\n', '\r'], " ");
    let preview = if preview.chars().count() > 60 {
        let head: String = preview.chars().take(59).collect();
        format!("{head}…")
    } else {
        preview
    };

    (kind.clone(), preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> PropertyIndex {
        PropertyIndex::from_response(&json!([
            { "id": "prop_1", "name": "Body" },
            { "id": "prop_2", "name": "Title" },
        ]))
    }

    #[test]
    fn a_property_id_resolves_to_itself() {
        assert_eq!(index().resolve("prop_1").unwrap(), "prop_1");
    }

    #[test]
    fn a_property_name_resolves_to_its_id() {
        assert_eq!(index().resolve("Title").unwrap(), "prop_2");
    }

    #[test]
    fn an_unknown_property_lists_the_ones_that_exist() {
        let error = index().resolve("Missing").unwrap_err().to_string();

        assert!(error.contains("Missing"));
        assert!(error.contains("Body"));
        assert!(error.contains("Title"));
    }

    #[test]
    fn a_value_renders_as_its_kind_and_a_preview() {
        let value = json!({ "markdown": "# Title\nbody" });

        let (kind, preview) = describe_value(Some(&value));

        assert_eq!(kind, "markdown");
        assert_eq!(preview, "# Title body");
    }

    #[test]
    fn a_missing_value_renders_as_dashes() {
        assert_eq!(
            describe_value(Some(&Value::Null)),
            ("-".to_string(), "-".to_string())
        );
        assert_eq!(
            describe_value(None),
            ("-".to_string(), "-".to_string())
        );
    }

    #[test]
    fn a_long_value_preview_is_truncated() {
        let value = json!({ "string": "x".repeat(200) });

        let (_, preview) = describe_value(Some(&value));

        assert_eq!(preview.chars().count(), 60);
        assert!(preview.ends_with('…'));
    }
}
