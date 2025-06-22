use anyhow::Result;
use automation_api::BrowserAction;
use automation_browser::{ChromeAutomationEngine, VisionStrategy};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Enhanced Google Search with OpenRouter Vision");
    println!("===============================================");

    // Get API key
    let api_key = get_api_key()?;

    // Demo different approaches to Google search
    demo_vision_google_search(&api_key).await?;
    demo_robust_assertion(&api_key).await?;

    println!("\n✅ Enhanced Google Search demo completed!");
    Ok(())
}

async fn demo_vision_google_search(api_key: &str) -> Result<()> {
    println!("\n👁️ === Vision-Based Google Search ===");

    let mut engine = ChromeAutomationEngine::new(false)
        .with_vision_mode(
            api_key.to_string(),
            Some("anthropic/claude-3-5-sonnet-20241022".to_string()),
        )
        .with_vision_strategy(VisionStrategy::Adaptive);

    engine.initialize().await?;
    println!("✅ Vision engine initialized with Claude 3.5 Sonnet");

    let actions = vec![
        BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://www.google.com".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "wait".to_string(),
            wait_condition: Some("page_stable".to_string()),
            selector: None,
            element_description: None,
            text: None,
            url: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "type".to_string(),
            element_description: Some("Google search input box".to_string()),
            text: Some("OpenRouter AI vision automation".to_string()),
            selector: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "click".to_string(),
            element_description: Some("Google search button".to_string()),
            selector: None,
            url: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "wait".to_string(),
            wait_condition: Some("page_stable".to_string()),
            selector: None,
            element_description: None,
            text: None,
            url: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "screenshot".to_string(),
            selector: None,
            element_description: None,
            text: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
    ];

    for (i, action) in actions.iter().enumerate() {
        println!("  📋 Step {}: {}", i + 1, action.action_type);
        match engine.execute_browser_action(action).await {
            Ok(result) => println!("    ✅ {}", result),
            Err(e) => println!("    ❌ Failed: {}", e),
        }
    }

    // Test the enhanced assertion logic
    println!("\n🔍 Testing enhanced search verification...");
    match engine.execute_assertion("search_results_visible").await {
        Ok(_) => println!("  ✅ Search results verified successfully"),
        Err(e) => println!("  ⚠️ Search verification failed: {}", e),
    }

    engine.close().await?;
    Ok(())
}

async fn demo_robust_assertion(api_key: &str) -> Result<()> {
    println!("\n🛡️ === Robust Assertion Testing ===");

    let mut engine = ChromeAutomationEngine::new(false)
        .with_vision_mode(
            api_key.to_string(),
            Some("openai/gpt-4o-2024-11-20".to_string()),
        )
        .with_vision_strategy(VisionStrategy::CoordinateBased);

    engine.initialize().await?;
    println!("✅ Engine initialized with GPT-4o for coordinate-based vision");

    // Quick search
    let actions = vec![
        BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://www.google.com".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "type".to_string(),
            element_description: Some("search input field".to_string()),
            text: Some("rust browser automation".to_string()),
            selector: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "click".to_string(),
            element_description: Some("search button".to_string()),
            selector: None,
            url: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
    ];

    for action in &actions {
        match engine.execute_browser_action(action).await {
            Ok(_) => println!("  ✅ Action succeeded"),
            Err(e) => println!("  ⚠️ Action failed: {}", e),
        }
    }

    // Test different assertion approaches
    println!("\n🧪 Testing different assertion methods:");

    // Test 1: Traditional URL check (may fail on modern Google)
    println!("  1️⃣ Traditional URL check:");
    match engine.execute_assertion("url_contains: search").await {
        Ok(_) => println!("    ✅ URL contains 'search'"),
        Err(e) => println!("    ⚠️ URL check failed: {}", e),
    }

    // Test 2: Enhanced search results check
    println!("  2️⃣ Enhanced search results check:");
    match engine.execute_assertion("search_results_visible").await {
        Ok(_) => println!("    ✅ Search results are visible"),
        Err(e) => println!("    ❌ Search results check failed: {}", e),
    }

    // Test 3: Page content check
    println!("  3️⃣ Page content verification:");
    match engine
        .execute_assertion("page_content_contains: results")
        .await
    {
        Ok(_) => println!("    ✅ Page contains 'results'"),
        Err(e) => println!("    ⚠️ Content check failed: {}", e),
    }

    engine.close().await?;
    Ok(())
}

fn get_api_key() -> Result<String> {
    if let Ok(key) = env::var("OPENROUTER_API_KEY") {
        if !key.is_empty() && !key.contains("your-key-here") {
            println!("✅ Using OpenRouter API key from environment");
            return Ok(key);
        }
    }

    // Check other environment variables
    for env_var in ["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "VISION_API_KEY"] {
        if let Ok(key) = env::var(env_var) {
            if !key.is_empty() {
                println!("✅ Using API key from {} environment variable", env_var);
                return Ok(key);
            }
        }
    }

    println!("\n❌ No OpenRouter API key found!");
    println!("\nTo run this demo, you need an OpenRouter API key:");
    println!("1. Sign up at https://openrouter.ai");
    println!("2. Get your API key from the dashboard");
    println!("3. Set it as an environment variable:");
    println!("   Windows: set OPENROUTER_API_KEY=sk-or-v1-your-key-here");
    println!("   Unix: export OPENROUTER_API_KEY=sk-or-v1-your-key-here");

    Err(anyhow::anyhow!("OpenRouter API key required"))
}
