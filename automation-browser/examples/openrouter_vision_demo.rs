use anyhow::Result;
use automation_api::BrowserAction;
use automation_browser::{ChromeAutomationEngine, VisionStrategy};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 OpenRouter LLM Vision Demo - Advanced Browser Automation");
    println!("===============================================================");

    // Get API key from environment or prompt user
    let api_key = get_api_key()?;

    // Demonstrate different OpenRouter models and strategies
    demo_model_comparison(&api_key).await?;

    // Show real-world automation scenarios
    demo_practical_examples(&api_key).await?;

    println!("\n✅ OpenRouter Vision Demo completed successfully!");
    Ok(())
}

async fn demo_model_comparison(api_key: &str) -> Result<()> {
    println!("\n🧠 === OpenRouter Model Comparison ===");

    let models = vec![
        (
            "anthropic/claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet (Latest)",
            "Best for HTML analysis",
        ),
        (
            "openai/gpt-4o-2024-11-20",
            "GPT-4o (Latest)",
            "Best for visual coordinates",
        ),
        (
            "openai/gpt-4o-mini-2024-07-18",
            "GPT-4o Mini",
            "Budget-friendly option",
        ),
        (
            "google/gemini-pro-1.5",
            "Gemini Pro 1.5",
            "Good alternative",
        ),
    ];

    for (model_id, name, description) in models {
        println!("\n📊 Testing {}: {}", name, description);

        let mut engine = ChromeAutomationEngine::new(false)
            .with_vision_mode(api_key.to_string(), Some(model_id.to_string()))
            .with_vision_strategy(VisionStrategy::Adaptive);

        engine.initialize().await?;

        // Test with a simple page
        let result = test_model_performance(&mut engine, model_id).await;

        match result {
            Ok(duration) => println!("  ✅ {} completed in {:.2}s", name, duration),
            Err(e) => println!("  ❌ {} failed: {}", name, e),
        }

        engine.close().await?;
        sleep(Duration::from_secs(2)).await; // Rate limiting
    }

    Ok(())
}

async fn test_model_performance(engine: &mut ChromeAutomationEngine, _model: &str) -> Result<f64> {
    let start = std::time::Instant::now();

    // Navigate to a test page
    let navigate_action = BrowserAction {
        action_type: "navigate".to_string(),
        url: Some("https://example.com".to_string()),
        selector: None,
        element_description: None,
        text: None,
        wait_condition: None,
        screenshot: false,
    };

    engine.execute_browser_action(&navigate_action).await?;

    // Test vision capabilities with element description
    let vision_action = BrowserAction {
        action_type: "click".to_string(),
        element_description: Some("More information link".to_string()),
        selector: None,
        url: None,
        text: None,
        wait_condition: None,
        screenshot: false,
    };

    let _ = engine.execute_browser_action(&vision_action).await; // May fail, that's ok for testing

    Ok(start.elapsed().as_secs_f64())
}

async fn demo_practical_examples(api_key: &str) -> Result<()> {
    println!("\n🎯 === Practical OpenRouter Vision Examples ===");

    // Demo 1: Adaptive Strategy (Recommended)
    println!("\n1️⃣ Adaptive Strategy Demo (Claude 3.5 Sonnet)");
    demo_adaptive_strategy(api_key).await?;

    // Demo 2: Coordinate-Based Strategy (GPT-4o)
    println!("\n2️⃣ Coordinate-Based Strategy Demo (GPT-4o)");
    demo_coordinate_strategy(api_key).await?;

    // Demo 3: DOM Inspection Strategy (Claude)
    println!("\n3️⃣ DOM Inspection Strategy Demo (Claude)");
    demo_dom_inspection_strategy(api_key).await?;

    // Demo 4: Model Auto-Optimization
    println!("\n4️⃣ Auto-Optimization Demo");
    demo_auto_optimization(api_key).await?;

    Ok(())
}

async fn demo_adaptive_strategy(api_key: &str) -> Result<()> {
    let mut engine = ChromeAutomationEngine::new(false)
        .with_hybrid_mode(
            api_key.to_string(),
            Some("anthropic/claude-3-5-sonnet-20241022".to_string()),
        )
        .with_vision_strategy(VisionStrategy::Adaptive);

    engine.initialize().await?;
    println!("  🚀 Using Adaptive Strategy with Claude 3.5 Sonnet");

    // Navigate to a complex page
    let actions = vec![
        BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://httpbin.org/forms/post".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "type".to_string(),
            element_description: Some("customer name input field".to_string()),
            text: Some("OpenRouter Vision Test".to_string()),
            selector: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "type".to_string(),
            element_description: Some("email input field".to_string()),
            text: Some("test@openrouter.com".to_string()),
            selector: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
    ];

    for action in actions {
        match engine.execute_browser_action(&action).await {
            Ok(result) => println!("    ✅ {}", result),
            Err(e) => println!("    ⚠️ Action failed: {}", e),
        }
    }

    engine.close().await?;
    Ok(())
}

async fn demo_coordinate_strategy(api_key: &str) -> Result<()> {
    let mut engine = ChromeAutomationEngine::new(false)
        .with_vision_mode(
            api_key.to_string(),
            Some("openai/gpt-4o-2024-11-20".to_string()),
        )
        .with_vision_strategy(VisionStrategy::CoordinateBased);

    engine.initialize().await?;
    println!("  🎯 Using Coordinate-Based Strategy with GPT-4o");

    let actions = vec![
        BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://httpbin.org/forms/post".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "click".to_string(),
            element_description: Some("submit button at the bottom".to_string()),
            selector: None,
            url: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
    ];

    for action in actions {
        match engine.execute_browser_action(&action).await {
            Ok(result) => println!("    ✅ {}", result),
            Err(e) => println!("    ⚠️ Action failed: {}", e),
        }
    }

    engine.close().await?;
    Ok(())
}

async fn demo_dom_inspection_strategy(api_key: &str) -> Result<()> {
    let mut engine = ChromeAutomationEngine::new(false)
        .with_vision_mode(
            api_key.to_string(),
            Some("anthropic/claude-3-5-sonnet-20241022".to_string()),
        )
        .with_vision_strategy(VisionStrategy::DomInspection);

    engine.initialize().await?;
    println!("  🔍 Using DOM Inspection Strategy with Claude 3.5 Sonnet");

    let actions = vec![
        BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://httpbin.org/forms/post".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        },
        BrowserAction {
            action_type: "type".to_string(),
            element_description: Some("telephone number input".to_string()),
            text: Some("555-123-4567".to_string()),
            selector: None,
            url: None,
            wait_condition: None,
            screenshot: false,
        },
    ];

    for action in actions {
        match engine.execute_browser_action(&action).await {
            Ok(result) => println!("    ✅ {}", result),
            Err(e) => println!("    ⚠️ Action failed: {}", e),
        }
    }

    engine.close().await?;
    Ok(())
}

async fn demo_auto_optimization(api_key: &str) -> Result<()> {
    println!("  ⚡ Demonstrating automatic model optimization for different strategies");

    for strategy in [
        VisionStrategy::DomInspection,
        VisionStrategy::CoordinateBased,
        VisionStrategy::Adaptive,
    ] {
        let mut engine = ChromeAutomationEngine::new(false)
            .with_vision_mode(api_key.to_string(), None)
            .with_vision_strategy(strategy.clone());

        // Auto-optimize model for strategy
        engine.with_optimal_model_for_strategy(&strategy);

        let strategy_name = match strategy {
            VisionStrategy::DomInspection => "DOM Inspection",
            VisionStrategy::CoordinateBased => "Coordinate-Based",
            VisionStrategy::Adaptive => "Adaptive",
        };

        engine.initialize().await?;
        println!("    🔧 {} strategy auto-optimized", strategy_name);

        // Quick test
        let action = BrowserAction {
            action_type: "navigate".to_string(),
            url: Some("https://example.com".to_string()),
            selector: None,
            element_description: None,
            text: None,
            wait_condition: None,
            screenshot: false,
        };

        match engine.execute_browser_action(&action).await {
            Ok(_) => println!("    ✅ {} strategy working", strategy_name),
            Err(e) => println!("    ⚠️ {} strategy issue: {}", strategy_name, e),
        }

        engine.close().await?;
    }

    Ok(())
}

fn get_api_key() -> Result<String> {
    // Try environment variable first
    if let Ok(key) = env::var("OPENROUTER_API_KEY") {
        if !key.is_empty() && !key.contains("your-key-here") {
            println!("✅ Using OpenRouter API key from environment");
            return Ok(key);
        }
    }

    // Check other common environment variables
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
