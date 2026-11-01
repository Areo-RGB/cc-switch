//! Parser for the MCP, prompt, and skill deep-link formats retained by Desk.

use super::DeepLinkImportRequest;
use crate::error::AppError;
use std::collections::HashMap;
use url::Url;

pub fn parse_deeplink_url(url_str: &str) -> Result<DeepLinkImportRequest, AppError> {
    let url = Url::parse(url_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid deep link URL: {e}")))?;
    if url.scheme() != "ccswitch" {
        return Err(AppError::InvalidInput(
            "Invalid deep link scheme".to_string(),
        ));
    }
    let version = url
        .host_str()
        .ok_or_else(|| AppError::InvalidInput("Missing version in URL host".to_string()))?;
    if version != "v1" || url.path() != "/import" {
        return Err(AppError::InvalidInput(
            "Unsupported deep link version or path".to_string(),
        ));
    }

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let resource = params
        .get("resource")
        .cloned()
        .ok_or_else(|| AppError::InvalidInput("Missing 'resource' parameter".to_string()))?;
    let mut request = DeepLinkImportRequest {
        version: version.to_string(),
        resource: resource.clone(),
        ..Default::default()
    };

    match resource.as_str() {
        "prompt" => {
            request.app = required(&params, "app", "prompt")?;
            request.name = required(&params, "name", "prompt")?;
            request.content = required(&params, "content", "prompt")?;
            request.description = params.get("description").cloned();
            request.enabled = parse_bool(&params, "enabled");
        }
        "mcp" => {
            request.apps = required(&params, "apps", "MCP")?;
            request.config = required(&params, "config", "MCP")?;
            request.config_format = Some("json".to_string());
            request.enabled = parse_bool(&params, "enabled");
        }
        "skill" => {
            let repo = required(&params, "repo", "skill")?.ok_or_else(|| {
                AppError::InvalidInput("Missing 'repo' parameter for skill".to_string())
            })?;
            if repo.split('/').count() != 2 || repo.starts_with('/') || repo.ends_with('/') {
                return Err(AppError::InvalidInput(format!(
                    "Invalid repo format: expected 'owner/name', got '{repo}'"
                )));
            }
            request.repo = Some(repo);
            request.directory = params.get("directory").cloned();
            request.branch = params.get("branch").cloned();
            request.enabled = parse_bool(&params, "enabled");
        }
        _ => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported resource type: {resource}"
            )))
        }
    }

    Ok(request)
}

fn required(
    params: &HashMap<String, String>,
    key: &str,
    resource: &str,
) -> Result<Option<String>, AppError> {
    params
        .get(key)
        .cloned()
        .map(Some)
        .ok_or_else(|| AppError::InvalidInput(format!("Missing '{key}' parameter for {resource}")))
}

fn parse_bool(params: &HashMap<String, String>, key: &str) -> Option<bool> {
    params.get(key).and_then(|value| value.parse().ok())
}
