use anyhow::Result;
use automation_api::{AutomationWorkflow, BrowserAction, TestStep};
use automation_browser::{AutomationExecutor, VisionStrategy};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Robust Google Search with Computer Vision");
    println!("===========================================");

    let api_key = get_api_key()?;

    // Create robust Google search workflow
    let workflow = create_robust_google_workflow();

    // Execute with adaptive vision strategy for maximum robustness
    let mut executor = AutomationExecutor::new()?
        .with_hybrid_mode(
            api_key,
            Some("anthropic/claude-3-5-sonnet-20241022".to_string()),
        )
        .with_vision_strategy(VisionStrategy::Adaptive)
        .with_headless(false);

    match executor.execute_workflow(&workflow).await {
        Ok((report, logs)) => {
            println!("\n📊 === Execution Results ===");
            println!("🎯 Workflow: {}", report.workflow_name);
            println!("📈 Success Rate: {:.1}%", report.success_rate() * 100.0);
            println!("✅ Successful Steps: {}", report.successful_steps);
            println!("❌ Failed Steps: {}", report.failed_steps);

            if let Some(duration) = report.get_duration() {
                println!("⏱️  Duration: {:.2}s", duration.as_secs_f64());
            }

            // Print detailed results
            report.print_summary();

            // Show some vision-specific logs
            println!("\n🧠 Computer Vision Logs:");
            for log in logs
                .iter()
                .filter(|l| l.step_type == Some("vision".to_string()))
                .take(5)
            {
                println!("  [{}] {}: {}", log.timestamp, log.level, log.message);
            }
        }
        Err(e) => {
            println!("❌ Execution failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn create_robust_google_workflow() -> AutomationWorkflow {
    AutomationWorkflow {
        id: "robust_google_search".to_string(),
        name: "Robust Google Search with Vision".to_string(),
        description: "Enhanced Google search using computer vision with fallbacks".to_string(),
        source_criteria: "google_search_test".to_string(),
        tags: vec![
            "google".to_string(),
            "search".to_string(),
            "vision".to_string(),
        ],
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
                description: "Handle blocking elements".to_string(),
                browser_actions: vec![BrowserAction {
                    action_type: "wait".to_string(),
                    wait_condition: Some("page_stable".to_string()),
                    selector: None,
                    element_description: None,
                    url: None,
                    text: None,
                    screenshot: false,
                }],
                assertions: vec![],
            },
            TestStep {
                step_type: "when".to_string(),
                description: "Perform search using computer vision".to_string(),
                browser_actions: vec![
                    BrowserAction {
                        action_type: "type".to_string(),
                        element_description: Some("Google search input box".to_string()),
                        text: Some("rust browser automation computer vision".to_string()),
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
                        screenshot: false,
                    },
                    BrowserAction {
                        action_type: "wait".to_string(),
                        wait_condition: Some("page_stable".to_string()),
                        selector: None,
                        element_description: None,
                        url: None,
                        text: None,
                        screenshot: false,
                    },
                ],
                assertions: vec![],
            },
            TestStep {
                step_type: "then".to_string(),
                description: "Verify search results".to_string(),
                browser_actions: vec![BrowserAction {
                    action_type: "screenshot".to_string(),
                    selector: None,
                    element_description: None,
                    url: None,
                    text: None,
                    wait_condition: None,
                    screenshot: true,
                }],
                assertions: vec![
                    "search_results_visible".to_string(), // Use the enhanced assertion
                    "page_content_contains: results".to_string(),
                ],
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

    // Check alternative environment variables
    for env_var in ["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "VISION_API_KEY"] {
        if let Ok(key) = env::var(env_var) {
            if !key.is_empty() {
                println!("✅ Using API key from {} environment variable", env_var);
                return Ok(key);
            }
        }
    }

    println!("\n❌ No API key found!");
    println!("\nRequired environment variables (in order of preference):");
    println!("- OPENROUTER_API_KEY (preferred)");
    println!("- OPENAI_API_KEY");
    println!("- ANTHROPIC_API_KEY");
    println!("- VISION_API_KEY");
    println!("\nExample: export OPENROUTER_API_KEY=sk-or-v1-your-key-here");

    Err(anyhow::anyhow!(
        "API key required for computer vision functionality"
    ))
}
