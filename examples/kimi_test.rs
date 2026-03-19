//! Kimi/Moonshot API Example
//!
//! Run with: KIMI_API_KEY=your-key cargo run --example kimi_test

use reopencode::provider::{Message, OpenAiProvider, Provider, ProviderConfig};

const KIMI_BASE_URL: &str = "https://api.moonshot.cn/v1";
const KIMI_MODEL: &str = "moonshot-v1-8k";

#[tokio::main]
async fn main() {
    let api_key =
        std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable must be set");

    println!("=== Kimi/Moonshot API Integration Test ===\n");
    println!("Base URL: {}", KIMI_BASE_URL);
    println!("Model: {}\n", KIMI_MODEL);

    let config = ProviderConfig::new("kimi", api_key).with_base_url(KIMI_BASE_URL);

    let provider = OpenAiProvider::new(config);

    println!("Sending test message: 'Hello!'\n");

    let messages = vec![Message::user("Hello!")];

    match provider
        .chat(messages, KIMI_MODEL, 0.7, Some(100), &[])
        .await
    {
        Ok(response) => {
            println!("=== Response ===");
            println!("Model: {}", response.model);
            println!("Content: {}", response.content);
            println!(
                "Tokens: prompt={}, completion={}, total={}",
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens
            );
            println!("Finish Reason: {:?}", response.finish_reason);
            println!("\n=== SUCCESS: Kimi API integration works! ===");
        }
        Err(e) => {
            eprintln!("=== FAILED ===");
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
