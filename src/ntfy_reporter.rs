use anyhow::{Result, anyhow};
use log::{info, error};
use reqwest::Client;
use dotenv::dotenv;
use std::env;

pub async fn send_ntfy_report(message: &str, _breaking_change: bool) -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let ntfy_url = env::var("NTFY_URL").map_err(|_| anyhow!("NTFY URL is required"))?;
    let ntfy_topic = env::var("NTFY_TOPIC").map_err(|_| anyhow!("NTFY Topic is required"))?;
    let url = format!("https://{}/{}", ntfy_url, ntfy_topic);

    let client = Client::builder()
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let res = client.post(&url)
        .header("Title", "Log Message")
        .header("Priority", "urgent")
        .header("Tags", "info")
        .body(message.to_string())
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send POST request: {}", e))?;

    let status = res.status();
    let res_body = res.text().await.map_err(|e| anyhow!("Failed to read response body: {}", e))?;
    if status.is_success() {
        info!("NTFY report sent successfully: {}", res_body);
    } else {
        error!("Failed to send NTFY report: {} - {}", status, res_body);
        return Err(anyhow!("Failed to send NTFY report: {} - {}", status, res_body).into());
    }

    Ok(())
}