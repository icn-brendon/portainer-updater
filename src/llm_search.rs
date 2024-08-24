use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use serde_json::Value;
use std::env;
use std::time::Duration;

pub async fn llm_search(current_version: &str, latest_version: &str, image_source: &str, namespace: &str, repository: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Load environment variables
    let api_url = env::var("OPEN_WEBUI_URL").expect("OPEN_WEBUI_URL not set");
    let api_key = env::var("OLLAMA_API").expect("OLLAMA_API not set");

    // Create a new HTTP client with a timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(120)) // Set timeout to 2 minutes
        .build()?;

    // Construct the container URL based on the image source
    let container_url = match image_source {
        "dockerhub" => format!("https://hub.docker.com/r/{}/{}", namespace, repository),
        "ghcr" => format!("https://ghcr.io/{}/{}", namespace, repository),
        _ => return Err(Box::from("Unsupported image source")),
    };

    // Construct the search query string
    let search_query = format!("Upgrading {} from {} to {} release notes or breaking changes requirements", container_url, current_version, latest_version );

    // Step 1: Perform a web search with increased timeout
    let search_body = json!({
        "collection_name": "",
        "query": search_query
    });

    let search_response = client.post(&format!("{}/rag/api/v1/web/search", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header(CONTENT_TYPE, "application/json")
        .json(&search_body)
        .send()
        .await?;

    let search_response_body: Value = search_response.json().await?;

    // Extract collection name and URLs from search response
    let collection_name = search_response_body["collection_name"].as_str().unwrap_or("");
    let urls: Vec<String> = search_response_body["filenames"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|url| url.as_str().map(String::from))
        .collect();

    // Step 2: Send a chat message with URLs
    let chat_message_body = json!({
        "stream": false,
        "model": "llama3.1:8b",
        "options": {},
        "files": [{
            "collection_name": collection_name,
            "name": search_query,
            "type": "web_search_results",
            "urls": urls
        }],
        "messages": [{
            "role": "user",
            "content": format!("We are upgrading a container from version {} to {}. Please make entire output in clean readable format for mobile users for notification purposes, ensure that the upgrade process between versions don't have any major issues or requirements, providing only the necessary information.\n\nFormatting standard as:\nNamespace: {}\nNew version: {}\nSafe to Upgrade? Yes/No\nSummary of impact:\n\nConfirm whether there are any breaking changes by providing a true or false statement based on the web search query. If there are breaking changes, list them in the summary. Do not hesitate to get to the point; minor changes can be listed as safe if no user actions are required.", current_version, latest_version, namespace, latest_version)
        }]
    });

    let chat_message_response = client.post(&format!("{}/ollama/api/chat", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header(CONTENT_TYPE, "application/json")
        .json(&chat_message_body)
        .send()
        .await?;

    let chat_message_response_body: Value = chat_message_response.json().await?;
    let assistant_content = chat_message_response_body["message"]["content"].as_str().unwrap_or("No content found");

    Ok(assistant_content.to_string())
}