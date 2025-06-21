pub mod chrome;

use anyhow::Result;
use automation_api::{AutomationWorkflow, TestStep};
use std::time::{Duration, SystemTime};

pub use chrome::{AutomationLog, AutomationMode, ChromeAutomationEngine, VisionAction};

#[derive(Debug, Clone)]
pub struct AutomationExecutor {
    verbose: bool,
    headless: bool,
    mode: AutomationMode,
    vision_api_key: Option<String>,
    vision_model: Option<String>,
}

impl AutomationExecutor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            verbose: true,
            headless: false,
            mode: AutomationMode::Dom,
            vision_api_key: None,
            vision_model: None,
        })
    }

    pub fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn with_vision_mode(mut self, api_key: String, model: Option<String>) -> Self {
        self.mode = AutomationMode::Vision;
        self.vision_api_key = Some(api_key);
        self.vision_model = model;
        self
    }

    pub fn with_hybrid_mode(mut self, api_key: String, model: Option<String>) -> Self {
        self.mode = AutomationMode::Hybrid;
        self.vision_api_key = Some(api_key);
        self.vision_model = model;
        self
    }

    pub async fn execute_workflow(
        &mut self,
        workflow: &AutomationWorkflow,
    ) -> Result<(ExecutionReport, Vec<AutomationLog>)> {
        let mut report = ExecutionReport::new(&workflow.id, &workflow.name);

        if self.verbose {
            println!("🚀 Starting execution of workflow: {}", workflow.name);
        }
        report.start_execution();

        let mut chrome_engine = match self.mode {
            AutomationMode::Dom => ChromeAutomationEngine::new(self.headless),
            AutomationMode::Vision => {
                if let Some(api_key) = &self.vision_api_key {
                    ChromeAutomationEngine::new(self.headless)
                        .with_vision_mode(api_key.clone(), self.vision_model.clone())
                } else {
                    return Err(anyhow::anyhow!("Vision mode requires API key"));
                }
            }
            AutomationMode::Hybrid => {
                if let Some(api_key) = &self.vision_api_key {
                    ChromeAutomationEngine::new(self.headless)
                        .with_hybrid_mode(api_key.clone(), self.vision_model.clone())
                } else {
                    return Err(anyhow::anyhow!("Hybrid mode requires API key"));
                }
            }
        };

        chrome_engine.set_verbose(self.verbose);

        match chrome_engine.initialize().await {
            Ok(_) => {
                if self.verbose {
                    println!("✅ Browser automation engine initialized");
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to initialize browser: {}", e);
                report.add_step_failure(
                    &TestStep {
                        step_type: "initialization".to_string(),
                        description: "Initialize browser".to_string(),
                        browser_actions: vec![],
                        assertions: vec![],
                    },
                    error_msg.clone(),
                );
                report.end_execution();

                let logs = chrome_engine.get_logs().await;
                return Ok((report, logs));
            }
        }

        for (step_index, step) in workflow.test_steps.iter().enumerate() {
            if self.verbose {
                println!(
                    "📋 Step {}: {} - {}",
                    step_index + 1,
                    step.step_type.to_uppercase(),
                    step.description
                );
            }

            let step_result = self.execute_test_step(&mut chrome_engine, step).await;
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

                    if step.step_type == "given" {
                        break;
                    }
                }
            }
        }

        if let Err(e) = chrome_engine.close().await {
            if self.verbose {
                println!("⚠️ Failed to close browser cleanly: {}", e);
            }
        }

        report.end_execution();
        if self.verbose {
            println!("🏁 Workflow execution completed");
            report.print_summary();
        }

        let logs = chrome_engine.get_logs().await;

        Ok((report, logs))
    }

    async fn execute_test_step(
        &mut self,
        chrome_engine: &mut ChromeAutomationEngine,
        step: &TestStep,
    ) -> Result<Vec<String>> {
        let mut executed_actions = Vec::new();

        for action in &step.browser_actions {
            match chrome_engine.execute_browser_action(action).await {
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

        for assertion in &step.assertions {
            match chrome_engine.execute_assertion(assertion).await {
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

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }
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
