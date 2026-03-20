//! OpenAPI specification generator for ReOpenCode API

mod paths;

use utoipa::openapi::{InfoBuilder, OpenApi, OpenApiBuilder};

/// Build OpenAPI 3.1 specification for the ReOpenCode API
pub fn build_openapi() -> OpenApi {
    OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("ReOpenCode API")
                .version(env!("CARGO_PKG_VERSION"))
                .description(Some(
                    "RESTful API for ReOpenCode - an AI coding assistant written in Rust",
                ))
                .build(),
        )
        .paths(paths::build_paths())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_info() {
        let spec = build_openapi();

        assert_eq!(spec.info.title, "ReOpenCode API");
        assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
        assert!(spec.info.description.is_some());
    }

    #[test]
    fn test_paths_exist() {
        let spec = build_openapi();

        assert!(!spec.paths.paths.is_empty());
        assert!(spec.paths.paths.len() >= 40);
        assert!(spec.paths.paths.contains_key("/session"));
        assert!(spec.paths.paths.contains_key("/session/{id}"));
        assert!(spec.paths.paths.contains_key("/mcp"));
        assert!(spec.paths.paths.contains_key("/provider"));
    }

    #[test]
    fn test_unique_operation_ids() {
        let spec = build_openapi();
        let mut operation_ids: Vec<String> = Vec::new();

        for path_item in spec.paths.paths.values() {
            if let Some(ref op) = path_item.get {
                if let Some(ref id) = op.operation_id {
                    operation_ids.push(id.clone());
                }
            }
            if let Some(ref op) = path_item.post {
                if let Some(ref id) = op.operation_id {
                    operation_ids.push(id.clone());
                }
            }
            if let Some(ref op) = path_item.put {
                if let Some(ref id) = op.operation_id {
                    operation_ids.push(id.clone());
                }
            }
            if let Some(ref op) = path_item.delete {
                if let Some(ref id) = op.operation_id {
                    operation_ids.push(id.clone());
                }
            }
            if let Some(ref op) = path_item.patch {
                if let Some(ref id) = op.operation_id {
                    operation_ids.push(id.clone());
                }
            }
        }

        let unique_count = operation_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(
            operation_ids.len(),
            unique_count,
            "Duplicate operation IDs found"
        );
        assert!(
            operation_ids.len() >= 50,
            "Expected at least 50 operations, got {}",
            operation_ids.len()
        );
    }

    #[test]
    fn test_serializable() {
        let spec = build_openapi();
        let json = serde_json::to_string(&spec).expect("Failed to serialize OpenAPI spec");
        assert!(json.contains("ReOpenCode API"));
        assert!(json.contains("3.1.0"));
    }
}
