//! `library api-key` — the keys an organization authenticates with.
//!
//! These live on the GraphQL endpoint rather than the REST surface the
//! rest of the CLI uses, so the calls here spell their queries out.
//!
//! Issuing and revoking are an owner's to do: with a key, that means a
//! key issued with `--role owner`. A key issued with a lesser role, or
//! none, can read and write records but not mint further keys.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::client::LibraryClient;
use crate::output::{array, field, print_json, Format, Table};

const CREATE_MUTATION: &str = "mutation CliCreateApiKey($input: CreateApiKeyInput!) {\n  createApiKey(input: $input) {\n    apiKey { id name value role createdAt }\n  }\n}\n";

const LIST_QUERY: &str = "query CliApiKeys($org: String!) {\n  apiKeys(orgUsername: $org) { id name role createdAt }\n}\n";

const REVOKE_MUTATION: &str = "mutation CliRevokeApiKey($input: RevokeApiKeyInput!) {\n  revokeApiKey(input: $input)\n}\n";

#[derive(clap::Subcommand)]
pub enum ApiKeyCommand {
    /// Issue a key for an organization
    Create {
        /// Organization username
        org: String,
        /// Name to tell this key apart later
        #[arg(long)]
        name: String,
        /// Repository access across the organization. Omit for a key
        /// that reaches public repositories only.
        #[arg(long, value_parser = ["reader", "writer", "owner"])]
        role: Option<String>,
    },
    /// List the keys an organization has issued
    List {
        /// Organization username
        org: String,
    },
    /// Revoke a key, by the id the listing reports
    Revoke {
        /// Organization username
        org: String,
        /// Key id (`pak_…`)
        api_key_id: String,
    },
}

pub async fn run(
    command: ApiKeyCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        ApiKeyCommand::Create { org, name, role } => {
            let operator_id = operator_id_for(client, &org).await?;
            let mut input = json!({
                "organizationUsername": org,
                "name": name,
            });
            if let Some(role) = role {
                input["role"] = json!(role.to_uppercase());
            }
            let data = client
                .graphql(
                    CREATE_MUTATION,
                    json!({ "input": input }),
                    operator_id.as_deref(),
                )
                .await?;
            let api_key = data
                .pointer("/createApiKey/apiKey")
                .ok_or_else(|| anyhow!("the API returned no key"))?;
            render_created(api_key, format);
            Ok(())
        }
        ApiKeyCommand::List { org } => {
            let operator_id = operator_id_for(client, &org).await?;
            let data = client
                .graphql(
                    LIST_QUERY,
                    json!({ "org": org }),
                    operator_id.as_deref(),
                )
                .await?;
            render_list(&data, format);
            Ok(())
        }
        ApiKeyCommand::Revoke { org, api_key_id } => {
            let operator_id = operator_id_for(client, &org).await?;
            let data = client
                .graphql(
                    REVOKE_MUTATION,
                    json!({
                        "input": {
                            "organizationUsername": org,
                            "apiKeyId": api_key_id,
                        }
                    }),
                    operator_id.as_deref(),
                )
                .await?;
            if format == Format::Json {
                print_json(&data);
            } else {
                println!("Revoked {api_key_id}");
            }
            Ok(())
        }
    }
}

/// The tenant a `pk_…` key is verified against. `/v1/graphql` carries no
/// organization in its path, so it travels as a header; the organization
/// page is where its id comes from, unless the profile already names one.
async fn operator_id_for(
    client: &LibraryClient,
    org: &str,
) -> Result<Option<String>> {
    if !client.has_api_key() {
        return Ok(None);
    }
    let response = client.get(&format!("/v1beta/orgs/{org}"), &[]).await?;
    Ok(response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string))
}

fn render_created(api_key: &Value, format: Format) {
    if format == Format::Json {
        print_json(api_key);
        return;
    }

    println!("{}", field(api_key, "name"));
    println!("  id:     {}", field(api_key, "id"));
    println!("  access: {}", role_label(api_key));
    println!("  key:    {}", field(api_key, "value"));
    println!();
    println!(
        "The key itself is readable this once; the listing shows \
         everything but the key."
    );
}

fn render_list(data: &Value, format: Format) {
    if format == Format::Json {
        print_json(data);
        return;
    }

    let keys = array(data, "apiKeys");
    let mut table = Table::new(&["ID", "NAME", "ACCESS", "CREATED"]);
    for api_key in keys {
        table.push(vec![
            field(api_key, "id"),
            field(api_key, "name"),
            role_label(api_key),
            field(api_key, "createdAt"),
        ]);
    }
    table.print();
}

/// A key with no role reaches public repositories only, which the API
/// reports as a null role.
fn role_label(api_key: &Value) -> String {
    match api_key.get("role").and_then(Value::as_str) {
        Some(role) => role.to_lowercase(),
        None => "public".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_without_a_role_reads_as_public() {
        assert_eq!(role_label(&json!({ "role": "READER" })), "reader");
        assert_eq!(role_label(&json!({ "role": null })), "public");
        assert_eq!(role_label(&json!({})), "public");
    }
}
