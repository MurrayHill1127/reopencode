//! Test Sisyphus Agent with Kimi API

use reopencode::agent::{Agent, Message, Role, Sisyphus, ToolDefinition};
use reopencode::provider::{OpenAiProvider, ProviderConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let api_key =
        std::env::var("KIMI_API_KEY").expect("KIMI_API_KEY environment variable must be set");

    let config = ProviderConfig::new("kimi", api_key).with_base_url("https://api.moonshot.cn/v1");

    let provider = Arc::new(OpenAiProvider::new(config));

    let agent = Sisyphus::new(provider)
        .with_model("moonshot-v1-8k")
        .with_temperature(0.7)
        .with_max_tokens(Some(100));

    println!("=== Sisyphus Agent Test with Kimi API ===\n");

    let messages = vec![Message {
        role: Role::User,
        content: "Say 'Hello from Sisyphus!' and nothing else.".to_string(),
    }];

    println!("Sending message via Agent...\n");

    match agent.execute(messages, vec![]).await {
        Ok(response) => {
            println!("=== Agent Response ===");
            println!("Content: {}", response.content);
            println!(
                "Tokens: prompt={}, completion={}, total={}",
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens
            );
            println!("\n=== SUCCESS: Agent → Provider → Kimi API works! ===");
        }
        Err(e) => {
            eprintln!("=== FAILED ===");
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
