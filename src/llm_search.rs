use std::env;
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    let search_url = env::var("WEB_SEARCH").expect("WEB_SEARCH not set");
    let api_url = env::var("OPEN_WEBUI_URL").expect("OPEN_WEBUI_URL not set");
    let api_key = env::var("OLLAMA_API").expect("OLLAMA_API not set");

    // Ensure the search URL ends with a slash
    let search_url = if search_url.ends_with('/') {
        search_url
    } else {
        format!("{}/", search_url)
    };

    // Create a new HTTP client
    let client = Client::new();

    // Step 1: POST to /rag/api/v1/web/search
    let search_query = "Home Assistant 2020";
    let search_body = json!({
        "collection_name": "test",
        "query": format!("{}search?q={}", search_url, search_query)
    });

    let search_response = client.post(&format!("{}/rag/api/v1/web/search", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&search_body)
        .send()
        .await?;

    // Print the status code and headers for debugging
    println!("Search Status: {}", search_response.status());
    println!("Search Headers:\n{:#?}", search_response.headers());

    // Step 2: POST to /rag/api/v1/query/collection
    let collection_query = "Home Assistant 2020"; // Replace with actual query if needed
    let collection_body = json!({
        "collection_names": ["test"],
        "query": collection_query,
        "k": null,
        "r": null,
        "hybrid": true
    });

    let collection_response = client.post(&format!("{}/rag/api/v1/query/collection", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&collection_body)
        .send()
        .await?;

    // Print the status code and headers for debugging
    println!("Collection Query Status: {}", collection_response.status());
    println!("Collection Query Headers:\n{:#?}", collection_response.headers());

    // Step 3: Extract data from collection response
    let collection_data: serde_json::Value = collection_response.json().await?;
    println!("Collection Data: {}", collection_data);

    // Step 4: Format the data to match the required structure for sending to the generate API
    let documents = &collection_data["documents"];
    let content = documents.as_array().unwrap().iter().map(|doc| doc.to_string()).collect::<Vec<String>>().join("\n");

    let generate_body = json!({
        "model": "llama3.1:8b",
        "prompt": content,
        "images": [],
        "format": "text",
        "options": {},
        "system": "",
        "template": "",
        "context": "",
        "stream": true,
        "raw": true,
        "keep_alive": 0
    });

    // Print the generate body for debugging
    println!("Generate Body: {}", generate_body);

    // Step 5: POST to the generate API endpoint with the formatted data
    let generate_url = format!("{}/api/generate", api_url);
    let generate_response = client.post(&generate_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&generate_body)
        .send()
        .await?;

    // Print the status code, headers, and body for debugging
    println!("Generate Status: {}", generate_response.status());
    println!("Generate Headers:\n{:#?}", generate_response.headers());
    let generate_response_text = generate_response.text().await?;
    println!("Generate Response Body: {}", generate_response_text);

    Ok(())
}