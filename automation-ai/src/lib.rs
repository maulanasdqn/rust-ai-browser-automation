use anyhow::Result;
use automation_api::AcceptanceCriteria;
use automation_browser::ExecutionReport;
use openrouter_rs::{
    OpenRouterClient,
    api::chat::{ChatCompletionRequest, Message},
    types::{Choice, Role},
};
use serde::{Deserialize, Serialize};

pub struct AIAutomationEngine {
    openrouter_client: OpenRouterClient,
    model: String,
    conversation_history: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub action: String,
    pub selector: Option<String>,
    pub url: Option<String>,
    pub text: Option<String>,
    pub wait_for: Option<String>,
    #[serde(default)]
    pub screenshot: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExecutionPlan {
    pub title: String,
    pub description: String,
    pub steps: Vec<BrowserAction>,
    pub assertions: Vec<String>,
    pub estimated_duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExecutionResult {
    pub plan: AIExecutionPlan,
    pub execution_report: Option<ExecutionReport>,
    pub mcp_script: String,
    pub llm_reasoning: String,
}

impl AIAutomationEngine {
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        let client = OpenRouterClient::builder()
            .api_key(&api_key)
            .http_referer("https://ac-automation.local")
            .x_title("AC Automation Tool")
            .build()?;

        let model = model.unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string());

        Ok(Self {
            openrouter_client: client,
            model,
            conversation_history: Vec::new(),
        })
    }

    pub async fn analyze_and_execute_ac(
        &mut self,
        criteria: &AcceptanceCriteria,
        execute_immediately: bool,
    ) -> Result<AIExecutionResult> {
        println!("🤖 AI analyzing acceptance criteria: {}", criteria.title);

        // Create system prompt for browser automation
        let system_prompt = self.create_system_prompt();

        // Create user prompt with acceptance criteria
        let user_prompt = self.create_user_prompt(criteria);

        // Initialize conversation
        self.conversation_history.clear();
        self.conversation_history
            .push(Message::new(Role::System, &system_prompt));
        self.conversation_history
            .push(Message::new(Role::User, &user_prompt));

        // Get AI analysis and plan
        let plan = self.get_ai_execution_plan().await?;

        // Generate MCP Browser script
        let mcp_script = self.generate_mcp_browser_script(&plan);

        // Execute if requested
        let execution_report = if execute_immediately {
            Some(self.execute_via_mcp(&plan).await?)
        } else {
            None
        };

        let reasoning = self.extract_llm_reasoning();

        Ok(AIExecutionResult {
            plan,
            execution_report,
            mcp_script,
            llm_reasoning: reasoning,
        })
    }

    fn create_system_prompt(&self) -> String {
        r#"You are an expert browser automation AI that converts acceptance criteria into precise browser automation plans.

Your capabilities:
- Navigate to URLs
- Click elements (buttons, links, etc.)
- Type text into inputs
- Select from dropdowns
- Wait for elements/conditions
- Take screenshots
- Verify page content

When given acceptance criteria, you must:
1. Analyze the scenario step by step
2. Create a detailed execution plan with specific browser actions
3. Generate CSS selectors for UI elements
4. Add appropriate waits and assertions
5. Consider edge cases and error handling

Respond with ONLY a valid JSON object, no additional text or explanations:
{
  "title": "Test scenario title",
  "description": "What this automation does", 
  "steps": [
    {
      "action": "navigate|click|type|select|wait|screenshot",
      "selector": "CSS selector (if applicable)",
      "url": "URL to navigate to (for navigate action)",
      "text": "Text to type or select (if applicable)",
      "wait_for": "element_visible|page_load|network_idle|custom_condition",
      "screenshot": true (optional, only include if taking screenshot)
    }
  ],
  "assertions": ["List of things to verify"],
  "estimated_duration": 30.5
}

IMPORTANT: Return ONLY the JSON object above. Do not include any notes, explanations, or additional text before or after the JSON.

Be precise with selectors - use realistic CSS selectors that would work on typical web pages.
For forms, use input[name="fieldname"] or input[type="email"] etc.
For buttons, use button[type="submit"] or .btn-primary etc.
For links, use a[href*="keyword"] or .nav-link etc.

Make the automation robust and realistic."#.to_string()
    }

    fn create_user_prompt(&self, criteria: &AcceptanceCriteria) -> String {
        format!(
            r#"Convert this acceptance criteria to a browser automation plan:

Title: {}
Feature: {}
Scenario: {}

Given steps:
{}

When steps:
{}

Then steps:
{}

Tags: {:?}

Create a detailed browser automation plan that can be executed via MCP Browser to test this scenario."#,
            criteria.title,
            criteria.feature,
            criteria.scenario,
            criteria.given_steps.join("\n"),
            criteria.when_steps.join("\n"),
            criteria.then_steps.join("\n"),
            criteria.tags
        )
    }

    async fn get_ai_execution_plan(&mut self) -> Result<AIExecutionPlan> {
        let request = ChatCompletionRequest::builder()
            .model(&self.model)
            .messages(self.conversation_history.clone())
            .temperature(0.3)
            .max_tokens(2000)
            .build()?;

        let response = self
            .openrouter_client
            .send_chat_completion(&request)
            .await?;

        // Add AI response to conversation history
        if let Some(choice) = response.choices.first() {
            if let Choice::NonStreaming(choice_data) = choice {
                if let Some(content) = &choice_data.message.content {
                    self.conversation_history
                        .push(Message::new(Role::Assistant, content));

                    // Extract JSON from response (handle trailing text)
                    let json_content = self.extract_json_from_response(content)?;

                    // Parse JSON response
                    let plan: AIExecutionPlan = serde_json::from_str(&json_content).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to parse AI response as JSON: {}\nExtracted JSON: {}\nFull Response: {}",
                            e,
                            json_content,
                            content
                        )
                    })?;

                    return Ok(plan);
                }
            }
        }
        Err(anyhow::anyhow!("AI response had no content"))
    }

    fn generate_mcp_browser_script(&self, plan: &AIExecutionPlan) -> String {
        let mut script = format!(
            "// AI-Generated MCP Browser Script\n// Title: {}\n// Description: {}\n// Estimated Duration: {:.1}s\n\n",
            plan.title, plan.description, plan.estimated_duration
        );

        script.push_str("const { MCPBrowser } = require('@mcp/browser');\n\n");
        script.push_str("async function executeAutomation() {\n");
        script.push_str("  const browser = new MCPBrowser();\n");
        script.push_str("  \n");
        script.push_str("  try {\n");

        for (i, step) in plan.steps.iter().enumerate() {
            script.push_str(&format!("    // Step {}: {}\n", i + 1, step.action));

            match step.action.as_str() {
                "navigate" => {
                    if let Some(url) = &step.url {
                        script.push_str(&format!("    await browser.navigate('{}');\n", url));
                    }
                }
                "click" => {
                    if let Some(selector) = &step.selector {
                        script.push_str(&format!("    await browser.click('{}');\n", selector));
                    }
                }
                "type" => {
                    if let Some(selector) = &step.selector {
                        if let Some(text) = &step.text {
                            script.push_str(&format!(
                                "    await browser.type('{}', '{}');\n",
                                selector, text
                            ));
                        }
                    }
                }
                "select" => {
                    if let Some(selector) = &step.selector {
                        if let Some(text) = &step.text {
                            script.push_str(&format!(
                                "    await browser.select('{}', '{}');\n",
                                selector, text
                            ));
                        }
                    }
                }
                "wait" => {
                    if let Some(condition) = &step.wait_for {
                        script.push_str(&format!("    await browser.waitFor('{}');\n", condition));
                    }
                }
                "screenshot" => {
                    script.push_str("    await browser.screenshot();\n");
                }
                _ => {
                    script.push_str(&format!("    // Unknown action: {}\n", step.action));
                }
            }

            if step.screenshot.unwrap_or(false) {
                script.push_str("    await browser.screenshot();\n");
            }

            script.push_str("\n");
        }

        // Add assertions
        for assertion in &plan.assertions {
            script.push_str(&format!("    // Verify: {}\n", assertion));
            script.push_str(&format!("    await browser.assert('{}');\n", assertion));
        }

        script.push_str("    \n");
        script.push_str("    console.log('✅ Automation completed successfully!');\n");
        script.push_str("    \n");
        script.push_str("  } catch (error) {\n");
        script.push_str("    console.error('❌ Automation failed:', error);\n");
        script.push_str("    await browser.screenshot('error');\n");
        script.push_str("    throw error;\n");
        script.push_str("  } finally {\n");
        script.push_str("    await browser.close();\n");
        script.push_str("  }\n");
        script.push_str("}\n");
        script.push_str("\n");
        script.push_str("executeAutomation().catch(console.error);\n");

        script
    }

    async fn execute_via_mcp(&self, plan: &AIExecutionPlan) -> Result<ExecutionReport> {
        println!("🚀 Executing AI plan via MCP Browser simulation...");

        // For now, simulate execution since we don't have actual MCP Browser setup
        // In a real implementation, this would use the MCP Browser protocol
        let mut report = ExecutionReport::new("ai_execution", &plan.title);
        report.start_execution();

        let mut successful_steps = 0;
        let mut failed_steps = 0;

        for (i, step) in plan.steps.iter().enumerate() {
            println!(
                "  🔧 Step {}: {} {}",
                i + 1,
                step.action,
                step.selector.as_ref().unwrap_or(&"".to_string())
            );

            // Simulate execution time
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Simulate success/failure (90% success rate for AI-generated plans)
            let success = self.simulate_success(0.9);

            if success {
                successful_steps += 1;
                println!("    ✅ Success");
            } else {
                failed_steps += 1;
                println!("    ❌ Failed");
            }
        }

        // Simulate assertion checking
        for assertion in &plan.assertions {
            println!("  ✓ Checking: {}", assertion);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let success = self.simulate_success(0.85);
            if success {
                successful_steps += 1;
            } else {
                failed_steps += 1;
            }
        }

        report.successful_steps = successful_steps;
        report.failed_steps = failed_steps;
        report.total_steps = successful_steps + failed_steps;
        report.end_execution();

        println!(
            "📊 AI Execution completed: {:.1}% success rate",
            report.success_rate() * 100.0
        );

        Ok(report)
    }

    fn simulate_success(&self, probability: f64) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::SystemTime;

        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        (hash as f64 / u64::MAX as f64) < probability
    }

    fn extract_llm_reasoning(&self) -> String {
        if let Some(last_message) = self.conversation_history.last() {
            if matches!(last_message.role, Role::Assistant) {
                return last_message.content.clone();
            }
        }
        "No reasoning available".to_string()
    }

    fn extract_json_from_response(&self, content: &str) -> Result<String> {
        // Find the start of JSON object
        if let Some(start) = content.find('{') {
            let mut brace_count = 0;
            let mut end_pos = start;

            for (i, ch) in content[start..].char_indices() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            end_pos = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if brace_count == 0 {
                return Ok(content[start..end_pos].to_string());
            }
        }

        // Fallback to original content if JSON extraction fails
        Ok(content.to_string())
    }

    pub async fn get_available_models(&self) -> Result<Vec<String>> {
        // For now, return a list of good models for browser automation
        Ok(vec![
            "anthropic/claude-3.5-sonnet".to_string(),
            "openai/gpt-4o".to_string(),
            "openai/gpt-4o-mini".to_string(),
            "anthropic/claude-3-opus".to_string(),
            "meta-llama/llama-3.1-70b-instruct".to_string(),
            "google/gemini-pro-1.5".to_string(),
        ])
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub fn get_conversation_history(&self) -> &[Message] {
        &self.conversation_history
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    pub async fn chat_with_ai(&mut self, message: String) -> Result<String> {
        self.conversation_history
            .push(Message::new(Role::User, &message));

        let request = ChatCompletionRequest::builder()
            .model(&self.model)
            .messages(self.conversation_history.clone())
            .temperature(0.7)
            .max_tokens(1000)
            .build()?;

        let response = self
            .openrouter_client
            .send_chat_completion(&request)
            .await?;

        if let Some(choice) = response.choices.first() {
            if let Choice::NonStreaming(choice_data) = choice {
                if let Some(content) = &choice_data.message.content {
                    self.conversation_history
                        .push(Message::new(Role::Assistant, content));
                    return Ok(content.clone());
                }
            }
        }
        Err(anyhow::anyhow!("AI response had no content"))
    }
}
