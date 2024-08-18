use serde::Deserialize;
use anyhow::{Result, anyhow};
use log::{info, error};
use dotenv::dotenv;
use reqwest::Client;

#[derive(Deserialize, Debug)]
struct DockerHubTag {
    name: String,
}

#[derive(Deserialize, Debug)]
struct DockerHubResponse {
    results: Vec<DockerHubTag>,
}

pub async fn fetch_latest_version(namespace: &str, repository: &str, image_source: &str, current_version: &str) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let url = match image_source {
        "dockerhub" => format!("https://hub.docker.com/v2/repositories/{}/{}/tags?page_size=100", namespace, repository),
        "ghcr" => format!("https://ghcr.io/v2/{}/{}/tags/list?n=10", namespace, repository),
        _ => return Err(anyhow!("Unsupported image source")),
    };

    info!("Fetching latest version from URL: {}", url);

    let client = Client::builder()
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to execute request: {}", e))?;

    let rate_limit = response.headers().get("X-RateLimit-Limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let rate_limit_remaining = response.headers().get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let rate_limit_reset = response.headers().get("X-RateLimit-Reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    info!("Rate limit: {}", rate_limit);
    info!("Rate limit remaining: {}", rate_limit_remaining);
    info!("Rate limit reset: {}", rate_limit_reset);

    if response.status() == 429 {
        let retry_after = response.headers().get("X-Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        error!("Rate limit exceeded. Retry after: {}", retry_after);
        return Err(anyhow!("Rate limit exceeded. Retry after: {}", retry_after));
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        error!("Failed to fetch tags: {} - {}", status, text);
        return Err(anyhow!("Failed to fetch tags: {} - {}", status, text));
    }

    let raw_body = response.text().await.map_err(|e| anyhow!("Failed to read response body: {}", e))?;
    info!("Raw response body: {}", raw_body);

    let docker_response: DockerHubResponse = serde_json::from_str(&raw_body)
        .map_err(|e| {
            error!("Failed to parse JSON response: {}", e);
            anyhow!("Failed to parse JSON response: {}", e)
        })?;
    info!("Received Docker Hub response: {:?}", docker_response);

    let valid_tags: Vec<&DockerHubTag> = docker_response.results.iter()
        .filter(|tag| {
            let stripped_version = strip_suffix(&tag.name);
            info!("Checking if '{}' is a valid semantic version", stripped_version);
            is_semver(stripped_version)
        })
        .collect();

    // Log the valid tags found
    info!("Valid semantic version tags found: {:?}", valid_tags);

    if valid_tags.is_empty() {
        error!("No valid semantic version tags found in the response");
        return Err(anyhow!("No valid semantic version tags found in the response"));
    }

    let latest_tag = valid_tags.iter()
        .max_by(|a, b| {
            let a_stripped = strip_suffix(&a.name);
            let b_stripped = strip_suffix(&b.name);
            info!("Comparing '{}' with '{}'", a_stripped, b_stripped);
            compare_semver(a_stripped, b_stripped)
        })
        .ok_or_else(|| {
            anyhow!("No valid semantic version tags found in the response")
        })?;

    let latest_version = latest_tag.name.clone();

    if latest_version == current_version {
        println!("The container is up to date with version: {} (repository: {}/{})", current_version, namespace, repository);
    } else {
        println!("A new version is available: {} (repository: {}/{})", latest_version, namespace, repository);
    }

    Ok(latest_version)
}

fn is_semver(version: &str) -> bool {
    version.split('.').all(|part| part.parse::<u64>().is_ok())
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u64> = a.split('.').map(|part| part.parse().unwrap()).collect();
    let b_parts: Vec<u64> = b.split('.').map(|part| part.parse().unwrap()).collect();
    a_parts.cmp(&b_parts)
}

fn strip_suffix(version: &str) -> &str {
    let version = version.strip_prefix('v').unwrap_or(version);
    version.split('-').next().unwrap_or(version)
}