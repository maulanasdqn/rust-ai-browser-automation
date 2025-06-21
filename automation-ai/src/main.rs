use automation_ai::AIAutomationEngine;
use automation_api::AcceptanceCriteria;
use automation_integration::examples::{
    sample_form_submission_criteria, sample_login_acceptance_criteria,
    sample_shopping_cart_criteria,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 AI-Powered AC to Browser Automation");
    println!("🚀 Using OpenRouter LLM for intelligent browser control");
    println!("========================================\n");

    // Get OpenRouter API key
    let api_key = get_api_key()?;

    // Initialize AI engine
    let mut ai_engine = AIAutomationEngine::new(api_key, None)?;

    // Show available models
    println!("📋 Available AI Models:");
    let models = ai_engine.get_available_models().await?;
    for (i, model) in models.iter().enumerate() {
        println!("  {}. {}", i + 1, model);
    }

    // Model selection
    print!(
        "\nSelect model (1-{}, or Enter for default): ",
        models.len()
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if let Ok(choice) = input.trim().parse::<usize>() {
        if choice > 0 && choice <= models.len() {
            ai_engine.set_model(models[choice - 1].clone());
            println!("✅ Selected: {}", models[choice - 1]);
        }
    }

    loop {
        println!("\n🎯 Choose test scenario:");
        println!("  1. Login Test (AI Analysis)");
        println!("  2. Form Submission (AI Analysis)");
        println!("  3. Shopping Cart (AI Analysis)");
        println!("  4. Custom AC (Enter your own)");
        println!("  5. Chat with AI");
        println!("  6. Exit");

        print!("\nChoice (1-6): ");
        io::stdout().flush()?;

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        match choice.trim() {
            "1" => {
                let (title, feature, criteria_text, tags) = sample_login_acceptance_criteria();
                let criteria = create_criteria("login_1", title, feature, criteria_text, tags);
                run_ai_analysis(&mut ai_engine, &criteria).await?;
            }
            "2" => {
                let (title, feature, criteria_text, tags) = sample_form_submission_criteria();
                let criteria = create_criteria("form_1", title, feature, criteria_text, tags);
                run_ai_analysis(&mut ai_engine, &criteria).await?;
            }
            "3" => {
                let (title, feature, criteria_text, tags) = sample_shopping_cart_criteria();
                let criteria = create_criteria("cart_1", title, feature, criteria_text, tags);
                run_ai_analysis(&mut ai_engine, &criteria).await?;
            }
            "4" => {
                let criteria = get_custom_criteria()?;
                run_ai_analysis(&mut ai_engine, &criteria).await?;
            }
            "5" => {
                chat_with_ai(&mut ai_engine).await?;
            }
            "6" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => {
                println!("❌ Invalid choice. Please try again.");
            }
        }
    }

    Ok(())
}

fn get_api_key() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
        return Ok(key);
    }

    print!("🔑 Enter your OpenRouter API key: ");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;

    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("API key cannot be empty".into());
    }

    Ok(api_key)
}

fn create_criteria(
    id: &str,
    title: String,
    feature: String,
    criteria_text: String,
    tags: Vec<String>,
) -> AcceptanceCriteria {
    let mut given_steps = Vec::new();
    let mut when_steps = Vec::new();
    let mut then_steps = Vec::new();
    let mut scenario = String::new();

    let lines: Vec<&str> = criteria_text.lines().collect();
    let mut current_step_type = "";

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("Scenario:") {
            scenario = trimmed.replace("Scenario:", "").trim().to_string();
        } else if trimmed.starts_with("Given") {
            current_step_type = "given";
            given_steps.push(trimmed.replace("Given", "").trim().to_string());
        } else if trimmed.starts_with("When") {
            current_step_type = "when";
            when_steps.push(trimmed.replace("When", "").trim().to_string());
        } else if trimmed.starts_with("Then") {
            current_step_type = "then";
            then_steps.push(trimmed.replace("Then", "").trim().to_string());
        } else if trimmed.starts_with("And") || trimmed.starts_with("But") {
            let step_text = trimmed
                .replace("And", "")
                .replace("But", "")
                .trim()
                .to_string();
            match current_step_type {
                "given" => given_steps.push(step_text),
                "when" => when_steps.push(step_text),
                "then" => then_steps.push(step_text),
                _ => {}
            }
        }
    }

    AcceptanceCriteria {
        id: id.to_string(),
        title,
        feature,
        scenario,
        given_steps,
        when_steps,
        then_steps,
        tags,
        priority: "normal".to_string(),
    }
}

async fn run_ai_analysis(
    ai_engine: &mut AIAutomationEngine,
    criteria: &AcceptanceCriteria,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🤖 AI analyzing acceptance criteria...");

    print!("Execute immediately? (y/N): ");
    io::stdout().flush()?;
    let mut execute_input = String::new();
    io::stdin().read_line(&mut execute_input)?;
    let execute_immediately = execute_input.trim().to_lowercase() == "y";

    match ai_engine
        .analyze_and_execute_ac(criteria, execute_immediately)
        .await
    {
        Ok(result) => {
            println!("\n✅ AI Analysis Complete!");
            println!("📋 Plan: {}", result.plan.title);
            println!("📄 Description: {}", result.plan.description);
            println!(
                "⏱️  Estimated Duration: {:.1}s",
                result.plan.estimated_duration
            );
            println!("🔧 Steps: {}", result.plan.steps.len());
            println!("✓ Assertions: {}", result.plan.assertions.len());

            if let Some(report) = &result.execution_report {
                println!("\n📊 Execution Results:");
                println!("🎯 Success Rate: {:.1}%", report.success_rate() * 100.0);
                println!("✅ Successful: {}", report.successful_steps);
                println!("❌ Failed: {}", report.failed_steps);
            }

            println!("\n🧠 AI Reasoning:");
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result.llm_reasoning) {
                println!("{}", serde_json::to_string_pretty(&parsed)?);
            } else {
                println!("{}", result.llm_reasoning);
            }
        }
        Err(e) => {
            println!("❌ AI Analysis failed: {}", e);
        }
    }

    Ok(())
}

fn get_custom_criteria() -> Result<AcceptanceCriteria, Box<dyn std::error::Error>> {
    println!("\n📝 Enter Custom Acceptance Criteria:");

    print!("Title: ");
    io::stdout().flush()?;
    let mut title = String::new();
    io::stdin().read_line(&mut title)?;

    print!("Feature: ");
    io::stdout().flush()?;
    let mut feature = String::new();
    io::stdin().read_line(&mut feature)?;

    println!("Criteria (Gherkin format, end with empty line):");
    let mut criteria_text = String::new();
    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        if line.trim().is_empty() {
            break;
        }
        criteria_text.push_str(&line);
    }

    print!("Tags (comma-separated): ");
    io::stdout().flush()?;
    let mut tags_input = String::new();
    io::stdin().read_line(&mut tags_input)?;
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(create_criteria(
        "custom_1",
        title.trim().to_string(),
        feature.trim().to_string(),
        criteria_text,
        tags,
    ))
}

async fn chat_with_ai(
    ai_engine: &mut AIAutomationEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n💬 Chat with AI (type 'exit' to return):");

    loop {
        print!("\nYou: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.to_lowercase() == "exit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        match ai_engine.chat_with_ai(input.to_string()).await {
            Ok(response) => {
                println!("🤖 AI: {}", response);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    Ok(())
}
