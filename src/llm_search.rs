use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use serde_json::Value;
use std::env;
use std::time::Duration;

pub async fn llm_search(current_version: &str, latest_version: &str, image_source: &str, namespace: &str, repository: &str, safe_update_check: bool) -> Result<String, Box<dyn std::error::Error>> {
    let api_url = env::var("OPEN_WEBUI_URL").expect("OPEN_WEBUI_URL not set");
    let api_key = env::var("OLLAMA_API").expect("OLLAMA_API not set");

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    let container_url = construct_container_url(image_source, namespace, repository)?;

    let search_query = format!("Upgrading {} from {} to {} release notes, upgrade notes, breaking changes list issues and problems", container_url, current_version, latest_version);

    let search_response_body = match perform_web_search(&client, &api_url, &api_key, &search_query).await {
        Ok(response) => response,
        Err(e) => {
            if safe_update_check {
                return Err(Box::from(format!("Failed to connect to LLM host: {}", e)));
            } else {
                return Ok(String::from("LLM search failed, proceeding with upgrade"));
            }
        }
    };

    let (collection_name, urls) = extract_search_results(&search_response_body);

    let chat_message_body = construct_chat_message_body(&search_query, &collection_name, &urls, current_version, latest_version);
    let assistant_content = send_chat_message(&client, &api_url, &api_key, &chat_message_body).await?;

    Ok(assistant_content)
}

fn construct_container_url(image_source: &str, namespace: &str, repository: &str) -> Result<String, Box<dyn std::error::Error>> {
    match image_source {
        "dockerhub" => Ok(format!("https://hub.docker.com/r/{}/{}", namespace, repository)),
        "ghcr" => Ok(format!("https://ghcr.io/{}/{}", namespace, repository)),
        _ => Err(Box::from("Unsupported image source")),
    }
}

async fn perform_web_search(client: &Client, api_url: &str, api_key: &str, search_query: &str) -> Result<Value, Box<dyn std::error::Error>> {
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

    Ok(search_response.json().await?)
}

fn extract_search_results(search_response_body: &Value) -> (String, Vec<String>) {
    let collection_name = search_response_body["collection_name"].as_str().unwrap_or("").to_string();
    let urls = search_response_body["filenames"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|url| url.as_str().map(String::from))
        .collect();

    (collection_name, urls)
}

fn construct_chat_message_body(search_query: &str, collection_name: &str, urls: &[String], current_version: &str, latest_version: &str) -> Value {
    json!({
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
            "content": format!("We are upgrading a container from version {} to {}. Please make the entire output in a clean readable format for mobile users for notification purposes, there is no response that will happen to this make sure all the information user might require is in one chat. Ensure that the upgrade process between versions doesn't have any major issues or requirements as user might be skipping updates and going from older version, providing only the necessary information.\n\nFormatting standard as:\nNew version: {}\nSafe to Upgrade? Yes/No\nSummary of impact:\n\nConfirm whether there are any breaking changes between the versions. If there are breaking changes, list them in the summary. Do not hesitate to get to the point; minor changes can be listed as safe if no user actions are required.", current_version, latest_version, latest_version)
        }]
    })
}

async fn send_chat_message(client: &Client, api_url: &str, api_key: &str, chat_message_body: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let chat_message_response = client.post(&format!("{}/ollama/api/chat", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header(CONTENT_TYPE, "application/json")
        .json(chat_message_body)
        .send()
        .await?;

    let chat_message_response_body: Value = chat_message_response.json().await?;
    Ok(chat_message_response_body["message"]["content"].as_str().unwrap_or("No content found").to_string())
}