use anyhow::Result;
use automation_api::{AutomationWorkflow, BrowserAction, TestStep};
use automation_browser::{AutomationExecutor, VisionStrategy};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 Computer Vision Demo - AI-Powered Browser Automation");
    println!("===========================================================");

    // Get API key from environment or prompt user
    let api_key = get_api_key()?;

    // Demo different vision strategies
    demo_vision_strategies(&api_key).await?;

    println!("\n✅ Computer Vision Demo completed successfully!");
    Ok(())
}

async fn demo_vision_strategies(api_key: &str) -> Result<()> {
    println!("\n🎯 === Testing Vision Strategies ===");

    let strategies = vec![
        (
            VisionStrategy::DomInspection,
            "DOM Inspection",
            "Fast HTML analysis",
        ),
        (
            VisionStrategy::CoordinateBased,
            "Coordinate-Based",
            "Visual screenshot analysis",
        ),
        (
            VisionStrategy::Adaptive,
            "Adaptive",
            "Smart fallback approach",
        ),
    ];

    for (strategy, name, description) in strategies {
        println!("\n📊 Testing {}: {}", name, description);

        match test_vision_strategy(&api_key, strategy).await {
            Ok(()) => println!("  ✅ {} strategy completed successfully", name),
            Err(e) => println!("  ❌ {} strategy failed: {}", name, e),
        }
    }

    Ok(())
}

async fn test_vision_strategy(api_key: &str, strategy: VisionStrategy) -> Result<()> {
    let mut executor = AutomationExecutor::new()?
        .with_vision_mode(
            api_key.to_string(),
            Some("anthropic/claude-3-5-sonnet-20241022".to_string()),
        )
        .with_vision_strategy(strategy)
        .with_headless(false);

    // Create a simple test workflow
    let workflow = create_test_workflow();

    let (report, _logs) = executor.execute_workflow(&workflow).await?;

    println!("    📈 Success Rate: {:.1}%", report.success_rate() * 100.0);
    if let Some(duration) = report.get_duration() {
        println!("    ⏱️  Duration: {:.2}s", duration.as_secs_f64());
    }

    Ok(())
}

fn create_test_workflow() -> AutomationWorkflow {
    AutomationWorkflow {
        id: "computer_vision_test".to_string(),
        name: "Computer Vision Test".to_string(),
        description: "Test computer vision capabilities with Google search".to_string(),
        source_criteria: "vision_test".to_string(),
        tags: vec!["vision".to_string(), "test".to_string()],
        test_steps: vec![
            TestStep {
                step_type: "given".to_string(),
                description: "Navigate to Google".to_string(),
                browser_actions: vec![BrowserAction {
                    action_type: "navigate".to_string(),
                    url: Some("https://www.google.com".to_string()),
                    selector: None,
                    element_description: None,
                    text: None,
                    wait_condition: Some("page_load".to_string()),
                    screenshot: false,
                }],
                assertions: vec![],
            },
            TestStep {
                step_type: "when".to_string(),
                description: "Search using vision".to_string(),
                browser_actions: vec![
                    BrowserAction {
                        action_type: "type".to_string(),
                        element_description: Some("Google search input box".to_string()),
                        text: Some("computer vision automation".to_string()),
                        selector: None,
                        url: None,
                        wait_condition: Some("element_visible".to_string()),
                        screenshot: false,
                    },
                    BrowserAction {
                        action_type: "click".to_string(),
                        element_description: Some("Google search button".to_string()),
                        selector: None,
                        url: None,
                        text: None,
                        wait_condition: Some("element_clickable".to_string()),
                        screenshot: true,
                    },
                ],
                assertions: vec![],
            },
        ],
    }
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
