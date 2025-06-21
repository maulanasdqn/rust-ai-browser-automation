use anyhow::Result;
use automation_api::AcceptanceCriteria;
use automation_api::{AutomationWorkflow, BrowserAction, TestStep};
use automation_browser::{AutomationExecutor, ExecutionReport};
use openrouter_rs::{
    api::chat::{ChatCompletionRequest, Message},
    types::{Choice, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};

pub struct AIAutomationEngine {
    openrouter_client: OpenRouterClient,
    model: String,
    conversation_history: Vec<Message>,
    headless: bool,
    vision_mode: bool,
    vision_api_key: Option<String>,
    vision_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIBrowserAction {
    pub action: String,
    pub element_description: Option<String>,
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
    pub steps: Vec<AIBrowserAction>,
    pub assertions: Vec<String>,
    pub estimated_duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExecutionResult {
    pub plan: AIExecutionPlan,
    pub execution_report: Option<ExecutionReport>,
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
            headless: true,
            vision_mode: false,
            vision_api_key: None,
            vision_model: None,
        })
    }

    pub fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn set_headless(&mut self, headless: bool) {
        self.headless = headless;
    }

    pub fn set_vision_mode(
        &mut self,
        enabled: bool,
        api_key: Option<String>,
        model: Option<String>,
    ) {
        self.vision_mode = enabled;
        self.vision_api_key = api_key;
        self.vision_model = model;
    }

    pub async fn analyze_and_execute_ac(
        &mut self,
        criteria: &AcceptanceCriteria,
        execute_immediately: bool,
    ) -> Result<AIExecutionResult> {
        println!("🤖 AI analyzing acceptance criteria: {}", criteria.title);

        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_user_prompt(criteria);

        self.conversation_history.clear();
        self.conversation_history
            .push(Message::new(Role::System, &system_prompt));
        self.conversation_history
            .push(Message::new(Role::User, &user_prompt));

        let plan = self.get_ai_execution_plan().await?;

        let execution_report = if execute_immediately {
            Some(self.execute_real_browser_automation(&plan).await?)
        } else {
            None
        };

        let reasoning = self.extract_llm_reasoning();

        Ok(AIExecutionResult {
            plan,
            execution_report,
            llm_reasoning: reasoning,
        })
    }

    async fn execute_real_browser_automation(
        &self,
        plan: &AIExecutionPlan,
    ) -> Result<ExecutionReport> {
        println!("🚀 Executing AI plan with real browser automation...");

        let workflow = self.convert_ai_plan_to_workflow(plan);

        // Check if plan contains element descriptions that will need AI DOM inspection
        let needs_ai_dom_inspection = workflow.test_steps.iter().any(|step| {
            step.browser_actions
                .iter()
                .any(|action| action.element_description.is_some() && action.selector.is_none())
        });

        // Get API key for AI DOM inspection (try from environment if not provided)
        let api_key_for_ai_dom = if let Some(key) = &self.vision_api_key {
            Some(key.clone())
        } else if needs_ai_dom_inspection {
            // Try to get from environment variables
            std::env::var("OPENROUTER_API_KEY").ok()
        } else {
            None
        };

        let mut executor = if needs_ai_dom_inspection {
            if let Some(api_key) = &api_key_for_ai_dom {
                println!("🔍 Configuring automation executor with AI HTML analysis capability");
                AutomationExecutor::new()?
                    .with_headless(self.headless)
                    .with_hybrid_mode(
                        api_key.clone(),
                        Some("anthropic/claude-3.5-sonnet".to_string()),
                    )
            } else {
                println!(
                    "⚠️ AI HTML analysis needed but no API key available, using standard DOM mode"
                );
                AutomationExecutor::new()?.with_headless(self.headless)
            }
        } else {
            println!("🤖 Configuring automation executor with standard DOM mode");
            AutomationExecutor::new()?.with_headless(self.headless)
        };

        executor.set_verbose(true);

        let (report, _logs) = executor.execute_workflow(&workflow).await?;

        println!(
            "📊 AI Browser Automation completed: {:.1}% success rate",
            report.success_rate() * 100.0
        );

        Ok(report)
    }

    fn convert_ai_plan_to_workflow(&self, plan: &AIExecutionPlan) -> AutomationWorkflow {
        let mut test_steps = Vec::new();
        let mut current_actions = Vec::new();
        let mut step_counter = 1;

        // Detect if this is a login scenario
        let is_login_scenario = plan.title.to_lowercase().contains("login")
            || plan.description.to_lowercase().contains("login")
            || plan.steps.iter().any(|step| {
                step.text.as_ref().map_or(false, |t| t.contains("@"))
                    || step
                        .element_description
                        .as_ref()
                        .map_or(false, |s| s.contains("password") || s.contains("email"))
            });

        for ai_action in &plan.steps {
            // Skip vague wait actions for dashboard elements if login scenario
            if is_login_scenario
                && ai_action.action == "wait"
                && ai_action
                    .element_description
                    .as_ref()
                    .map_or(false, |desc| {
                        desc.contains("dashboard element")
                            || desc.contains("element containing text")
                    })
            {
                println!(
                    "🧹 Skipping vague wait action: {:?}",
                    ai_action.element_description
                );
                continue;
            }

            let browser_action = BrowserAction {
                action_type: ai_action.action.clone(),
                selector: None,
                element_description: ai_action.element_description.clone(),
                url: ai_action.url.clone(),
                text: ai_action.text.clone(),
                wait_condition: ai_action.wait_for.clone(),
                screenshot: ai_action.screenshot.unwrap_or(false),
            };

            current_actions.push(browser_action);

            if ai_action.action == "wait"
                || ai_action.action == "screenshot"
                || step_counter % 3 == 0
            {
                test_steps.push(TestStep {
                    step_type: if step_counter <= 3 {
                        "given"
                    } else if step_counter <= 6 {
                        "when"
                    } else {
                        "then"
                    }
                    .to_string(),
                    description: format!("AI Step {}: Execute browser actions", step_counter),
                    browser_actions: current_actions.clone(),
                    assertions: if step_counter > 6 {
                        plan.assertions.clone()
                    } else {
                        vec![]
                    },
                });
                current_actions.clear();
            }

            step_counter += 1;
        }

        // Ensure we have assertions for login scenarios
        let mut final_assertions = plan.assertions.clone();
        if is_login_scenario && !final_assertions.iter().any(|a| a.contains("login_success")) {
            println!("🔍 Login scenario detected - adding automatic login success assertion");
            final_assertions.insert(0, "login_success: true".to_string());
        }

        // Filter out vague assertions that are likely to fail
        final_assertions.retain(|assertion| {
            // Keep concrete assertions
            if assertion.contains("login_success")
                || assertion.contains("url_contains")
                || assertion.contains("page_content_contains")
            {
                return true;
            }

            // Remove vague element_visible assertions for generic elements
            if assertion.contains("element_visible:")
                && (assertion.contains("dashboard element")
                    || assertion.contains("element containing text")
                    || assertion.contains("generic"))
            {
                println!("🧹 Filtering out vague assertion: {}", assertion);
                return false;
            }

            true
        });

        if !current_actions.is_empty() {
            test_steps.push(TestStep {
                step_type: "then".to_string(),
                description: "Final AI actions and verification".to_string(),
                browser_actions: current_actions,
                assertions: final_assertions,
            });
        } else if !final_assertions.is_empty() {
            // Add a verification-only step if we have assertions but no remaining actions
            test_steps.push(TestStep {
                step_type: "then".to_string(),
                description: "Login verification and assertion checks".to_string(),
                browser_actions: vec![],
                assertions: final_assertions,
            });
        }

        AutomationWorkflow {
            id: format!(
                "ai_workflow_{}",
                uuid::Uuid::new_v4().to_string()[..8].to_string()
            ),
            name: plan.title.clone(),
            description: plan.description.clone(),
            test_steps,
            source_criteria: "ai_generated".to_string(),
            tags: vec![
                "ai".to_string(),
                "browser".to_string(),
                "automation".to_string(),
            ],
        }
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
3. DESCRIBE UI elements instead of guessing selectors - the system will inspect the page to find them
4. Add appropriate waits and assertions
5. Consider edge cases and error handling

CRITICAL FOR ELEMENT IDENTIFICATION:
- Instead of guessing CSS selectors, provide clear DESCRIPTIONS of elements
- For login forms, describe as: "email input field", "password input field", "login submit button"
- For navigation: "home link", "dashboard menu item", "logout button"
- For content: "error message container", "success notification", "user profile section"
- The system will take screenshots and use AI to find the actual selectors

CRITICAL FOR CREDENTIALS AND TEXT INPUT:
- Use text/credentials EXACTLY as provided in the scenario
- DO NOT modify, correct, or change any text values
- If scenario says "passwordss", use "passwordss" not "password"
- If scenario says "admin@example.com", use exactly "admin@example.com"
- Copy text values character-for-character from the provided scenario

CRITICAL FOR LOGIN SCENARIOS:
- ALWAYS add assertions to verify login success/failure
- Check for dashboard elements, error messages, or URL changes
- Include multiple verification methods (URL, page content, specific elements)
- Add screenshots after login attempts for debugging
- Use provided credentials exactly as written (don't fix "typos")

Respond with ONLY a valid JSON object, no additional text or explanations:
{
  "title": "Test scenario title",
  "description": "What this automation does", 
  "steps": [
    {
      "action": "navigate|click|type|select|wait|screenshot",
      "element_description": "clear description of UI element to find (e.g., 'email input field', 'submit button')",
      "url": "URL to navigate to (for navigate action)",
      "text": "Text to type or select (if applicable)",
      "wait_for": "element_visible|page_load|network_idle|custom_condition",
      "screenshot": true (optional, only include if taking screenshot)
    }
  ],
  "assertions": [
    "login_success: true",
    "url_contains: dashboard", 
    "page_content_contains: Welcome"
  ],
  "estimated_duration": 30.5
}

IMPORTANT: 
- Return ONLY the JSON object above
- Use element_description instead of selector - describe what element you want to find
- Use credentials and text EXACTLY as provided - no corrections or modifications
- For LOGIN tests, use SPECIFIC assertions that can be verified:
  * "login_success: true" (comprehensive URL + content check)
  * "url_contains: dashboard" (URL verification)
  * "page_content_contains: Welcome" (success text check)
  * "element_not_visible: .error" (error message check)
- AVOID vague assertions like "element_visible: dashboard element"
- NEVER use "element_visible" for generic elements like "dashboard element"
- ONLY use concrete assertions: login_success, url_contains, page_content_contains
- Examples of good element descriptions:
  * "email input field" or "username input field"
  * "password input field" 
  * "login button" or "submit button"
  * "error message" or "error notification"
- Add wait conditions after login submit: wait_for: "page_load"
- Include screenshot: true after critical actions
- Focus on verifiable assertions (URL changes, specific text, absence of errors)

The system will automatically take screenshots and use AI to find the actual selectors on the page, so focus on clear element descriptions."#.to_string()
    }

    fn create_user_prompt(&self, criteria: &AcceptanceCriteria) -> String {
        // Detect if this is a login scenario and extract credentials more explicitly
        let full_scenario_text = format!(
            "{}\n{}\n{}",
            criteria.given_steps.join("\n"),
            criteria.when_steps.join("\n"),
            criteria.then_steps.join("\n")
        );

        format!(
            r#"Convert this acceptance criteria to a browser automation plan:

Title: {}
Feature: {}
Scenario: {}

Full scenario text (use ALL text values EXACTLY as written):
{}

IMPORTANT: Pay special attention to any credentials, email addresses, passwords, or text values in the scenario above. Use them EXACTLY as provided - do not modify, correct, or change any characters.

Given steps:
{}

When steps:
{}

Then steps:
{}

Tags: {:?}

Create a detailed browser automation plan that uses ALL text values exactly as provided in the scenario."#,
            criteria.title,
            criteria.feature,
            criteria.scenario,
            full_scenario_text,
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
