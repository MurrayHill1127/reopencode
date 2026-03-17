//! Kimi/Moonshot API Integration Test
//!
//! This test verifies that the OpenAI provider works with Kimi API (OpenAI-compatible).
//!
//! ## Usage
//!
//! Set the environment variable and run:
//! ```bash
//! KIMI_API_KEY=your-api-key cargo test --test kimi_integration -- --nocapture
//! ```
//!
//! Or run the example binary:
//! ```bash
//! KIMI_API_KEY=your-api-key cargo run --example kimi_test
//! ```

use reopencode::provider::{Message, OpenAiProvider, Provider, ProviderConfig};

const KIMI_BASE_URL: &str = "https://api.moonshot.cn/v1";
const KIMI_MODEL: &str = "moonshot-v1-8k";

/// Test basic Kimi API connectivity with a simple message
#[tokio::test]
async fn test_kimi_api_basic_chat() {
    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable must be set");

    let config = ProviderConfig::new("kimi", api_key)
        .with_base_url(KIMI_BASE_URL);

    let provider = OpenAiProvider::new(config);

    let messages = vec![Message::user("Say 'Hello, World!' and nothing else.")];

    let result = provider
        .chat(messages, KIMI_MODEL, 0.7, Some(50), &[])
        .await;

    match result {
        Ok(response) => {
            println!("=== Kimi API Response ===");
            println!("Model: {}", response.model);
            println!("Content: {}", response.content);
            println!("Tokens: prompt={}, completion={}, total={}",
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens);
            println!("Finish Reason: {:?}", response.finish_reason);
            
            assert!(!response.content.is_empty(), "Response content should not be empty");
        }
        Err(e) => {
            panic!("API call failed: {:?}", e);
        }
    }
}

/// Test Kimi API with system message
#[tokio::test]
async fn test_kimi_api_with_system_message() {
    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable must be set");

    let config = ProviderConfig::new("kimi", api_key)
        .with_base_url(KIMI_BASE_URL);

    let provider = OpenAiProvider::new(config);

    let messages = vec![
        Message::system("You are a helpful assistant. Respond in exactly 5 words."),
        Message::user("What is the capital of France?"),
    ];

    let result = provider
        .chat(messages, KIMI_MODEL, 0.5, Some(20), &[])
        .await;

    match result {
        Ok(response) => {
            println!("=== Kimi API Response (with system) ===");
            println!("Content: {}", response.content);
            
            assert!(!response.content.is_empty(), "Response content should not be empty");
        }
        Err(e) => {
            panic!("API call failed: {:?}", e);
        }
    }
}

/// Test Kimi API streaming
#[tokio::test]
async fn test_kimi_api_streaming() {
    use futures::StreamExt;

    let api_key = std::env::var("KIMI_API_KEY")
        .expect("KIMI_API_KEY environment variable must be set");

    let config = ProviderConfig::new("kimi", api_key)
        .with_base_url(KIMI_BASE_URL);

    let provider = OpenAiProvider::new(config);

    let messages = vec![Message::user("Count from 1 to 5, one number per line.")];

    let mut stream = provider.chat_stream(messages, KIMI_MODEL, 0.7, Some(50), &[]);

    println!("=== Kimi API Streaming Response ===");
    let mut full_response = String::new();
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk);
                full_response.push_str(&chunk);
            }
            Err(e) => {
                eprintln!("\nStream error: {:?}", e);
                break;
            }
        }
    }
    println!();

    assert!(!full_response.is_empty(), "Streamed response should not be empty");
}