use anyhow::Result;
use automation_api::{ACAutomationProcessor, AcceptanceCriteria, AutomationWorkflow};
use automation_browser::{AutomationExecutor, AutomationLog, AutomationMode, ExecutionReport};
use uuid::Uuid;

#[derive(Debug)]
pub struct ACAutomationIntegration {
    processor: ACAutomationProcessor,
    executor: AutomationExecutor,
}

impl ACAutomationIntegration {
    pub fn new() -> Result<Self> {
        Ok(Self {
            processor: ACAutomationProcessor::new(),
            executor: AutomationExecutor::new()?,
        })
    }

    pub fn process_acceptance_criteria_text(
        &mut self,
        criteria_id: String,
        title: String,
        feature: String,
        criteria_text: String,
        tags: Vec<String>,
    ) -> Result<AcceptanceCriteria> {
        println!("📝 Processing acceptance criteria: {}", title);

        let criteria = self.processor.parse_acceptance_criteria(
            criteria_id,
            title,
            feature,
            criteria_text,
            tags,
        )?;

        println!("✅ Acceptance criteria parsed successfully");
        Ok(criteria)
    }

    pub fn convert_to_automation_workflow(
        &mut self,
        criteria_id: &str,
    ) -> Result<AutomationWorkflow> {
        println!("🔄 Converting acceptance criteria to automation workflow...");

        let workflow = self.processor.convert_to_automation(criteria_id)?;
        println!("✅ Automation workflow generated successfully");
        Ok(workflow)
    }

    pub async fn execute_automation_workflow(
        &mut self,
        workflow: &AutomationWorkflow,
    ) -> Result<(ExecutionReport, Vec<AutomationLog>)> {
        println!("🚀 Executing automation workflow: {}", workflow.name);

        let (report, logs) = self.executor.execute_workflow(workflow).await?;
        println!(
            "📊 Workflow execution completed with {}% success rate",
            (report.success_rate() * 100.0).round()
        );
        Ok((report, logs))
    }

    pub async fn execute_automation_workflow_with_vision_config(
        &mut self,
        workflow: &AutomationWorkflow,
        use_vision: bool,
        automation_mode: Option<String>,
        vision_api_key: Option<String>,
        vision_model: Option<String>,
    ) -> Result<(ExecutionReport, Vec<AutomationLog>)> {
        println!(
            "🚀 Executing automation workflow with vision config: {}",
            workflow.name
        );

        // Configure executor based on vision settings
        if use_vision && vision_api_key.is_some() {
            let api_key = vision_api_key.unwrap();
            let mode = automation_mode.as_deref().unwrap_or("hybrid");

            self.executor = match mode {
                "vision" => {
                    println!("👁️ Using Vision Mode (AI Visual Understanding)");
                    AutomationExecutor::new()?
                        .with_vision_mode(api_key, vision_model)
                        .with_headless(false) // Vision works better with visible browser
                }
                "hybrid" => {
                    println!("🔀 Using Hybrid Mode (DOM + Vision Fallback)");
                    AutomationExecutor::new()?
                        .with_hybrid_mode(api_key, vision_model)
                        .with_headless(false)
                }
                _ => {
                    println!("🔧 Using DOM Mode (Traditional)");
                    AutomationExecutor::new()?
                }
            };
        }

        let (report, logs) = self.executor.execute_workflow(workflow).await?;
        println!(
            "📊 Workflow execution completed with {}% success rate",
            (report.success_rate() * 100.0).round()
        );
        Ok((report, logs))
    }

    pub fn get_all_workflows(&self) -> Vec<&AutomationWorkflow> {
        self.processor.get_workflows()
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Option<&AutomationWorkflow> {
        self.processor.get_workflow(workflow_id)
    }

    pub fn generate_browser_script(
        &self,
        workflow: &AutomationWorkflow,
        script_format: &str,
    ) -> String {
        self.processor
            .generate_browser_script(workflow, script_format)
    }

    pub async fn full_ac_to_automation_pipeline(
        &mut self,
        title: String,
        feature: String,
        criteria_text: String,
        tags: Vec<String>,
        execute_immediately: bool,
    ) -> Result<PipelineResult> {
        self.full_ac_to_automation_pipeline_with_vision(
            title,
            feature,
            criteria_text,
            tags,
            execute_immediately,
            false,
            None,
            None,
            None,
        )
        .await
    }

    pub async fn full_ac_to_automation_pipeline_with_vision(
        &mut self,
        title: String,
        feature: String,
        criteria_text: String,
        tags: Vec<String>,
        execute_immediately: bool,
        use_vision: bool,
        automation_mode: Option<String>,
        vision_api_key: Option<String>,
        vision_model: Option<String>,
    ) -> Result<PipelineResult> {
        let criteria_id = format!("ac_{}", Uuid::new_v4().to_string()[..8].to_string());

        println!("🔄 Starting full AC to Automation pipeline for: {}", title);

        // Step 1: Parse acceptance criteria
        let criteria = self.process_acceptance_criteria_text(
            criteria_id.clone(),
            title.clone(),
            feature.clone(),
            criteria_text,
            tags,
        )?;

        // Step 2: Convert to automation workflow
        let workflow = self.convert_to_automation_workflow(&criteria.id)?;

        // Step 3: Generate browser script (optional)
        let mcp_script = self.generate_browser_script(&workflow, "mcp_browser");

        // Step 4: Execute if requested (with vision config)
        let (execution_report, execution_logs) = if execute_immediately {
            let (report, logs) = if use_vision {
                self.execute_automation_workflow_with_vision_config(
                    &workflow,
                    use_vision,
                    automation_mode,
                    vision_api_key,
                    vision_model,
                )
                .await?
            } else {
                self.execute_automation_workflow(&workflow).await?
            };
            (Some(report), Some(logs))
        } else {
            (None, None)
        };

        Ok(PipelineResult {
            criteria,
            workflow,
            mcp_script,
            execution_report,
            execution_logs,
        })
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.executor.set_verbose(verbose);
    }

    pub fn set_headless(&mut self, headless: bool) {
        self.executor = self.executor.clone().with_headless(headless);
    }
}

#[derive(Debug)]
pub struct PipelineResult {
    pub criteria: AcceptanceCriteria,
    pub workflow: AutomationWorkflow,
    pub mcp_script: String,
    pub execution_report: Option<ExecutionReport>,
    pub execution_logs: Option<Vec<AutomationLog>>,
}

impl PipelineResult {
    pub fn print_summary(&self) {
        println!("\n📋 === AC to Automation Pipeline Summary ===");
        println!("📝 Acceptance Criteria: {}", self.criteria.title);
        println!("🎯 Feature: {}", self.criteria.feature);
        println!("📊 Scenario: {}", self.criteria.scenario);
        println!("📦 Given Steps: {}", self.criteria.given_steps.len());
        println!("⚡ When Steps: {}", self.criteria.when_steps.len());
        println!("✅ Then Steps: {}", self.criteria.then_steps.len());
        println!("🏷️  Tags: {:?}", self.criteria.tags);

        println!("\n🤖 Automation Workflow: {}", self.workflow.name);
        println!("📝 Description: {}", self.workflow.description);
        println!("🔢 Total Test Steps: {}", self.workflow.test_steps.len());

        if let Some(report) = &self.execution_report {
            println!("\n📊 === Execution Report ===");
            println!("🎯 Workflow: {}", report.workflow_name);
            println!("📈 Success Rate: {:.1}%", report.success_rate() * 100.0);
            println!("✅ Successful Steps: {}", report.successful_steps);
            println!("❌ Failed Steps: {}", report.failed_steps);

            if let Some(duration) = report.get_duration() {
                println!("⏱️  Duration: {:.2}s", duration.as_secs_f64());
            }
        }

        if let Some(logs) = &self.execution_logs {
            println!("\n📝 === Execution Logs ({} entries) ===", logs.len());
            for log in logs.iter().take(10) {
                println!("[{}] {}: {}", log.timestamp, log.level, log.message);
            }
            if logs.len() > 10 {
                println!("... and {} more log entries", logs.len() - 10);
            }
        }

        println!("\n🔧 Generated MCP Browser Script Preview:");
        let script_preview: String = self
            .mcp_script
            .lines()
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        println!("{}", script_preview);
        if self.mcp_script.lines().count() > 10 {
            println!(
                "... (truncated, {} total lines)",
                self.mcp_script.lines().count()
            );
        }

        println!("\n==========================================\n");
    }
}

pub mod examples {
    pub fn sample_login_acceptance_criteria() -> (String, String, String, Vec<String>) {
        let title = "User Login".to_string();
        let feature = "Authentication".to_string();
        let criteria_text = r#"
Scenario: Successful user login
Given I am on the login page
When I enter valid credentials
And I click the login button
Then I should be redirected to the dashboard
And I should see a welcome message
        "#
        .to_string();
        let tags = vec![
            "login".to_string(),
            "authentication".to_string(),
            "smoke".to_string(),
        ];

        (title, feature, criteria_text, tags)
    }

    pub fn sample_form_submission_criteria() -> (String, String, String, Vec<String>) {
        let title = "Contact Form Submission".to_string();
        let feature = "Contact Management".to_string();
        let criteria_text = r#"
Scenario: Submit contact form with valid data
Given I navigate to the contact page
When I fill in the name field with "John Doe"
And I fill in the email field with "john@example.com"
And I fill in the message field with "Hello World"
And I click the submit button
Then I should see a success message
And the form should be cleared
        "#
        .to_string();
        let tags = vec![
            "contact".to_string(),
            "forms".to_string(),
            "regression".to_string(),
        ];

        (title, feature, criteria_text, tags)
    }

    pub fn sample_shopping_cart_criteria() -> (String, String, String, Vec<String>) {
        let title = "Add Product to Cart".to_string();
        let feature = "E-commerce".to_string();
        let criteria_text = r#"
Scenario: Add product to shopping cart
Given I am on the product page for "Laptop Computer"
When I select quantity "2"
And I click "Add to Cart" button
Then the cart icon should show "2" items
And I should see a confirmation message
        "#
        .to_string();
        let tags = vec![
            "ecommerce".to_string(),
            "cart".to_string(),
            "critical".to_string(),
        ];

        (title, feature, criteria_text, tags)
    }
}
