//! Limits on the shape of a GraphQL request.
//!
//! `/v1/graphql` is reachable anonymously, and a single document can
//! alias the same root field thousands of times. Every root `repo` or
//! `organization` alias on an unknown organization costs a database miss
//! and a tachyon-api lookup, so an unbounded document turns one request
//! into an unbounded upstream fan-out. These limits bound that.
//!
//! The numbers leave headroom over what the first-party clients send
//! today (apps/web, apps/client and apps/cli peak at depth 7, complexity
//! 66 and 4 root fields) and over the graphql-js introspection query that
//! GraphiQL issues (depth 15, complexity about 230).

use std::collections::HashSet;
use std::sync::Arc;

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextParseQuery,
};
use async_graphql::parser::types::{
    ExecutableDocument, Selection, SelectionSet,
};
use async_graphql::{
    Name, ObjectType, SchemaBuilder, ServerError, ServerResult,
    SubscriptionType, Variables,
};

/// Deepest selection nesting accepted, introspection included.
pub const MAX_QUERY_DEPTH: usize = 20;

/// Largest number of fields one document may select (async-graphql
/// counts each field as 1, with fragments expanded).
pub const MAX_QUERY_COMPLEXITY: usize = 500;

/// Most root-level fields, aliases included, a single operation may
/// select. This is the cap that bounds the per-request fan-out.
pub const MAX_ROOT_FIELDS: usize = 20;

/// Largest request body `/v1/graphql` reads.
///
/// Kept at axum's previous implicit default rather than lowered: record
/// bodies (`addData` / `updateData`) and rich-text templates (up to
/// 1 MiB before JSON escaping) are written through GraphQL, so a smaller
/// cap would reject saves that work today. The shape limits above are
/// what bound the cost of a request; this only makes the cap explicit.
pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;

/// Applies every request-shape limit to a schema under construction.
pub fn apply<Query, Mutation, Subscription>(
    builder: SchemaBuilder<Query, Mutation, Subscription>,
) -> SchemaBuilder<Query, Mutation, Subscription>
where
    Query: ObjectType + 'static,
    Mutation: ObjectType + 'static,
    Subscription: SubscriptionType + 'static,
{
    builder
        .limit_depth(MAX_QUERY_DEPTH)
        .limit_complexity(MAX_QUERY_COMPLEXITY)
        .extension(RootFieldLimit {
            max: MAX_ROOT_FIELDS,
        })
}

/// Rejects an operation that selects more than `max` root fields.
///
/// async-graphql has depth and complexity limits but nothing that counts
/// aliases, and complexity alone still admits a few hundred cheap root
/// aliases. The check runs right after parsing, before validation or any
/// resolver.
#[derive(Debug, Clone, Copy)]
pub struct RootFieldLimit {
    pub max: usize,
}

impl ExtensionFactory for RootFieldLimit {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(*self)
    }
}

#[async_trait::async_trait]
impl Extension for RootFieldLimit {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        for (_, operation) in document.operations.iter() {
            let count = count_root_fields(
                &operation.node.selection_set.node,
                &document,
                &mut HashSet::new(),
            );
            if count > self.max {
                return Err(ServerError::new(
                    format!(
                        "Operation selects {count} root fields; the limit is {}",
                        self.max
                    ),
                    Some(operation.pos),
                ));
            }
        }
        Ok(document)
    }
}

/// Root fields an operation resolves, counting through fragment spreads
/// and inline fragments. A fragment that spreads itself is counted once;
/// validation rejects the cycle afterwards.
fn count_root_fields<'a>(
    selection_set: &'a SelectionSet,
    document: &'a ExecutableDocument,
    visiting: &mut HashSet<&'a Name>,
) -> usize {
    selection_set
        .items
        .iter()
        .map(|selection| match &selection.node {
            Selection::Field(_) => 1,
            Selection::InlineFragment(fragment) => count_root_fields(
                &fragment.node.selection_set.node,
                document,
                visiting,
            ),
            Selection::FragmentSpread(spread) => {
                let name = &spread.node.fragment_name.node;
                let Some(definition) = document.fragments.get(name) else {
                    return 0;
                };
                if !visiting.insert(name) {
                    return 0;
                }
                let count = count_root_fields(
                    &definition.node.selection_set.node,
                    document,
                    visiting,
                );
                visiting.remove(name);
                count
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

    /// The graphql-js 16 introspection query with every option on, which
    /// is what GraphiQL and most codegen tools send.
    const INTROSPECTION_QUERY: &str =
        include_str!("testdata/introspection.graphql");

    struct Query;

    #[Object]
    impl Query {
        async fn value(&self) -> i32 {
            1
        }

        async fn node(&self) -> Node {
            Node
        }
    }

    struct Node;

    #[Object]
    impl Node {
        async fn id(&self) -> i32 {
            1
        }

        async fn child(&self) -> Node {
            Node
        }
    }

    fn schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
        apply(Schema::build(Query, EmptyMutation, EmptySubscription))
            .finish()
    }

    fn aliases(count: usize) -> String {
        let fields = (0..count)
            .map(|index| format!("a{index}: value"))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{{ {fields} }}")
    }

    fn nested(depth: usize) -> String {
        // `node` plus `depth - 2` `child` levels plus `id` = `depth` fields.
        let mut query = String::from("id");
        for _ in 0..depth.saturating_sub(2) {
            query = format!("child {{ {query} }}");
        }
        format!("{{ node {{ {query} }} }}")
    }

    async fn errors(query: &str) -> Vec<String> {
        schema()
            .execute(query)
            .await
            .errors
            .into_iter()
            .map(|error| error.message)
            .collect()
    }

    #[tokio::test]
    async fn accepts_root_fields_up_to_the_limit() {
        assert!(errors(&aliases(MAX_ROOT_FIELDS)).await.is_empty());
    }

    #[tokio::test]
    async fn rejects_aliases_beyond_the_root_field_limit() {
        let errors = errors(&aliases(MAX_ROOT_FIELDS + 1)).await;
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("root fields"), "{errors:?}");
    }

    #[tokio::test]
    async fn counts_root_fields_hidden_in_fragments() {
        let query = format!(
            "query {{ ...F ... on Query {{ {inline} }} }} fragment F on Query {{ {spread} }}",
            inline = (0..MAX_ROOT_FIELDS / 2)
                .map(|index| format!("i{index}: value"))
                .collect::<Vec<_>>()
                .join(" "),
            spread = (0..MAX_ROOT_FIELDS / 2 + 1)
                .map(|index| format!("s{index}: value"))
                .collect::<Vec<_>>()
                .join(" "),
        );
        let errors = errors(&query).await;
        assert!(
            errors.iter().any(|error| error.contains("root fields")),
            "{errors:?}"
        );
    }

    #[tokio::test]
    async fn a_self_spreading_fragment_does_not_hang_the_count() {
        let errors =
            errors("query { ...F } fragment F on Query { value ...F }")
                .await;
        assert!(!errors.is_empty(), "the cycle is still rejected");
    }

    #[tokio::test]
    async fn rejects_queries_deeper_than_the_limit() {
        assert!(errors(&nested(MAX_QUERY_DEPTH)).await.is_empty());
        let errors = errors(&nested(MAX_QUERY_DEPTH + 1)).await;
        assert!(
            errors.iter().any(|error| error.contains("too deep")),
            "{errors:?}"
        );
    }

    #[tokio::test]
    async fn rejects_queries_more_complex_than_the_limit() {
        // Stays under the root and depth limits; only the field count
        // is over.
        let wide = (0..MAX_QUERY_COMPLEXITY)
            .map(|index| format!("f{index}: id"))
            .collect::<Vec<_>>()
            .join(" ");
        let errors = errors(&format!("{{ node {{ {wide} }} }}")).await;
        assert!(
            errors.iter().any(|error| error.contains("too complex")),
            "{errors:?}"
        );
    }

    #[tokio::test]
    async fn the_application_schema_still_answers_introspection() {
        let schema = apply(async_graphql::Schema::build(
            super::super::Query::default(),
            super::super::Mutation::default(),
            EmptySubscription,
        ))
        .finish();

        let response = schema.execute(INTROSPECTION_QUERY).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
    }
}
