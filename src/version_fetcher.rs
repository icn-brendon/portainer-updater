use serde::Deserialize;
use anyhow::{Result, anyhow};
use log::{info, error};
use std::env;
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

#[derive(Deserialize, Debug)]
struct GHCRResponse {
    tags: Vec<String>,
}

pub async fn fetch_latest_version(namespace: &str, repository: &str, image_source: &str, current_version: &str) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let url = match image_source {
        "dockerhub" => format!("https://hub.docker.com/v2/repositories/{}/{}/tags?page_size=5", namespace, repository),
        "ghcr" => format!("https://ghcr.io/v2/{}/{}/tags/list?n=10", namespace, repository),
        _ => return Err(anyhow!("Unsupported image source")),
    };

    info!("Fetching latest version from URL: {}", url);

    let client = Client::builder()
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let response = if image_source == "ghcr" {
        let token = env::var("GHCR_TOKEN").map_err(|_| anyhow!("GHCR token is required"))?;
        client.get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute request: {}", e))?
    } else {
        client.get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute request: {}", e))?
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        error!("Failed to fetch tags: {} - {}", status, text);
        return Err(anyhow!("Failed to fetch tags: {} - {}", status, text));
    }

    let raw_body = response.text().await.map_err(|e| anyhow!("Failed to read response body: {}", e))?;
    info!("Raw response body: {}", raw_body);

    let latest_version = if image_source == "ghcr" {
        let ghcr_response: GHCRResponse = serde_json::from_str(&raw_body)
            .map_err(|e| {
                error!("Failed to parse JSON response: {}", e);
                anyhow!("Failed to parse JSON response: {}", e)
            })?;
        info!("Received GHCR response: {:?}", ghcr_response);

        // Output the GHCR tags
        println!("GHCR tags: {:?}", ghcr_response.tags);

        // Find the tag that comes after "main"
        let main_index = ghcr_response.tags.iter().position(|tag| tag == "main");
        let latest_tag = if let Some(index) = main_index {
            if index + 1 < ghcr_response.tags.len() {
                &ghcr_response.tags[index + 1]
            } else {
                return Err(anyhow!("No tag found after 'main'"));
            }
        } else {
            return Err(anyhow!("'main' tag not found"));
        };

        latest_tag.clone()
    } else {
        let docker_response: DockerHubResponse = serde_json::from_str(&raw_body)
            .map_err(|e| {
                error!("Failed to parse JSON response: {}", e);
                anyhow!("Failed to parse JSON response: {}", e)
            })?;
        info!("Received Docker Hub response: {:?}", docker_response);

        let latest_tag = docker_response.results.iter()
            .filter(|tag| is_semver(&tag.name))
            .max_by(|a, b| compare_semver(&a.name, &b.name))
            .ok_or_else(|| {
                anyhow!("No valid semantic version tags found in the response")
            })?;

        latest_tag.name.clone()
    };

    if latest_version == current_version {
        println!("The container is up to date with version: {}", current_version);
    } else {
        println!("A new version is available: {}", latest_version);
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