use anyhow::Result;
use automation_api::BrowserAction;
use base64::{engine::general_purpose, Engine as _};
use regex;
use reqwest;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use thirtyfour::prelude::*;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutomationLog {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub step_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisionAction {
    pub action_type: String,
    pub description: String,
    pub coordinates: Option<(i32, i32)>,
    pub text_input: Option<String>,
    pub screenshot_before: Option<String>,
    pub screenshot_after: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AutomationMode {
    Dom,    // Traditional DOM-based (existing)
    Vision, // AI Vision-based (new)
    Hybrid, // Combination of both
}

pub struct ChromeAutomationEngine {
    driver: Option<WebDriver>,
    headless: bool,
    logs: Arc<Mutex<Vec<AutomationLog>>>,
    verbose: bool,
    mode: AutomationMode,
    vision_api_key: Option<String>,
    vision_model: String,
}

impl ChromeAutomationEngine {
    pub fn new(headless: bool) -> Self {
        Self {
            driver: None,
            headless,
            logs: Arc::new(Mutex::new(Vec::new())),
            verbose: true,
            mode: AutomationMode::Dom, // Default to DOM mode
            vision_api_key: None,
            vision_model: "anthropic/claude-3.5-sonnet".to_string(), // Claude has fewer safety restrictions
        }
    }

    pub fn with_vision_mode(mut self, api_key: String, model: Option<String>) -> Self {
        self.mode = AutomationMode::Vision;
        self.vision_api_key = Some(api_key);
        if let Some(m) = model {
            self.vision_model = m;
        }
        self
    }

    pub fn with_hybrid_mode(mut self, api_key: String, model: Option<String>) -> Self {
        self.mode = AutomationMode::Hybrid;
        self.vision_api_key = Some(api_key);
        if let Some(m) = model {
            self.vision_model = m;
        }
        self
    }

    pub async fn get_logs(&self) -> Vec<AutomationLog> {
        self.logs.lock().await.clone()
    }

    pub async fn clear_logs(&self) {
        self.logs.lock().await.clear();
    }

    async fn log(&self, level: &str, message: &str, step_type: Option<&str>) {
        let log_entry = AutomationLog {
            timestamp: chrono::Utc::now().format("%H:%M:%S%.3f").to_string(),
            level: level.to_string(),
            message: message.to_string(),
            step_type: step_type.map(|s| s.to_string()),
        };

        // Add to internal logs
        self.logs.lock().await.push(log_entry.clone());

        // Also print to console if verbose
        if self.verbose {
            println!("[{}] {}: {}", log_entry.timestamp, level, message);
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.log("INFO", "🚀 Initializing Chrome WebDriver...", None)
            .await;

        let mut caps = DesiredCapabilities::chrome();

        if self.headless {
            caps.add_arg("--headless")?;
            self.log("INFO", "Running in headless mode", None).await;
        } else {
            self.log("INFO", "Running with visible browser window", None)
                .await;
        }

        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;
        caps.add_arg("--disable-gpu")?;
        caps.add_arg("--window-size=1920,1080")?;

        let driver = WebDriver::new("http://localhost:9515", caps).await
            .map_err(|e| {
                let error_msg = format!("Failed to initialize Chrome WebDriver: {}. Make sure ChromeDriver is running on port 9515", e);
                anyhow::anyhow!(error_msg)
            })?;

        driver.set_window_rect(0, 0, 1920, 1080).await?;

        self.driver = Some(driver);
        self.log("INFO", "✅ Chrome WebDriver initialized successfully", None)
            .await;
        Ok(())
    }

    pub async fn execute_browser_action(&mut self, action: &BrowserAction) -> Result<String> {
        self.log(
            "INFO",
            &format!("🔧 Executing action: {}", action.action_type),
            Some("action"),
        )
        .await;

        match &self.mode {
            AutomationMode::Dom => self.execute_dom_action(action).await,
            AutomationMode::Vision => self.execute_vision_action(action).await,
            AutomationMode::Hybrid => self.execute_hybrid_action(action).await,
        }
    }

    async fn execute_dom_action(&mut self, action: &BrowserAction) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        self.log(
            "INFO",
            &format!("🔧 Executing action: {}", action.action_type),
            Some("dom"),
        )
        .await;

        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.url {
                    let clean_url = self.clean_url(url);
                    self.log(
                        "INFO",
                        &format!("🌐 Navigating to: {}", clean_url),
                        Some("navigate"),
                    )
                    .await;
                    driver.goto(&clean_url).await?;
                    Ok(format!("Navigated to {}", clean_url))
                } else {
                    Err(anyhow::anyhow!("No URL provided for navigate action"))
                }
            }
            "click" => {
                let selector = self.resolve_element_selector(action).await?;

                self.log(
                    "INFO",
                    &format!("🖱️ Clicking element: {}", selector),
                    Some("click"),
                )
                .await;

                let element = self.find_element_with_retry(driver, &selector, 5).await?;
                element.click().await?;
                Ok("Clicked element".to_string())
            }
            "type" => {
                if let Some(text) = &action.text {
                    let selector = self.resolve_element_selector(action).await?;

                    self.log(
                        "INFO",
                        &format!("⌨️ Typing '{}' into: {}", text, selector),
                        Some("type"),
                    )
                    .await;

                    let element = self.find_element_with_retry(driver, &selector, 5).await?;
                    element.clear().await?;
                    element.send_keys(text).await?;
                    Ok(format!("Typed '{}' into element", text))
                } else {
                    Err(anyhow::anyhow!("No text provided for type action"))
                }
            }
            "select" => {
                let selector = self.resolve_element_selector(action).await?;
                if let Some(value) = &action.text {
                    self.log(
                        "INFO",
                        &format!("📋 Selecting '{}' from: {}", value, selector),
                        Some("select"),
                    )
                    .await;

                    let select_element = self.find_element_with_retry(driver, &selector, 5).await?;

                    // Try different selection methods
                    let options = select_element.find_all(By::Tag("option")).await?;
                    let mut selected = false;

                    for option in options {
                        let option_text = option.text().await.unwrap_or_default();
                        let option_value = option
                            .get_attribute("value")
                            .await
                            .unwrap_or_default()
                            .unwrap_or_default();

                        if option_text.contains(value) || option_value.contains(value) {
                            option.click().await?;
                            selected = true;
                            break;
                        }
                    }

                    if !selected {
                        return Err(anyhow::anyhow!(
                            "Could not find option '{}' in select element",
                            value
                        ));
                    }

                    Ok(format!("Selected '{}' from: {}", value, selector))
                } else {
                    Err(anyhow::anyhow!("No value provided for select action"))
                }
            }
            "wait" => {
                if let Some(condition) = &action.wait_condition {
                    self.log(
                        "INFO",
                        &format!("⏱️ Waiting for condition: {}", condition),
                        Some("wait"),
                    )
                    .await;

                    match condition.as_str() {
                        "page_load" => {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                        }
                        "element_visible" => {
                            let selector = self.resolve_element_selector(action).await?;
                            self.wait_for_element_visible(driver, &selector, 10).await?;
                        }
                        "element_clickable" => {
                            let selector = self.resolve_element_selector(action).await?;
                            self.wait_for_element_visible(driver, &selector, 10).await?;
                            let element =
                                self.find_element_with_retry(driver, &selector, 3).await?;
                            if !element.is_enabled().await? {
                                return Err(anyhow::anyhow!(
                                    "Element is not clickable: {}",
                                    selector
                                ));
                            }
                        }
                        "page_stable" => {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            // Check document.readyState
                            let ready_state: String = driver
                                .execute("return document.readyState", vec![])
                                .await?
                                .convert()?;
                            if ready_state != "complete" {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                        "network_idle" => {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                        _ => {
                            // Try to parse as a number of seconds
                            if let Ok(seconds) = condition.parse::<u64>() {
                                tokio::time::sleep(Duration::from_secs(seconds)).await;
                            } else {
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                    Ok(format!("Wait completed for condition: {}", condition))
                } else {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    Ok("Default wait completed".to_string())
                }
            }
            "screenshot" => {
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("dom_screenshot_{}.png", timestamp);
                self.log(
                    "INFO",
                    &format!("📸 Taking DOM screenshot: {}", filename),
                    Some("screenshot"),
                )
                .await;
                let saved_filename = self.take_screenshot(Some(&filename)).await?;
                Ok(format!("Screenshot saved as {}", saved_filename))
            }
            _ => Err(anyhow::anyhow!(
                "Unknown DOM action type: {}",
                action.action_type
            )),
        }
    }

    // New method to resolve element selector using AI HTML analysis when needed
    async fn resolve_element_selector(&self, action: &BrowserAction) -> Result<String> {
        // If we have a selector, use it directly
        if let Some(selector) = &action.selector {
            return Ok(selector.clone());
        }

        // If we have an element description, use AI HTML analysis to get selector
        if let Some(description) = &action.element_description {
            self.log(
                "INFO",
                &format!("🔍 Using AI HTML analysis to find: {}", description),
                Some("ai_html"),
            )
            .await;

            // Determine element type from action
            let element_type = match action.action_type.as_str() {
                "type" => "input",
                "click" => "button",
                "select" => "select",
                _ => "element",
            };

            // Use AI to analyze HTML and get selector (much cheaper than coordinates)
            let selector = self
                .get_ai_selector_for_element("", description, element_type)
                .await?;

            self.log(
                "SUCCESS",
                &format!(
                    "✅ AI HTML analysis found selector '{}' for '{}'",
                    selector, description
                ),
                Some("ai_html"),
            )
            .await;

            Ok(selector)
        } else {
            Err(anyhow::anyhow!(
                "No selector or element description provided"
            ))
        }
    }

    async fn execute_vision_action(&mut self, action: &BrowserAction) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        self.log(
            "INFO",
            "🧠 Using AI DOM Inspection to find precise selectors",
            Some("ai_dom"),
        )
        .await;

        // Take screenshot for AI analysis
        let screenshot_data = driver.screenshot_as_png().await?;
        let screenshot_base64 = general_purpose::STANDARD.encode(&screenshot_data);

        // Save debug screenshot
        let debug_timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let debug_filename = format!("debug_ai_dom_{}.png", debug_timestamp);
        std::fs::write(&debug_filename, &screenshot_data)?;

        self.log(
            "INFO",
            &format!("📸 Debug screenshot saved: {}", debug_filename),
            Some("ai_dom"),
        )
        .await;

        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.url {
                    let clean_url = self.clean_url(url);
                    self.log(
                        "INFO",
                        &format!("🌐 Navigating to: {}", clean_url),
                        Some("navigate"),
                    )
                    .await;
                    driver.goto(&clean_url).await?;
                    tokio::time::sleep(Duration::from_millis(3000)).await;
                    let result_msg = format!("Navigated to {}", clean_url);
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("navigate"))
                        .await;
                    Ok(result_msg)
                } else {
                    Err(anyhow::anyhow!("Navigate action requires URL"))
                }
            }

            "type" => {
                if let Some(text) = &action.text {
                    let element_description = action
                        .element_description
                        .as_deref()
                        .unwrap_or("input field");

                    self.log(
                        "INFO",
                        &format!(
                            "🧠 AI DOM Inspection: Finding selector for '{}'",
                            element_description
                        ),
                        Some("ai_dom"),
                    )
                    .await;

                    // Get AI-generated selector
                    let selector = self
                        .get_ai_selector_for_element(
                            &screenshot_base64,
                            element_description,
                            "input",
                        )
                        .await?;

                    self.log(
                        "INFO",
                        &format!("🎯 AI found selector: {}", selector),
                        Some("ai_dom"),
                    )
                    .await;

                    // Use traditional DOM automation with AI-found selector
                    let dom_action = BrowserAction {
                        action_type: "type".to_string(),
                        selector: Some(selector),
                        element_description: None,
                        text: Some(text.clone()),
                        url: None,
                        wait_condition: None,
                        screenshot: false,
                    };

                    match self.execute_dom_action(&dom_action).await {
                        Ok(result) => {
                            self.log(
                                "SUCCESS",
                                &format!("✅ AI DOM typed '{}' successfully", text),
                                Some("ai_dom"),
                            )
                            .await;
                            Ok(result)
                        }
                        Err(e) => {
                            self.log(
                                "ERROR",
                                &format!("❌ AI DOM typing failed: {}", e),
                                Some("ai_dom"),
                            )
                            .await;
                            Err(e)
                        }
                    }
                } else {
                    Err(anyhow::anyhow!("Type action requires text"))
                }
            }

            "click" => {
                let element_description = action
                    .element_description
                    .as_deref()
                    .unwrap_or("clickable element");

                self.log(
                    "INFO",
                    &format!(
                        "🧠 AI DOM Inspection: Finding selector for '{}'",
                        element_description
                    ),
                    Some("ai_dom"),
                )
                .await;

                // Get AI-generated selector
                let selector = self
                    .get_ai_selector_for_element(&screenshot_base64, element_description, "button")
                    .await?;

                self.log(
                    "INFO",
                    &format!("🎯 AI found selector: {}", selector),
                    Some("ai_dom"),
                )
                .await;

                // Use traditional DOM automation with AI-found selector
                let dom_action = BrowserAction {
                    action_type: "click".to_string(),
                    selector: Some(selector),
                    element_description: None,
                    text: None,
                    url: None,
                    wait_condition: None,
                    screenshot: false,
                };

                match self.execute_dom_action(&dom_action).await {
                    Ok(result) => {
                        self.log("SUCCESS", "✅ AI DOM click successful", Some("ai_dom"))
                            .await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.log(
                            "ERROR",
                            &format!("❌ AI DOM click failed: {}", e),
                            Some("ai_dom"),
                        )
                        .await;
                        Err(e)
                    }
                }
            }

            "wait" => {
                // Wait actions don't need AI inspection - use DOM implementation
                self.log(
                    "INFO",
                    "⏱️ AI DOM mode: Using standard wait implementation",
                    Some("ai_dom"),
                )
                .await;
                self.execute_dom_action(action).await
            }

            "screenshot" => {
                self.log(
                    "INFO",
                    "📸 Taking AI DOM enhanced screenshot",
                    Some("screenshot"),
                )
                .await;
                let screenshot_data = driver.screenshot_as_png().await?;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("ai_dom_screenshot_{}.png", timestamp);
                std::fs::write(&filename, screenshot_data)?;
                let result_msg = format!("AI DOM screenshot saved as {}", filename);
                self.log("SUCCESS", &format!("✅ {}", result_msg), Some("screenshot"))
                    .await;
                Ok(result_msg)
            }

            _ => {
                let error_msg = format!(
                    "Unsupported action type for AI DOM mode: {}",
                    action.action_type
                );
                self.log("ERROR", &format!("❌ {}", error_msg), Some("ai_dom"))
                    .await;
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn execute_hybrid_action(&mut self, action: &BrowserAction) -> Result<String> {
        self.log(
            "INFO",
            "🔀 Using Hybrid mode: Trying DOM first, falling back to Vision",
            Some("hybrid"),
        )
        .await;

        // Try DOM approach first
        match self.execute_dom_action(action).await {
            Ok(result) => {
                self.log("SUCCESS", "✅ DOM approach succeeded", Some("hybrid"))
                    .await;
                Ok(result)
            }
            Err(_dom_error) => {
                self.log(
                    "WARN",
                    "⚠️ DOM approach failed, trying Vision approach",
                    Some("hybrid"),
                )
                .await;
                self.execute_vision_action(action).await
            }
        }
    }

    async fn get_element_coordinates_via_vision(
        &self,
        screenshot_base64: &str,
        prompt: &str,
    ) -> Result<(i32, i32)> {
        if let Some(api_key) = &self.vision_api_key {
            self.log(
                "INFO",
                "🧠 Analyzing screenshot with AI Vision",
                Some("vision"),
            )
            .await;

            // Make API call to vision model (OpenAI GPT-4V, Claude Vision, etc.)
            let coordinates = self
                .call_vision_api(api_key, screenshot_base64, prompt)
                .await?;

            self.log(
                "SUCCESS",
                &format!(
                    "✅ AI Vision found coordinates: ({}, {})",
                    coordinates.0, coordinates.1
                ),
                Some("vision"),
            )
            .await;
            Ok(coordinates)
        } else {
            Err(anyhow::anyhow!("Vision API key not configured"))
        }
    }

    async fn call_vision_api(
        &self,
        api_key: &str,
        screenshot_base64: &str,
        prompt: &str,
    ) -> Result<(i32, i32)> {
        self.log(
            "INFO",
            &format!(
                "🤖 Calling OpenRouter {} for vision analysis",
                self.vision_model
            ),
            Some("vision"),
        )
        .await;

        let client = reqwest::Client::new();

        // Create the OpenRouter API payload
        let payload = json!({
            "model": self.vision_model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": format!("{}\n\nIMPORTANT: You must respond with ONLY the coordinates in 'x,y' format (e.g., '450,320'). Do not include any other text, explanations, or formatting.", prompt)
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", screenshot_base64)
                        }
                    }
                ]
            }],
            "max_tokens": 50,
            "temperature": 0.1
        });

        self.log(
            "INFO",
            "📡 Sending screenshot to OpenRouter Vision API...",
            Some("vision"),
        )
        .await;

        // Make the API call to OpenRouter
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://ai-ac-automation.local") // Optional: for OpenRouter analytics
            .header("X-Title", "AI Browser Automation") // Optional: for OpenRouter analytics
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("OpenRouter API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            self.log(
                "ERROR",
                &format!("❌ OpenRouter API error: {} - {}", status, error_text),
                Some("vision"),
            )
            .await;
            return Err(anyhow::anyhow!(
                "OpenRouter API returned error: {}",
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter response: {}", e))?;

        // Extract the coordinates from the response
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or("");

        self.log(
            "INFO",
            &format!("🧠 OpenRouter Vision response: '{}'", content.trim()),
            Some("vision"),
        )
        .await;

        // Parse coordinates from the response
        let coordinates = self.parse_coordinates_from_response(content).await?;

        self.log(
            "SUCCESS",
            &format!(
                "✅ OpenRouter Vision found coordinates: ({}, {})",
                coordinates.0, coordinates.1
            ),
            Some("vision"),
        )
        .await;

        Ok(coordinates)
    }

    async fn parse_coordinates_from_response(&self, response: &str) -> Result<(i32, i32)> {
        let cleaned = response.trim();

        self.log(
            "INFO",
            &format!("🔍 Parsing coordinates from AI response: '{}'", cleaned),
            Some("vision"),
        )
        .await;

        // Try different coordinate formats
        let patterns = vec![
            // Standard x,y format
            r"(\d+),\s*(\d+)",
            // Parentheses format
            r"\((\d+),\s*(\d+)\)",
            // Bracket format
            r"\[(\d+),\s*(\d+)\]",
            // x: y: format
            r"x:\s*(\d+).*?y:\s*(\d+)",
            // More flexible format
            r"(\d+)\s*,\s*(\d+)",
        ];

        for pattern in patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(cleaned) {
                    if let (Some(x_match), Some(y_match)) = (captures.get(1), captures.get(2)) {
                        if let (Ok(x), Ok(y)) = (
                            x_match.as_str().parse::<i32>(),
                            y_match.as_str().parse::<i32>(),
                        ) {
                            // Validate coordinates are within reasonable screen bounds
                            if x >= 0 && x <= 3840 && y >= 0 && y <= 2160 {
                                self.log(
                                    "SUCCESS",
                                    &format!(
                                        "✅ Parsed coordinates using pattern '{}': ({}, {})",
                                        pattern, x, y
                                    ),
                                    Some("vision"),
                                )
                                .await;
                                return Ok((x, y));
                            }
                        }
                    }
                }
            }
        }

        // If no valid coordinates found, try to extract first two numbers
        let numbers: Vec<i32> = cleaned
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|s| s.parse().ok())
            .collect();

        if numbers.len() >= 2 {
            let x = numbers[0];
            let y = numbers[1];
            if x >= 0 && x <= 3840 && y >= 0 && y <= 2160 {
                self.log(
                    "WARN",
                    &format!("⚠️ Fallback coordinate parsing: ({}, {})", x, y),
                    Some("vision"),
                )
                .await;
                return Ok((x, y));
            }
        }

        // CRITICAL: Don't fall back to center - return error instead
        self.log(
            "ERROR",
            &format!(
                "❌ VISION FAILED: Could not parse valid coordinates from: '{}'. This action will be skipped to prevent random clicking.",
                cleaned
            ),
            Some("vision"),
        )
        .await;

        Err(anyhow::anyhow!(
            "Vision AI failed to provide valid coordinates. Response was: '{}'",
            cleaned
        ))
    }

    async fn click_at_coordinates(&self, driver: &WebDriver, x: i32, y: i32) -> Result<()> {
        self.log(
            "INFO",
            &format!("🖱️ Clicking at coordinates ({}, {})", x, y),
            Some("vision"),
        )
        .await;

        driver
            .action_chain()
            .move_to(x as i64, y as i64)
            .click()
            .perform()
            .await?;

        tokio::time::sleep(Duration::from_millis(1000)).await;
        Ok(())
    }

    async fn find_element_with_retry(
        &self,
        driver: &WebDriver,
        selector: &str,
        max_retries: u32,
    ) -> Result<WebElement> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            if attempt > 1 {
                self.log(
                    "INFO",
                    &format!("🔄 Retry attempt {} for selector: {}", attempt, selector),
                    Some("retry"),
                )
                .await;
            }

            let element_result = self.try_find_element(driver, selector).await;

            match element_result {
                Ok(element) => {
                    if attempt > 1 {
                        self.log(
                            "SUCCESS",
                            &format!("✅ Element found on attempt {}", attempt),
                            Some("retry"),
                        )
                        .await;
                    }
                    return Ok(element);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                    }
                }
            }
        }

        let error_msg = format!("Failed to find element: {}", selector);
        self.log("ERROR", &format!("❌ {}", error_msg), Some("find"))
            .await;
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!(error_msg)))
    }

    async fn try_find_element(&self, driver: &WebDriver, selector: &str) -> Result<WebElement> {
        // Try different selector strategies
        if selector.starts_with("//") {
            // XPath selector
            driver
                .find(By::XPath(selector))
                .await
                .map_err(|e| anyhow::anyhow!("XPath selector failed: {}", e))
        } else if selector.starts_with("#") {
            // ID selector
            let id = &selector[1..];
            driver
                .find(By::Id(id))
                .await
                .map_err(|e| anyhow::anyhow!("ID selector failed: {}", e))
        } else if selector.starts_with(".") {
            // Class selector
            let class = &selector[1..];
            driver
                .find(By::ClassName(class))
                .await
                .map_err(|e| anyhow::anyhow!("Class selector failed: {}", e))
        } else if selector.contains("[") && selector.contains("]") {
            // Attribute selector or CSS selector
            driver
                .find(By::Css(selector))
                .await
                .map_err(|e| anyhow::anyhow!("CSS selector failed: {}", e))
        } else if selector.starts_with("button")
            || selector.starts_with("input")
            || selector.starts_with("a")
        {
            // CSS selector for HTML elements
            driver
                .find(By::Css(selector))
                .await
                .map_err(|e| anyhow::anyhow!("CSS selector failed: {}", e))
        } else {
            // Try as text content
            let xpath = format!("//*[contains(text(), '{}')]", selector);
            driver
                .find(By::XPath(&xpath))
                .await
                .map_err(|e| anyhow::anyhow!("Text content selector failed: {}", e))
        }
    }

    async fn wait_for_element_visible(
        &self,
        driver: &WebDriver,
        selector: &str,
        timeout_seconds: u64,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);

        while start_time.elapsed() < timeout {
            if let Ok(element) = self.try_find_element(driver, selector).await {
                if element.is_displayed().await.unwrap_or(false) {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(anyhow::anyhow!(
            "Element '{}' did not become visible within {} seconds",
            selector,
            timeout_seconds
        ))
    }

    pub async fn take_screenshot(&mut self, filename: Option<&str>) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        let screenshot = driver.screenshot_as_png().await?;
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        let final_filename = match filename {
            Some(fname) => fname.to_string(),
            None => format!("screenshot_{}.png", timestamp),
        };

        std::fs::write(&final_filename, screenshot)?;
        Ok(final_filename)
    }

    pub async fn execute_assertion(&mut self, assertion: &str) -> Result<()> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        let parts: Vec<&str> = assertion.split(": ").collect();
        if parts.len() != 2 {
            let error_msg = format!("Invalid assertion format: {}", assertion);
            self.log("ERROR", &format!("❌ {}", error_msg), Some("assertion"))
                .await;
            return Err(anyhow::anyhow!(error_msg));
        }

        let assertion_type = parts[0];
        let assertion_value = parts[1];

        self.log(
            "INFO",
            &format!("✓ Checking assertion: {}", assertion),
            Some("assertion"),
        )
        .await;

        let result = match assertion_type {
            "login_success" => {
                // Comprehensive login success detection
                let current_url = driver.current_url().await?.to_string();
                let page_source = driver.source().await?;

                // Check multiple indicators of login success
                let url_changed = !current_url.contains("/login") && !current_url.contains("/auth");
                let has_dashboard = current_url.contains("/dashboard")
                    || current_url.contains("/home")
                    || current_url.contains("/app");
                let no_login_form =
                    !page_source.contains("type=\"password\"") || !page_source.contains("login");
                let has_success_indicators = page_source.contains("dashboard")
                    || page_source.contains("welcome")
                    || page_source.contains("logout");

                self.log(
                    "INFO",
                    &format!(
                        "🔍 Login check - URL: {}, Changed: {}, Dashboard: {}, No form: {}, Success indicators: {}",
                        current_url, url_changed, has_dashboard, no_login_form, has_success_indicators
                    ),
                    Some("assertion"),
                )
                .await;

                if url_changed || has_dashboard || (no_login_form && has_success_indicators) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Login failed - still on login page. Current URL: {}",
                        current_url
                    ))
                }
            }
            "login_failure" => {
                // Check if we're still on login page (indicating failure)
                let current_url = driver.current_url().await?.to_string();
                let page_source = driver.source().await?;

                if current_url.contains("/login")
                    || current_url.contains("/auth")
                    || page_source.contains("type=\"password\"")
                {
                    Ok(()) // We're still on login page, so login failed as expected
                } else {
                    Err(anyhow::anyhow!(
                        "Expected login failure but login succeeded. Current URL: {}",
                        current_url
                    ))
                }
            }
            "element_visible" => {
                let element = self
                    .find_element_with_retry(driver, assertion_value, 5)
                    .await;
                match element {
                    Ok(el) => {
                        let is_displayed = el.is_displayed().await?;
                        if is_displayed {
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Element '{}' exists but is not visible",
                                assertion_value
                            ))
                        }
                    }
                    Err(_) => Err(anyhow::anyhow!(
                        "Element '{}' is not visible",
                        assertion_value
                    )),
                }
            }
            "element_not_visible" => {
                let element = driver.find(By::Css(assertion_value)).await;
                match element {
                    Ok(el) => {
                        let is_displayed = el.is_displayed().await.unwrap_or(false);
                        if !is_displayed {
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Element '{}' should not be visible",
                                assertion_value
                            ))
                        }
                    }
                    Err(_) => Ok(()), // Element not found, so it's not visible
                }
            }
            "url_contains" => {
                let current_url = driver.current_url().await?.to_string();
                if current_url.contains(assertion_value) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Current URL '{}' does not contain '{}'",
                        current_url,
                        assertion_value
                    ))
                }
            }
            "page_content_contains" => {
                let page_source = driver.source().await?;
                if page_source.contains(assertion_value) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Page content does not contain '{}'",
                        assertion_value
                    ))
                }
            }
            "page_title_contains" => {
                let title = driver.title().await?;
                if title.contains(assertion_value) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Page title '{}' does not contain '{}'",
                        title,
                        assertion_value
                    ))
                }
            }
            _ => Err(anyhow::anyhow!(
                "Unknown assertion type: {}",
                assertion_type
            )),
        };

        match &result {
            Ok(_) => {
                self.log(
                    "SUCCESS",
                    &format!("✅ Assertion passed: {}", assertion),
                    Some("assertion"),
                )
                .await
            }
            Err(e) => {
                self.log(
                    "ERROR",
                    &format!("❌ Assertion failed: {}", e),
                    Some("assertion"),
                )
                .await
            }
        }

        result
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(driver) = self.driver.take() {
            self.log("INFO", "🔄 Closing browser", None).await;
            driver.quit().await?;
            self.log("SUCCESS", "✅ Browser closed successfully", None)
                .await;
        }
        Ok(())
    }

    pub async fn get_current_url(&self) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;
        Ok(driver.current_url().await?.to_string())
    }

    pub async fn get_title(&self) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;
        Ok(driver.title().await?)
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    fn clean_url(&self, url: &str) -> String {
        // Remove surrounding quotes and whitespace
        url.trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string()
    }

    async fn get_ai_selector_for_element(
        &self,
        screenshot_base64: &str,
        element_description: &str,
        element_type: &str,
    ) -> Result<String> {
        self.log(
            "INFO",
            "🧠 Using AI to analyze page HTML structure",
            Some("ai_html"),
        )
        .await;

        // Get page HTML source (much cheaper than screenshots)
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        let page_html = driver.source().await?;

        // Truncate HTML to avoid huge token costs (keep first 8000 chars which usually contains forms)
        let truncated_html = if page_html.len() > 8000 {
            format!("{}...[truncated]", &page_html[..8000])
        } else {
            page_html
        };

        let vision_api_key = self
            .vision_api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("API key not configured for HTML analysis"))?;

        let vision_model = &self.vision_model;

        let prompt = format!(
            r#"You are a CSS selector generator. Analyze this HTML and return ONLY a valid CSS selector for: {}

HTML snippet:
{}

CRITICAL RULES:
1. Return ONLY a CSS selector - NO explanations, NO text, NO markdown
2. If you cannot find the element, return one of these fallback selectors:
   - For login buttons: button[type="submit"]
   - For email inputs: input[type="email"] 
   - For password inputs: input[type="password"]
   - For dashboard elements: .dashboard, #dashboard, .main-content, .content
   - For general elements: div, span, *

Examples of VALID responses:
input[type="email"]
button[type="submit"]
.dashboard
#main-content

Examples of INVALID responses (DO NOT DO THIS):
"I cannot find the element"
"Based on the HTML provided..."
```css
selector
```

Return only the selector:"#,
            element_description, truncated_html
        );

        let payload = json!({
            "model": vision_model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 100,
            "temperature": 0.1
        });

        let client = reqwest::Client::new();
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", vision_api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://ac-automation.local")
            .header("X-Title", "AC Automation HTML Analysis")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Fallback to common selectors if AI fails
            self.log(
                "WARN",
                "⚠️ AI HTML analysis failed, using fallback selectors",
                Some("ai_html"),
            )
            .await;

            return Ok(self.get_fallback_selector(element_description, element_type));
        }

        let response_json: serde_json::Value = response.json().await?;

        if let Some(content) = response_json["choices"][0]["message"]["content"].as_str() {
            let selector = content.trim().to_string();

            self.log(
                "INFO",
                &format!("🧠 AI HTML analysis result: '{}'", selector),
                Some("ai_html"),
            )
            .await;

            // Clean and validate the selector
            let cleaned_selector = self.extract_valid_selector(&selector, element_description);

            // Check if it's a valid selector (not an explanation)
            if cleaned_selector.len() > 100
                || cleaned_selector.contains("cannot find")
                || cleaned_selector.contains("provided")
            {
                self.log(
                    "WARN",
                    "⚠️ AI returned explanation instead of selector, using fallback",
                    Some("ai_html"),
                )
                .await;
                return Ok(self.get_fallback_selector(element_description, ""));
            }

            self.log(
                "SUCCESS",
                &format!("✅ AI HTML found selector: {}", cleaned_selector),
                Some("ai_html"),
            )
            .await;

            Ok(cleaned_selector)
        } else {
            self.log(
                "WARN",
                "⚠️ AI HTML analysis returned no result, using fallback",
                Some("ai_html"),
            )
            .await;
            Ok(self.get_fallback_selector(element_description, ""))
        }
    }

    fn get_fallback_selector(&self, element_description: &str, element_type: &str) -> String {
        let desc = element_description.to_lowercase();

        // Smart fallback selectors based on description
        if desc.contains("email") {
            "input[type=\"email\"]".to_string()
        } else if desc.contains("password") {
            "input[type=\"password\"]".to_string()
        } else if desc.contains("login") || desc.contains("submit") || desc.contains("sign in") {
            "button[type=\"submit\"]".to_string()
        } else if element_type == "input" {
            "input".to_string()
        } else if element_type == "button" {
            "button".to_string()
        } else {
            "*".to_string()
        }
    }

    fn extract_valid_selector(&self, selector: &str, element_description: &str) -> String {
        // Clean and validate the selector
        let cleaned_selector = selector
            .trim()
            .trim_matches('"')
            .trim_matches('`')
            .replace("CSS:", "")
            .replace("XPath:", "")
            .trim()
            .to_string();

        // Validate the selector isn't jQuery syntax or an explanation
        if cleaned_selector.contains(":contains(")
            || cleaned_selector.len() > 100
            || cleaned_selector.contains("cannot find")
            || cleaned_selector.contains("provided")
            || cleaned_selector.contains("Based on")
        {
            return self.get_fallback_selector(element_description, "");
        }

        cleaned_selector
    }
}

impl Drop for ChromeAutomationEngine {
    fn drop(&mut self) {
        if self.driver.is_some() {
            // Can't await in Drop, but we can print a warning
            if self.verbose {
                println!("⚠️ ChromeAutomationEngine dropped with active WebDriver session");
            }
        }
    }
}
