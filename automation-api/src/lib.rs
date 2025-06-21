use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    pub id: String,
    pub title: String,
    pub feature: String,
    pub scenario: String,
    pub given_steps: Vec<String>,
    pub when_steps: Vec<String>,
    pub then_steps: Vec<String>,
    pub tags: Vec<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    pub action_type: String,
    pub selector: Option<String>,
    pub element_description: Option<String>,
    pub url: Option<String>,
    pub text: Option<String>,
    pub wait_condition: Option<String>,
    pub screenshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub step_type: String, // given, when, then
    pub description: String,
    pub browser_actions: Vec<BrowserAction>,
    pub assertions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub test_steps: Vec<TestStep>,
    pub source_criteria: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ACAutomationProcessor {
    workflows: HashMap<String, AutomationWorkflow>,
    acceptance_criteria: HashMap<String, AcceptanceCriteria>,
}

impl ACAutomationProcessor {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            acceptance_criteria: HashMap::new(),
        }
    }

    pub fn parse_acceptance_criteria(
        &mut self,
        criteria_id: String,
        title: String,
        feature: String,
        criteria_text: String,
        tags: Vec<String>,
    ) -> Result<AcceptanceCriteria> {
        let parsed_criteria =
            self.parse_gherkin_or_text(&criteria_id, &criteria_text, &title, &feature, &tags);

        self.acceptance_criteria
            .insert(criteria_id.clone(), parsed_criteria.clone());
        Ok(parsed_criteria)
    }

    pub fn convert_to_automation(&mut self, criteria_id: &str) -> Result<AutomationWorkflow> {
        let criteria = self
            .acceptance_criteria
            .get(criteria_id)
            .ok_or_else(|| anyhow::anyhow!("Acceptance criteria not found: {}", criteria_id))?;

        let workflow = self.generate_automation_workflow(criteria);
        self.workflows.insert(workflow.id.clone(), workflow.clone());
        Ok(workflow)
    }

    pub fn get_workflows(&self) -> Vec<&AutomationWorkflow> {
        self.workflows.values().collect()
    }

    pub fn get_workflow(&self, workflow_id: &str) -> Option<&AutomationWorkflow> {
        self.workflows.get(workflow_id)
    }

    pub fn get_acceptance_criteria(&self) -> Vec<&AcceptanceCriteria> {
        self.acceptance_criteria.values().collect()
    }

    pub fn generate_browser_script(
        &self,
        workflow: &AutomationWorkflow,
        script_format: &str,
    ) -> String {
        match script_format {
            "mcp_browser" => self.generate_mcp_browser_script(workflow),
            "selenium" => self.generate_selenium_script(workflow),
            "playwright" => self.generate_playwright_script(workflow),
            _ => format!("// Unsupported script format: {}", script_format),
        }
    }

    fn parse_gherkin_or_text(
        &self,
        criteria_id: &str,
        criteria_text: &str,
        title: &str,
        feature: &str,
        tags: &Vec<String>,
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

        // If no Gherkin format detected, treat as plain text
        if given_steps.is_empty() && when_steps.is_empty() && then_steps.is_empty() {
            given_steps.push("Initial state defined by criteria".to_string());
            when_steps.push(criteria_text.to_string());
            then_steps.push("Expected outcome should be verified".to_string());
            scenario = title.to_string();
        }

        AcceptanceCriteria {
            id: criteria_id.to_string(),
            title: title.to_string(),
            feature: feature.to_string(),
            scenario,
            given_steps,
            when_steps,
            then_steps,
            tags: tags.clone(),
            priority: "normal".to_string(),
        }
    }

    fn generate_automation_workflow(&self, ac: &AcceptanceCriteria) -> AutomationWorkflow {
        let mut test_steps = Vec::new();

        // Convert Given steps
        for given in &ac.given_steps {
            let actions = self.step_to_browser_actions(given, "setup");
            test_steps.push(TestStep {
                step_type: "given".to_string(),
                description: given.clone(),
                browser_actions: actions,
                assertions: vec![],
            });
        }

        // Convert When steps
        for when in &ac.when_steps {
            let actions = self.step_to_browser_actions(when, "action");
            test_steps.push(TestStep {
                step_type: "when".to_string(),
                description: when.clone(),
                browser_actions: actions,
                assertions: vec![],
            });
        }

        // Convert Then steps
        for then in &ac.then_steps {
            let actions = self.step_to_browser_actions(then, "verification");
            let assertions = self.step_to_assertions(then);
            test_steps.push(TestStep {
                step_type: "then".to_string(),
                description: then.clone(),
                browser_actions: actions,
                assertions,
            });
        }

        AutomationWorkflow {
            id: format!("workflow_{}", ac.id),
            name: format!("Automation for: {}", ac.title),
            description: format!(
                "Automated test workflow for {} - {}",
                ac.feature, ac.scenario
            ),
            test_steps,
            source_criteria: ac.id.clone(),
            tags: ac.tags.clone(),
        }
    }

    fn step_to_browser_actions(&self, step: &str, step_category: &str) -> Vec<BrowserAction> {
        let mut actions = Vec::new();
        let step_lower = step.to_lowercase();

        // Navigation actions
        if step_lower.contains("visit")
            || step_lower.contains("navigate")
            || step_lower.contains("go to")
        {
            let url = self
                .extract_url_from_step(step)
                .unwrap_or("https://example.com".to_string());
            actions.push(BrowserAction {
                action_type: "navigate".to_string(),
                selector: None,
                element_description: None,
                url: Some(url),
                text: None,
                wait_condition: Some("page_load".to_string()),
                screenshot: false,
            });
        }

        // Click actions
        if step_lower.contains("click") || step_lower.contains("press") {
            let selector = self.extract_selector_from_step(step, "button");
            actions.push(BrowserAction {
                action_type: "click".to_string(),
                selector: Some(selector),
                element_description: None,
                url: None,
                text: None,
                wait_condition: Some("element_clickable".to_string()),
                screenshot: false,
            });
        }

        // Input actions
        if step_lower.contains("enter")
            || step_lower.contains("type")
            || step_lower.contains("fill")
        {
            let selector = self.extract_selector_from_step(step, "input");
            let text = self.extract_text_from_step(step);
            actions.push(BrowserAction {
                action_type: "type".to_string(),
                selector: Some(selector),
                element_description: None,
                url: None,
                text: Some(text),
                wait_condition: Some("element_visible".to_string()),
                screenshot: false,
            });
        }

        // Select actions
        if step_lower.contains("select") || step_lower.contains("choose") {
            let selector = self.extract_selector_from_step(step, "select");
            let text = self.extract_text_from_step(step);
            actions.push(BrowserAction {
                action_type: "select".to_string(),
                selector: Some(selector),
                element_description: None,
                url: None,
                text: Some(text),
                wait_condition: Some("element_visible".to_string()),
                screenshot: false,
            });
        }

        // Wait/verification actions for "then" steps
        if step_category == "verification" {
            actions.push(BrowserAction {
                action_type: "wait".to_string(),
                selector: None,
                element_description: None,
                url: None,
                text: None,
                wait_condition: Some("page_stable".to_string()),
                screenshot: true,
            });
        }

        actions
    }

    fn step_to_assertions(&self, step: &str) -> Vec<String> {
        let mut assertions = Vec::new();
        let step_lower = step.to_lowercase();

        if step_lower.contains("see")
            || step_lower.contains("display")
            || step_lower.contains("show")
        {
            assertions.push(format!(
                "element_visible: {}",
                self.extract_text_from_step(step)
            ));
        }

        if step_lower.contains("redirect") || step_lower.contains("url") {
            assertions.push(format!(
                "url_contains: {}",
                self.extract_text_from_step(step)
            ));
        }

        if step_lower.contains("not") {
            assertions.push(format!(
                "element_not_visible: {}",
                self.extract_text_from_step(step)
            ));
        }

        if assertions.is_empty() {
            assertions.push(format!(
                "page_content_contains: {}",
                self.extract_text_from_step(step)
            ));
        }

        assertions
    }

    fn extract_selector_from_step(&self, step: &str, default_element: &str) -> String {
        // Simple heuristic to extract selectors from natural language
        if step.contains("button") {
            return format!(
                "button[contains(text(), '{}')]",
                self.extract_text_from_step(step)
            );
        }
        if step.contains("link") {
            return format!(
                "a[contains(text(), '{}')]",
                self.extract_text_from_step(step)
            );
        }
        if step.contains("input") || step.contains("field") {
            return format!(
                "input[name*='{}']",
                self.extract_text_from_step(step)
                    .to_lowercase()
                    .replace(" ", "_")
            );
        }

        format!(
            "{}[contains(text(), '{}')]",
            default_element,
            self.extract_text_from_step(step)
        )
    }

    fn extract_text_from_step(&self, step: &str) -> String {
        // Extract text between quotes or after keywords
        if let Some(start) = step.find('"') {
            if let Some(end) = step[start + 1..].find('"') {
                return step[start + 1..start + 1 + end].to_string();
            }
        }

        if let Some(start) = step.find('\'') {
            if let Some(end) = step[start + 1..].find('\'') {
                return step[start + 1..start + 1 + end].to_string();
            }
        }

        // Extract words after common keywords
        for keyword in &["enter", "type", "click", "select", "see", "display"] {
            if let Some(pos) = step.to_lowercase().find(keyword) {
                let after_keyword = &step[pos + keyword.len()..].trim();
                if !after_keyword.is_empty() {
                    return after_keyword
                        .split_whitespace()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                }
            }
        }

        "test_value".to_string()
    }

    fn extract_url_from_step(&self, step: &str) -> Option<String> {
        // Look for URLs in the step
        let words: Vec<&str> = step.split_whitespace().collect();
        for word in words {
            if word.starts_with("http") || word.contains(".com") || word.contains(".org") {
                return Some(word.to_string());
            }
        }

        // Look for page references
        if step.to_lowercase().contains("home") {
            return Some("/".to_string());
        }
        if step.to_lowercase().contains("login") {
            return Some("/login".to_string());
        }
        if step.to_lowercase().contains("dashboard") {
            return Some("/dashboard".to_string());
        }

        None
    }

    fn generate_mcp_browser_script(&self, workflow: &AutomationWorkflow) -> String {
        let mut script = format!(
            "// MCP Browser Automation Script for: {}\n// Generated from: {}\n\n",
            workflow.name, workflow.description
        );

        for step in &workflow.test_steps {
            script.push_str(&format!(
                "// {}: {}\n",
                step.step_type.to_uppercase(),
                step.description
            ));

            for action in &step.browser_actions {
                match action.action_type.as_str() {
                    "navigate" => {
                        if let Some(url) = &action.url {
                            script.push_str(&format!("await mcp.browser.navigate('{}');\n", url));
                        }
                    }
                    "click" => {
                        if let Some(selector) = &action.selector {
                            script.push_str(&format!("await mcp.browser.click('{}');\n", selector));
                        }
                    }
                    "type" => {
                        if let Some(selector) = &action.selector {
                            if let Some(text) = &action.text {
                                script.push_str(&format!(
                                    "await mcp.browser.type('{}', '{}');\n",
                                    selector, text
                                ));
                            }
                        }
                    }
                    "select" => {
                        if let Some(selector) = &action.selector {
                            if let Some(text) = &action.text {
                                script.push_str(&format!(
                                    "await mcp.browser.select('{}', '{}');\n",
                                    selector, text
                                ));
                            }
                        }
                    }
                    "wait" => {
                        if let Some(condition) = &action.wait_condition {
                            script.push_str(&format!("await mcp.browser.wait('{}');\n", condition));
                        }
                    }
                    _ => {}
                }

                if action.screenshot {
                    script.push_str("await mcp.browser.screenshot();\n");
                }
            }

            for assertion in &step.assertions {
                script.push_str(&format!("await mcp.browser.assert('{}');\n", assertion));
            }

            script.push_str("\n");
        }

        script
    }

    fn generate_selenium_script(&self, workflow: &AutomationWorkflow) -> String {
        let mut script = format!(
            "# Selenium WebDriver Python script for: {}\n# Generated from: {}\n\n",
            workflow.name, workflow.description
        );

        script.push_str("from selenium import webdriver\n");
        script.push_str("from selenium.webdriver.common.by import By\n");
        script.push_str("from selenium.webdriver.support.ui import WebDriverWait\n");
        script.push_str("from selenium.webdriver.support import expected_conditions as EC\n\n");
        script.push_str("driver = webdriver.Chrome()\n");
        script.push_str("wait = WebDriverWait(driver, 10)\n\n");

        for step in &workflow.test_steps {
            script.push_str(&format!(
                "# {}: {}\n",
                step.step_type.to_uppercase(),
                step.description
            ));

            for action in &step.browser_actions {
                match action.action_type.as_str() {
                    "navigate" => {
                        if let Some(url) = &action.url {
                            script.push_str(&format!("driver.get('{}')\n", url));
                        }
                    }
                    "click" => {
                        if let Some(selector) = &action.selector {
                            script.push_str(&format!("wait.until(EC.element_to_be_clickable((By.XPATH, \"{}\"))).click()\n", selector));
                        }
                    }
                    "type" => {
                        if let Some(selector) = &action.selector {
                            if let Some(text) = &action.text {
                                script.push_str(&format!("wait.until(EC.visibility_of_element_located((By.XPATH, \"{}\"))).send_keys('{}')\n", selector, text));
                            }
                        }
                    }
                    _ => {}
                }
            }
            script.push_str("\n");
        }

        script.push_str("driver.quit()\n");
        script
    }

    fn generate_playwright_script(&self, workflow: &AutomationWorkflow) -> String {
        let mut script = format!(
            "// Playwright JavaScript script for: {}\n// Generated from: {}\n\n",
            workflow.name, workflow.description
        );

        script.push_str("const { chromium } = require('playwright');\n\n");
        script.push_str("(async () => {\n");
        script.push_str("  const browser = await chromium.launch();\n");
        script.push_str("  const page = await browser.newPage();\n\n");

        for step in &workflow.test_steps {
            script.push_str(&format!(
                "  // {}: {}\n",
                step.step_type.to_uppercase(),
                step.description
            ));

            for action in &step.browser_actions {
                match action.action_type.as_str() {
                    "navigate" => {
                        if let Some(url) = &action.url {
                            script.push_str(&format!("  await page.goto('{}');\n", url));
                        }
                    }
                    "click" => {
                        if let Some(selector) = &action.selector {
                            script.push_str(&format!("  await page.click('{}');\n", selector));
                        }
                    }
                    "type" => {
                        if let Some(selector) = &action.selector {
                            if let Some(text) = &action.text {
                                script.push_str(&format!(
                                    "  await page.fill('{}', '{}');\n",
                                    selector, text
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
            script.push_str("\n");
        }

        script.push_str("  await browser.close();\n");
        script.push_str("})();\n");
        script
    }
}
