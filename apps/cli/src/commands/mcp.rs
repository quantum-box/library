//! `library mcp` — talk to the Library MCP server, and set up clients
//! that will.
//!
//! Library's MCP server is remote: it lives at `/mcp` on the API rather
//! than being spawned as a subprocess. These commands let an operator
//! see exactly what an agent will see, and print the client
//! configuration that points an agent at it.

use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Map, Value};

use crate::client::LibraryClient;
use crate::commands::{parse_pair, read_possible_file};
use crate::output::{array, field, print_json, short_field, Format, Table};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum McpTransport {
    /// Single request/response JSON-RPC over `POST /mcp`
    Http,
    /// Event stream over `GET /sse` with requests posted to
    /// `/messages`. Non-GA: the server registers these routes only when
    /// it sets `LIBRARY_MCP_SSE_ENABLED=true`
    Sse,
}

#[derive(Subcommand)]
pub enum McpCommand {
    /// Show the server info the MCP handshake returns
    Info,
    /// List the tools the server offers for the current credentials
    Tools,
    /// Call one MCP tool
    Call {
        /// Tool name, as listed by `library mcp tools`
        name: String,
        /// Tool argument as a string: `--arg org=acme`. May be given as
        /// `@path` to read a file, or `@-` for standard input.
        #[arg(long = "arg", value_name = "KEY=VALUE")]
        args: Vec<String>,
        /// Tool argument as raw JSON, for numbers, booleans, and lists:
        /// `--arg-json page=2`
        #[arg(long = "arg-json", value_name = "KEY=JSON")]
        json_args: Vec<String>,
        /// The whole arguments object as JSON, instead of building it up
        /// flag by flag
        #[arg(long, value_name = "JSON", conflicts_with_all = ["args", "json_args"])]
        arguments: Option<String>,
    },
    /// Print MCP client configuration pointing at this Library API
    Config {
        /// Which transport the client should use
        #[arg(long, value_enum, default_value_t = McpTransport::Http)]
        transport: McpTransport,
        /// Name the server appears under in the client
        #[arg(long, default_value = "library")]
        name: String,
        /// Leave the API key out of the output
        #[arg(long)]
        no_key: bool,
    },
}

pub async fn run(
    command: McpCommand,
    client: &LibraryClient,
    format: Format,
) -> Result<()> {
    match command {
        McpCommand::Info => {
            let result = client
                .mcp_rpc(
                    "initialize",
                    Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {
                            "name": "library-cli",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    })),
                )
                .await?;
            render_info(&result, format);
            Ok(())
        }
        McpCommand::Tools => {
            let result = client.mcp_rpc("tools/list", None).await?;
            render_tools(&result, client.has_api_key(), format);
            Ok(())
        }
        McpCommand::Call {
            name,
            args,
            json_args,
            arguments,
        } => {
            let arguments = build_arguments(&args, &json_args, arguments)?;
            let result = client
                .mcp_rpc(
                    "tools/call",
                    Some(json!({
                        "name": name,
                        "arguments": arguments,
                    })),
                )
                .await?;
            render_tool_result(&result, format);
            Ok(())
        }
        McpCommand::Config {
            transport,
            name,
            no_key,
        } => {
            render_config(client, transport, &name, no_key, format);
            Ok(())
        }
    }
}

fn build_arguments(
    args: &[String],
    json_args: &[String],
    whole: Option<String>,
) -> Result<Value> {
    if let Some(whole) = whole {
        let text = read_possible_file(&whole)?;
        return serde_json::from_str(&text).map_err(|error| {
            anyhow::anyhow!("--arguments is not valid JSON: {error}")
        });
    }

    let mut arguments = Map::new();
    for raw in args {
        let (key, value) = parse_pair(raw)?;
        arguments.insert(key, Value::String(read_possible_file(&value)?));
    }
    for raw in json_args {
        let (key, value) = parse_pair(raw)?;
        let text = read_possible_file(&value)?;
        let parsed: Value =
            serde_json::from_str(&text).map_err(|error| {
                anyhow::anyhow!("--arg-json {key}: not valid JSON: {error}")
            })?;
        arguments.insert(key, parsed);
    }

    Ok(Value::Object(arguments))
}

fn render_info(result: &Value, format: Format) {
    if format == Format::Json {
        print_json(result);
        return;
    }

    let server = result.get("serverInfo").cloned().unwrap_or(Value::Null);
    println!("{} {}", field(&server, "name"), field(&server, "version"));
    println!("  protocol: {}", field(result, "protocolVersion"));
}

fn render_tools(result: &Value, authenticated: bool, format: Format) {
    if format == Format::Json {
        print_json(result);
        return;
    }

    let tools = array(result, "tools");
    let mut table = Table::new(&["TOOL", "DESCRIPTION"]);
    for tool in tools {
        table.push(vec![
            field(tool, "name"),
            short_field(tool, "description", 72),
        ]);
    }
    table.print();

    if !authenticated {
        println!();
        println!(
            "Only the anonymous tools are listed. Authenticate to see \
             the write tools: `library auth login --api-key pk_…`"
        );
    }
}

/// A tool result is a list of content blocks. The text blocks hold the
/// JSON the tool produced, so printing them raw is what a caller wants.
fn render_tool_result(result: &Value, format: Format) {
    if format == Format::Json {
        print_json(result);
        return;
    }

    let content = array(result, "content");
    if content.is_empty() {
        print_json(result);
        return;
    }

    for block in content {
        match block.get("text").and_then(Value::as_str) {
            Some(text) => println!("{text}"),
            None => print_json(block),
        }
    }
}

fn render_config(
    client: &LibraryClient,
    transport: McpTransport,
    name: &str,
    no_key: bool,
    format: Format,
) {
    let base = client.api_base_url();
    let (kind, url) = match transport {
        McpTransport::Http => ("http", format!("{base}/mcp")),
        McpTransport::Sse => ("sse", format!("{base}/sse")),
    };

    let mut server = json!({ "type": kind, "url": url });
    if !no_key {
        // Printed in full because a client config file needs the real
        // value; the warning below makes that explicit.
        let bearer = client.api_key().unwrap_or("pk_REPLACE_ME");
        server["headers"] =
            json!({ "Authorization": format!("Bearer {bearer}") });
    }

    let config = json!({ "mcpServers": { name: server } });
    print_json(&config);

    // `format` does not change the shape here — a config snippet is JSON
    // either way — but a machine reading stdout should not also get the
    // warning mixed into its stream.
    let _ = format;
    if !no_key && client.has_api_key() {
        eprintln!(
            "warning: this output contains an API key; pass --no-key to \
             leave it out"
        );
    }
    if transport == McpTransport::Sse {
        // Emitting a config that points at an endpoint the server does
        // not register would send the user to a client that just hangs.
        eprintln!(
            "warning: the SSE transport is Non-GA and off unless the \
             server sets LIBRARY_MCP_SSE_ENABLED=true; --transport http \
             works everywhere"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_and_json_arguments_land_in_one_object() {
        let arguments = build_arguments(
            &["org=acme".to_string()],
            &["page=2".to_string()],
            None,
        )
        .unwrap();

        assert_eq!(arguments["org"], json!("acme"));
        assert_eq!(arguments["page"], json!(2));
    }

    #[test]
    fn a_whole_arguments_object_is_used_as_given() {
        let arguments = build_arguments(
            &[],
            &[],
            Some(r#"{"org":"acme","repo":"docs"}"#.to_string()),
        )
        .unwrap();

        assert_eq!(arguments["repo"], json!("docs"));
    }

    #[test]
    fn a_malformed_json_argument_names_the_key_that_failed() {
        let error = build_arguments(&[], &["page=".to_string()], None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("page"));
    }

    #[test]
    fn no_arguments_produces_an_empty_object() {
        let arguments = build_arguments(&[], &[], None).unwrap();

        assert_eq!(arguments, json!({}));
    }
}
