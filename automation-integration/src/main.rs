use automation_integration::{ACAutomationIntegration, examples};
use tokio::io::{self, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Welcome to AC (Acceptance Criteria) to Automation Converter!");
    println!("🔧 This system converts QA Acceptance Criteria to browser automation");
    println!("📋 Powered by Rust - Simplified Demo Version\n");

    let mut integration = ACAutomationIntegration::new()?;

    loop {
        println!("🔄 Choose an option:");
        println!("1. Run sample Login acceptance criteria");
        println!("2. Run sample Contact Form acceptance criteria");
        println!("3. Run sample Shopping Cart acceptance criteria");
        println!("4. Enter custom acceptance criteria");
        println!("5. View all workflows");
        println!("6. Exit");
        print!("Enter your choice (1-6): ");

        let mut input = String::new();
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);
        reader.read_line(&mut input).await?;
        let choice = input.trim();

        match choice {
            "1" => {
                let (title, feature, criteria_text, tags) =
                    examples::sample_login_acceptance_criteria();
                run_pipeline(&mut integration, title, feature, criteria_text, tags).await?;
            }
            "2" => {
                let (title, feature, criteria_text, tags) =
                    examples::sample_form_submission_criteria();
                run_pipeline(&mut integration, title, feature, criteria_text, tags).await?;
            }
            "3" => {
                let (title, feature, criteria_text, tags) =
                    examples::sample_shopping_cart_criteria();
                run_pipeline(&mut integration, title, feature, criteria_text, tags).await?;
            }
            "4" => {
                println!("📝 Enter custom acceptance criteria:");
                print!("Title: ");
                let mut title = String::new();
                reader.read_line(&mut title).await?;
                let title = title.trim().to_string();

                print!("Feature: ");
                let mut feature = String::new();
                reader.read_line(&mut feature).await?;
                let feature = feature.trim().to_string();

                println!(
                    "Acceptance Criteria Text (paste your Gherkin or plain text, press Enter twice when done):"
                );
                let mut criteria_text = String::new();
                let mut empty_lines = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).await?;
                    if line.trim().is_empty() {
                        empty_lines += 1;
                        if empty_lines >= 2 {
                            break;
                        }
                    } else {
                        empty_lines = 0;
                    }
                    criteria_text.push_str(&line);
                }

                print!("Tags (comma-separated): ");
                let mut tags_input = String::new();
                reader.read_line(&mut tags_input).await?;
                let tags: Vec<String> = tags_input
                    .trim()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                run_pipeline(&mut integration, title, feature, criteria_text, tags).await?;
            }
            "5" => {
                println!("📋 Viewing all workflows...");
                let workflows = integration.get_all_workflows();
                if workflows.is_empty() {
                    println!(
                        "📭 No workflows found. Create some by running acceptance criteria conversion first."
                    );
                } else {
                    println!("📊 Found {} workflow(s):", workflows.len());
                    for (i, workflow) in workflows.iter().enumerate() {
                        println!("{}. {} (ID: {})", i + 1, workflow.name, workflow.id);
                        println!("   Description: {}", workflow.description);
                        println!("   Steps: {}", workflow.test_steps.len());
                        println!("   Tags: {:?}", workflow.tags);
                        println!();
                    }
                }
            }
            "6" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => {
                println!("❌ Invalid choice. Please select 1-6.");
            }
        }

        println!("\n{}\n", "=".repeat(60));
    }

    Ok(())
}

async fn run_pipeline(
    integration: &mut ACAutomationIntegration,
    title: String,
    feature: String,
    criteria_text: String,
    tags: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Processing: {}", title);

    println!("🔄 Choose execution mode:");
    println!("1. Convert to automation workflow only (no execution)");
    println!("2. Convert and execute simulation");
    print!("Enter choice (1-2): ");

    let mut choice = String::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader.read_line(&mut choice).await?;
    let execute_immediately = choice.trim() == "2";

    match integration
        .full_ac_to_automation_pipeline(title, feature, criteria_text, tags, execute_immediately)
        .await
    {
        Ok(result) => {
            result.print_summary();

            if execute_immediately {
                if let Some(report) = &result.execution_report {
                    report.print_summary();
                }
            }

            println!("\n📁 Generated Outputs:");
            println!("- Workflow JSON: Available in result");
            println!("- Parsed AC: Available in result");

            if !execute_immediately {
                println!(
                    "\n💡 To execute this workflow later, use the workflow ID: {}",
                    result.workflow.id
                );
            }
        }
        Err(e) => {
            println!("❌ Pipeline failed: {}", e);
        }
    }

    Ok(())
}
