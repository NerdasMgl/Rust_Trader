use reqwest::Client;
use serde_json::json;
use std::env;
use dotenvy::dotenv;
use std::time::Instant;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let api_key = env::var("VOLC_API_KEY").expect("VOLC_API_KEY not set");
    let endpoint = env::var("VOLC_ENDPOINT").expect("VOLC_ENDPOINT not set");
    let model = env::var("VOLC_MODEL").expect("VOLC_MODEL not set");

    // 使用和 http_client.rs 一样的配置
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(60))
        .http1_only() // 关键：测试 HTTP/1.1
        .pool_max_idle_per_host(0)
        .no_proxy()
        .build()
        .unwrap();

    let url = format!("{}/embeddings", endpoint.trim_end_matches('/'));
    println!("🔗 Testing Connection to: {}", url);
    println!("🔑 Using Model: {}", model);

    let body = json!({
        "model": model,
        "input": "Hello, is this memory working?",
        "encoding_format": "float"
    });

    let start = Instant::now();
    match client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await 
    {
        Ok(resp) => {
            let duration = start.elapsed();
            println!("⏱️  Time taken: {:.2?}", duration);
            println!("📡 Status: {}", resp.status());
            if resp.status().is_success() {
                println!("✅ Success! RAG Network is healthy.");
            } else {
                println!("❌ Failed: {}", resp.text().await.unwrap_or_default());
            }
        }
        Err(e) => {
            println!("🔥 Network Error: {}", e);
            if e.is_timeout() {
                println!("   (The request timed out. HK->CN latency is too high)");
            }
            if e.is_connect() {
                println!("   (Could not connect. DNS or Firewall issue)");
            }
        }
    }
}