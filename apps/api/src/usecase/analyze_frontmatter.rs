//! Analyze frontmatter usecase

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::usecase::{
    AnalyzeFrontmatterInputPort, FrontmatterAnalysis,
    GetMarkdownPreviewsInputData, GetMarkdownPreviewsInputPort,
    SuggestedProperty,
};

#[derive(Clone)]
pub struct AnalyzeFrontmatter {
    get_previews: Arc<dyn GetMarkdownPreviewsInputPort>,
}

impl std::fmt::Debug for AnalyzeFrontmatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeFrontmatter").finish_non_exhaustive()
    }
}

impl AnalyzeFrontmatter {
    pub fn new(
        get_previews: Arc<dyn GetMarkdownPreviewsInputPort>,
    ) -> Arc<Self> {
        Arc::new(Self { get_previews })
    }
}

#[async_trait::async_trait]
impl AnalyzeFrontmatterInputPort for AnalyzeFrontmatter {
    #[tracing::instrument(name = "AnalyzeFrontmatter::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: GetMarkdownPreviewsInputData<'a>,
    ) -> errors::Result<FrontmatterAnalysis> {
        // Get previews for all files
        let previews = self.get_previews.execute(input).await?;

        let mut property_values: HashMap<String, Vec<String>> =
            HashMap::new();
        let mut valid_files = 0;

        for preview in &previews {
            if preview.parse_error.is_none()
                && preview.frontmatter_json.is_some()
            {
                valid_files += 1;

                if let Some(ref json_str) = preview.frontmatter_json {
                    if let Ok(frontmatter) =
                        serde_json::from_str::<serde_json::Value>(json_str)
                    {
                        if let Some(obj) = frontmatter.as_object() {
                            for (key, value) in obj {
                                let value_str = match value {
                                    serde_json::Value::String(s) => {
                                        s.clone()
                                    }
                                    _ => value.to_string(),
                                };
                                property_values
                                    .entry(key.clone())
                                    .or_default()
                                    .push(value_str);
                            }
                        }
                    }
                }
            }
        }

        // Build suggested properties
        let properties: Vec<SuggestedProperty> = property_values
            .into_iter()
            .map(|(key, values)| {
                let unique: HashSet<_> = values.iter().cloned().collect();
                let unique_values: Vec<String> =
                    unique.into_iter().collect();
                let suggest_select = unique_values.len() <= 5;
                let suggested_type = if suggest_select {
                    "SELECT".to_string()
                } else {
                    "STRING".to_string()
                };

                SuggestedProperty {
                    key,
                    suggested_type,
                    unique_values,
                    suggest_select,
                }
            })
            .collect();

        Ok(FrontmatterAnalysis {
            properties,
            total_files: previews.len() as i32,
            valid_files,
        })
    }
}
