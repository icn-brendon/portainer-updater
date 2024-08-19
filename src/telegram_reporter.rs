use anyhow::{Result, anyhow};
use log::{info, error};
use reqwest::Client;
use std::env;

pub async fn send_telegram_report(message: &str) -> Result<()> {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").map_err(|_| anyhow!("TELEGRAM_BOT_TOKEN must be set"))?;
    let chat_id = env::var("TELEGRAM_CHAT_ID").map_err(|_| anyhow!("TELEGRAM_CHAT_ID must be set"))?;
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let client = Client::builder()
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

    let res = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": message,
        }))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send POST request: {}", e))?;

    let status = res.status();
    let res_body = res.text().await.map_err(|e| anyhow!("Failed to read response body: {}", e))?;
    if status.is_success() {
        info!("Telegram report sent successfully: {}", res_body);
    } else {
        error!("Failed to send Telegram report: {} - {}", status, res_body);
        return Err(anyhow!("Failed to send Telegram report: {} - {}", status, res_body));
    }

    Ok(())
}