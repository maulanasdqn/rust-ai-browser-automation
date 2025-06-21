use automation_api::ACAutomationProcessor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 AC Automation Processor Demo");
    println!(
        "📋 This demonstrates parsing acceptance criteria and generating automation workflows"
    );
    println!();

    let mut processor = ACAutomationProcessor::new();

    // Sample acceptance criteria
    let criteria_text = r#"
Scenario: User Login
Given I am on the login page
When I enter valid credentials "user@example.com" and "password123"
And I click the "Login" button
Then I should be redirected to the dashboard
And I should see a welcome message "Welcome back!"
    "#;

    println!("📝 Sample Acceptance Criteria:");
    println!("{}", criteria_text);

    // Parse the criteria
    let criteria = processor.parse_acceptance_criteria(
        "demo_criteria_1".to_string(),
        "User Login Test".to_string(),
        "Authentication".to_string(),
        criteria_text.to_string(),
        vec!["login".to_string(), "auth".to_string()],
    )?;

    println!("\n✅ Parsed Acceptance Criteria:");
    println!("   ID: {}", criteria.id);
    println!("   Title: {}", criteria.title);
    println!("   Feature: {}", criteria.feature);
    println!("   Scenario: {}", criteria.scenario);
    println!("   Given steps: {:?}", criteria.given_steps);
    println!("   When steps: {:?}", criteria.when_steps);
    println!("   Then steps: {:?}", criteria.then_steps);

    // Convert to automation workflow
    let workflow = processor.convert_to_automation(&criteria.id)?;

    println!("\n🤖 Generated Automation Workflow:");
    println!("   ID: {}", workflow.id);
    println!("   Name: {}", workflow.name);
    println!("   Description: {}", workflow.description);
    println!("   Total test steps: {}", workflow.test_steps.len());

    for (i, step) in workflow.test_steps.iter().enumerate() {
        println!(
            "   Step {}: {} - {}",
            i + 1,
            step.step_type.to_uppercase(),
            step.description
        );
        println!("     Browser actions: {}", step.browser_actions.len());
        println!("     Assertions: {}", step.assertions.len());
    }

    // Generate scripts
    println!("\n🔧 Generated Scripts:");

    let mcp_script = processor.generate_browser_script(&workflow, "mcp_browser");
    println!("\n📜 MCP Browser Script:");
    println!("{}", mcp_script);

    let selenium_script = processor.generate_browser_script(&workflow, "selenium");
    println!("\n🐍 Selenium Python Script:");
    println!("{}", selenium_script);

    let playwright_script = processor.generate_browser_script(&workflow, "playwright");
    println!("\n🎭 Playwright JavaScript Script:");
    println!("{}", playwright_script);

    println!("\n✅ Demo completed successfully!");
    Ok(())
}
