pub mod chrome;

use anyhow::Result;
use automation_api::{AutomationWorkflow, BrowserAction, TestStep};
use std::time::{Duration, SystemTime};

pub use chrome::*;

#[derive(Debug, Clone)]
pub struct AutomationExecutor {
    verbose: bool,
}

impl AutomationExecutor {
    pub fn new() -> Result<Self> {
        Ok(Self { verbose: true })
    }

    pub async fn execute_workflow(
        &mut self,
        workflow: &AutomationWorkflow,
    ) -> Result<ExecutionReport> {
        let mut report = ExecutionReport::new(&workflow.id, &workflow.name);

        if self.verbose {
            println!("🚀 Starting execution of workflow: {}", workflow.name);
        }
        report.start_execution();

        for (step_index, step) in workflow.test_steps.iter().enumerate() {
            if self.verbose {
                println!(
                    "📋 Step {}: {} - {}",
                    step_index + 1,
                    step.step_type.to_uppercase(),
                    step.description
                );
            }

            let step_result = self.execute_test_step(step).await;
            match step_result {
                Ok(actions_executed) => {
                    report.add_step_success(step, actions_executed);
                    if self.verbose {
                        println!("✅ Step {} completed successfully", step_index + 1);
                    }
                }
                Err(e) => {
                    report.add_step_failure(step, e.to_string());
                    if self.verbose {
                        println!("❌ Step {} failed: {}", step_index + 1, e);
                    }

                    // Continue execution for now, but mark as failed
                    // In production, you might want to stop on failure
                }
            }
        }

        report.end_execution();
        if self.verbose {
            println!("🏁 Workflow execution completed");
        }

        Ok(report)
    }

    async fn execute_test_step(&mut self, step: &TestStep) -> Result<Vec<String>> {
        let mut executed_actions = Vec::new();

        for action in &step.browser_actions {
            match self.execute_browser_action(action).await {
                Ok(action_result) => {
                    executed_actions.push(format!("{}: {}", action.action_type, action_result));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Action '{}' failed: {}",
                        action.action_type,
                        e
                    ));
                }
            }
        }

        // Execute assertions
        for assertion in &step.assertions {
            match self.execute_assertion(assertion).await {
                Ok(_) => {
                    executed_actions.push(format!("assertion: {} - PASSED", assertion));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Assertion '{}' failed: {}", assertion, e));
                }
            }
        }

        Ok(executed_actions)
    }

    async fn execute_browser_action(&mut self, action: &BrowserAction) -> Result<String> {
        if self.verbose {
            println!("  🔧 Executing action: {}", action.action_type);
        }

        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.url {
                    // Simulate navigation
                    self.simulate_wait("navigation", 1000).await?;
                    Ok(format!("Navigated to {}", url))
                } else {
                    Err(anyhow::anyhow!("Navigate action requires URL"))
                }
            }
            "click" => {
                if let Some(selector) = &action.selector {
                    // Simulate click
                    self.simulate_wait("click", 500).await?;
                    Ok(format!("Clicked element {}", selector))
                } else {
                    Err(anyhow::anyhow!("Click action requires selector"))
                }
            }
            "type" => {
                if let Some(selector) = &action.selector {
                    if let Some(text) = &action.text {
                        // Simulate typing
                        self.simulate_wait("typing", 300 * text.len() as u64)
                            .await?;
                        Ok(format!("Typed '{}' into {}", text, selector))
                    } else {
                        Err(anyhow::anyhow!("Type action requires text"))
                    }
                } else {
                    Err(anyhow::anyhow!("Type action requires selector"))
                }
            }
            "select" => {
                if let Some(selector) = &action.selector {
                    if let Some(value) = &action.text {
                        // Simulate selection
                        self.simulate_wait("selection", 400).await?;
                        Ok(format!("Selected '{}' in {}", value, selector))
                    } else {
                        Err(anyhow::anyhow!("Select action requires value"))
                    }
                } else {
                    Err(anyhow::anyhow!("Select action requires selector"))
                }
            }
            "wait" => {
                if let Some(condition) = &action.wait_condition {
                    match condition.as_str() {
                        "page_load" => {
                            self.simulate_wait("page load", 2000).await?;
                            Ok("Waited for page load".to_string())
                        }
                        "page_stable" => {
                            self.simulate_wait("page stability", 500).await?;
                            Ok("Waited for page stability".to_string())
                        }
                        "element_visible" => {
                            self.simulate_wait("element visibility", 1000).await?;
                            Ok("Waited for element to be visible".to_string())
                        }
                        _ => {
                            self.simulate_wait("custom condition", 1000).await?;
                            Ok(format!("Waited for condition: {}", condition))
                        }
                    }
                } else {
                    self.simulate_wait("default wait", 1000).await?;
                    Ok("Waited for default duration".to_string())
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unknown action type: {}",
                action.action_type
            )),
        }?;

        if action.screenshot {
            if self.verbose {
                println!("  📸 Taking screenshot");
            }
            self.simulate_wait("screenshot", 200).await?;
        }

        Ok(format!("Executed {} action", action.action_type))
    }

    async fn execute_assertion(&mut self, assertion: &str) -> Result<()> {
        if self.verbose {
            println!("  ✓ Checking assertion: {}", assertion);
        }

        let parts: Vec<&str> = assertion.split(": ").collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid assertion format: {}", assertion));
        }

        let assertion_type = parts[0];
        let assertion_value = parts[1];

        // Simulate assertion checking
        self.simulate_wait("assertion check", 300).await?;

        match assertion_type {
            "element_visible" => {
                // Simulate checking if element is visible (90% success rate for demo)
                if rand_success(0.9) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Element '{}' is not visible",
                        assertion_value
                    ))
                }
            }
            "element_not_visible" => {
                // Simulate checking if element is not visible (95% success rate for demo)
                if rand_success(0.95) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Element '{}' should not be visible",
                        assertion_value
                    ))
                }
            }
            "url_contains" => {
                // Simulate URL check (85% success rate for demo)
                if rand_success(0.85) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Current URL does not contain '{}'",
                        assertion_value
                    ))
                }
            }
            "page_content_contains" => {
                // Simulate content check (80% success rate for demo)
                if rand_success(0.8) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Page content does not contain '{}'",
                        assertion_value
                    ))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unknown assertion type: {}",
                assertion_type
            )),
        }
    }

    async fn simulate_wait(&self, action_name: &str, duration_ms: u64) -> Result<()> {
        if self.verbose && duration_ms > 1000 {
            println!("    ⏳ Waiting for {} ({}ms)", action_name, duration_ms);
        }
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
}

// Simple random success simulation for demo purposes
fn rand_success(probability: f64) -> bool {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    let hash = hasher.finish();

    (hash as f64 / u64::MAX as f64) < probability
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionReport {
    pub workflow_id: String,
    pub workflow_name: String,
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub total_steps: usize,
    pub successful_steps: usize,
    pub failed_steps: usize,
    pub step_details: Vec<StepResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StepResult {
    pub step_description: String,
    pub step_type: String,
    pub success: bool,
    pub actions_executed: Vec<String>,
    pub error_message: Option<String>,
}

impl ExecutionReport {
    pub fn new(workflow_id: &str, workflow_name: &str) -> Self {
        Self {
            workflow_id: workflow_id.to_string(),
            workflow_name: workflow_name.to_string(),
            start_time: None,
            end_time: None,
            total_steps: 0,
            successful_steps: 0,
            failed_steps: 0,
            step_details: Vec::new(),
        }
    }

    pub fn start_execution(&mut self) {
        self.start_time = Some(SystemTime::now());
    }

    pub fn end_execution(&mut self) {
        self.end_time = Some(SystemTime::now());
    }

    pub fn add_step_success(&mut self, step: &TestStep, actions_executed: Vec<String>) {
        self.total_steps += 1;
        self.successful_steps += 1;
        self.step_details.push(StepResult {
            step_description: step.description.clone(),
            step_type: step.step_type.clone(),
            success: true,
            actions_executed,
            error_message: None,
        });
    }

    pub fn add_step_failure(&mut self, step: &TestStep, error: String) {
        self.total_steps += 1;
        self.failed_steps += 1;
        self.step_details.push(StepResult {
            step_description: step.description.clone(),
            step_type: step.step_type.clone(),
            success: false,
            actions_executed: Vec::new(),
            error_message: Some(error),
        });
    }

    pub fn get_duration(&self) -> Option<Duration> {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            end.duration_since(start).ok()
        } else {
            None
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_steps == 0 {
            0.0
        } else {
            self.successful_steps as f64 / self.total_steps as f64
        }
    }

    pub fn print_summary(&self) {
        println!("\n📊 === Execution Report ===");
        println!("🎯 Workflow: {}", self.workflow_name);
        println!("📈 Success Rate: {:.1}%", self.success_rate() * 100.0);
        println!("✅ Successful Steps: {}", self.successful_steps);
        println!("❌ Failed Steps: {}", self.failed_steps);

        if let Some(duration) = self.get_duration() {
            println!("⏱️  Duration: {:.2}s", duration.as_secs_f64());
        }

        println!("\n🔍 Step Details:");
        for (i, step_result) in self.step_details.iter().enumerate() {
            let status = if step_result.success { "✅" } else { "❌" };
            println!(
                "  {}. {} {} ({})",
                i + 1,
                status,
                step_result.step_description,
                step_result.step_type.to_uppercase()
            );

            if !step_result.actions_executed.is_empty() && step_result.success {
                println!("     Actions: {}", step_result.actions_executed.join(", "));
            }

            if let Some(error) = &step_result.error_message {
                println!("     Error: {}", error);
            }
        }
        println!("================================\n");
    }
}
