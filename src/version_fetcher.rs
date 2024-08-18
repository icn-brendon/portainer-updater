use serde::Deserialize;
use anyhow::{Result, anyhow};
use log::info;
use std::env;
use dotenv::dotenv;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct DockerResponse {
    name: Option<String>,
    tags: Option<Vec<String>>,
}

pub async fn fetch_latest_version(namespace: &str, repository: &str, image_source: &str, ghcr_token: Option<&str>) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let url = match image_source {
        "dockerhub" => format!("https://hub.docker.com/v2/namespaces/{}/repositories/{}/tags", namespace, repository),
        "ghcr" => format!("https://ghcr.io/v2/{}/{}/tags/list", namespace, repository),
        _ => return Err(anyhow!("Unsupported image source")),
    };

    info!("Fetching latest version from URL: {}", url);

    let output = if image_source == "ghcr" {
        let token = env::var("GHCR_TOKEN").map_err(|_| anyhow!("GHCR token is required"))?;
        Command::new("curl")
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", token))
            .arg(&url)
            .output()
            .map_err(|e| anyhow!("Failed to execute curl: {}", e))?
    } else {
        Command::new("curl")
            .arg(&url)
            .output()
            .map_err(|e| anyhow!("Failed to execute curl: {}", e))?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to fetch tags: {}", stderr));
    }

    let raw_body = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Failed to parse curl output: {}", e))?;
    info!("Raw response body: {}", raw_body);

    // Print the raw response body for debugging
    println!("Raw response body: {}", raw_body);

    let docker_response: DockerResponse = serde_json::from_str(&raw_body)
        .map_err(|e| anyhow!("Failed to parse JSON response: {}", e))?;
    info!("Received response: {:?}", docker_response);

    // Handle the case where tags might be None
    let tags = docker_response.tags.ok_or_else(|| {
        let err_msg = "No tags found in the response";
        anyhow!(err_msg)
    })?;

    // Assuming you want to return the latest tag
    if let Some(latest_tag) = tags.first() {
        Ok(latest_tag.clone())
    } else {
        Err(anyhow!("No tags available"))
    }
}