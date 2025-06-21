use anyhow::Result;
use automation_api::BrowserAction;
use base64::{engine::general_purpose, Engine as _};
use regex;
use reqwest;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::components::SelectElement;
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
    timeout: Duration,
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
            timeout: Duration::from_secs(30),
            logs: Arc::new(Mutex::new(Vec::new())),
            verbose: true,
            mode: AutomationMode::Dom, // Default to DOM mode
            vision_api_key: None,
            vision_model: "gpt-4o".to_string(), // GPT-4 with vision
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
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

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

        // Existing DOM-based implementation
        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.url {
                    self.log(
                        "INFO",
                        &format!("🌐 Navigating to: {}", url),
                        Some("navigate"),
                    )
                    .await;
                    driver.goto(url).await?;

                    tokio::time::sleep(Duration::from_millis(2000)).await;

                    let current_url = driver.current_url().await?.to_string();
                    let result_msg = format!("Navigated to {}", current_url);
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("navigate"))
                        .await;
                    Ok(result_msg)
                } else {
                    let error_msg = "Navigate action requires URL";
                    self.log("ERROR", &format!("❌ {}", error_msg), Some("navigate"))
                        .await;
                    Err(anyhow::anyhow!(error_msg))
                }
            }
            "click" => {
                if let Some(selector) = &action.selector {
                    self.log(
                        "INFO",
                        &format!("🖱️ Finding element to click: {}", selector),
                        Some("click"),
                    )
                    .await;

                    let element = self.find_element_with_retry(driver, selector, 10).await?;

                    self.log("INFO", "📍 Scrolling element into view", Some("click"))
                        .await;
                    driver
                        .execute(
                            "arguments[0].scrollIntoView({block: 'center'});",
                            vec![element.to_json()?],
                        )
                        .await?;
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    self.log(
                        "INFO",
                        &format!("👆 Clicking element: {}", selector),
                        Some("click"),
                    )
                    .await;
                    element.click().await?;
                    tokio::time::sleep(Duration::from_millis(1000)).await;

                    let result_msg = format!("Clicked element: {}", selector);
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("click"))
                        .await;
                    Ok(result_msg)
                } else {
                    let error_msg = "Click action requires selector";
                    self.log("ERROR", &format!("❌ {}", error_msg), Some("click"))
                        .await;
                    Err(anyhow::anyhow!(error_msg))
                }
            }
            "type" => {
                if let Some(selector) = &action.selector {
                    if let Some(text) = &action.text {
                        self.log(
                            "INFO",
                            &format!("⌨️ Finding input field: {}", selector),
                            Some("type"),
                        )
                        .await;

                        let element = self.find_element_with_retry(driver, selector, 10).await?;

                        self.log("INFO", "🧹 Clearing existing text", Some("type"))
                            .await;
                        element.clear().await?;
                        tokio::time::sleep(Duration::from_millis(300)).await;

                        self.log(
                            "INFO",
                            &format!("⌨️ Typing '{}' into: {}", text, selector),
                            Some("type"),
                        )
                        .await;
                        element.send_keys(text).await?;
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        let result_msg = format!("Typed '{}' into {}", text, selector);
                        self.log("SUCCESS", &format!("✅ {}", result_msg), Some("type"))
                            .await;
                        Ok(result_msg)
                    } else {
                        let error_msg = "Type action requires text";
                        self.log("ERROR", &format!("❌ {}", error_msg), Some("type"))
                            .await;
                        Err(anyhow::anyhow!(error_msg))
                    }
                } else {
                    let error_msg = "Type action requires selector";
                    self.log("ERROR", &format!("❌ {}", error_msg), Some("type"))
                        .await;
                    Err(anyhow::anyhow!(error_msg))
                }
            }
            "screenshot" => {
                self.log("INFO", "📸 Taking screenshot", Some("screenshot"))
                    .await;
                let screenshot = driver.screenshot_as_png().await?;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("screenshot_{}.png", timestamp);
                std::fs::write(&filename, screenshot)?;
                let result_msg = format!("Screenshot saved as {}", filename);
                self.log("SUCCESS", &format!("✅ {}", result_msg), Some("screenshot"))
                    .await;
                Ok(result_msg)
            }
            "wait" => {
                if let Some(condition) = &action.wait_condition {
                    self.log(
                        "INFO",
                        &format!("⏱️ Waiting for condition: {}", condition),
                        Some("wait"),
                    )
                    .await;

                    let result = match condition.as_str() {
                        "page_load" => {
                            // Wait for page to load completely
                            tokio::time::sleep(Duration::from_millis(3000)).await;
                            self.log("SUCCESS", "✅ Page load wait completed", Some("wait"))
                                .await;
                            Ok("Page load wait completed".to_string())
                        }
                        "element_visible" => {
                            if let Some(selector) = &action.selector {
                                match self.wait_for_element_visible(driver, selector, 10).await {
                                    Ok(_) => {
                                        let msg = format!("Element '{}' is now visible", selector);
                                        self.log("SUCCESS", &format!("✅ {}", msg), Some("wait"))
                                            .await;
                                        Ok(msg)
                                    }
                                    Err(e) => {
                                        let msg = format!(
                                            "Element '{}' did not become visible: {}",
                                            selector, e
                                        );
                                        self.log("ERROR", &format!("❌ {}", msg), Some("wait"))
                                            .await;
                                        Err(anyhow::anyhow!(msg))
                                    }
                                }
                            } else {
                                // Generic wait if no selector provided
                                tokio::time::sleep(Duration::from_millis(2000)).await;
                                self.log(
                                    "SUCCESS",
                                    "✅ Element visibility wait completed",
                                    Some("wait"),
                                )
                                .await;
                                Ok("Element visibility wait completed".to_string())
                            }
                        }
                        "element_clickable" => {
                            if let Some(selector) = &action.selector {
                                // Wait for element to be both visible and clickable
                                match self.find_element_with_retry(driver, selector, 10).await {
                                    Ok(element) => {
                                        // Additional check for clickability
                                        let is_enabled =
                                            element.is_enabled().await.unwrap_or(false);
                                        if is_enabled {
                                            let msg =
                                                format!("Element '{}' is now clickable", selector);
                                            self.log(
                                                "SUCCESS",
                                                &format!("✅ {}", msg),
                                                Some("wait"),
                                            )
                                            .await;
                                            Ok(msg)
                                        } else {
                                            let msg = format!(
                                                "Element '{}' is visible but not clickable",
                                                selector
                                            );
                                            self.log("WARN", &format!("⚠️ {}", msg), Some("wait"))
                                                .await;
                                            Ok(msg)
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!(
                                            "Element '{}' not found for clickability check: {}",
                                            selector, e
                                        );
                                        self.log("ERROR", &format!("❌ {}", msg), Some("wait"))
                                            .await;
                                        Err(anyhow::anyhow!(msg))
                                    }
                                }
                            } else {
                                tokio::time::sleep(Duration::from_millis(1500)).await;
                                self.log("SUCCESS", "✅ Clickability wait completed", Some("wait"))
                                    .await;
                                Ok("Clickability wait completed".to_string())
                            }
                        }
                        "page_stable" => {
                            // Wait for page to be stable (no more loading)
                            self.log("INFO", "🔄 Waiting for page to stabilize", Some("wait"))
                                .await;

                            // Wait for any loading indicators to disappear
                            tokio::time::sleep(Duration::from_millis(2500)).await;

                            // Check if page is still loading
                            let ready_state_result =
                                driver.execute("return document.readyState;", vec![]).await;

                            let is_complete = match ready_state_result {
                                Ok(script_result) => {
                                    // Try to convert the result to string and check if it's "complete"
                                    script_result.json().as_str().unwrap_or("unknown") == "complete"
                                }
                                Err(_) => true, // Assume complete if we can't check
                            };

                            if is_complete {
                                self.log(
                                    "SUCCESS",
                                    "✅ Page stabilized successfully",
                                    Some("wait"),
                                )
                                .await;
                                Ok("Page stabilized successfully".to_string())
                            } else {
                                self.log("WARN", "⚠️ Page may still be loading", Some("wait"))
                                    .await;
                                Ok("Page stabilization attempted".to_string())
                            }
                        }
                        _ => {
                            // Generic wait for unknown conditions (fallback to time-based)
                            let wait_time = match condition.parse::<u64>() {
                                Ok(seconds) => seconds * 1000, // Convert seconds to milliseconds
                                Err(_) => 2000,                // Default 2 seconds
                            };

                            tokio::time::sleep(Duration::from_millis(wait_time)).await;
                            let msg =
                                format!("Generic wait completed for condition: {}", condition);
                            self.log("SUCCESS", &format!("✅ {}", msg), Some("wait"))
                                .await;
                            Ok(msg)
                        }
                    };

                    result
                } else {
                    // Default wait if no condition specified
                    self.log("INFO", "⏱️ Performing default wait (2s)", Some("wait"))
                        .await;
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                    let result_msg = "Default wait completed";
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("wait"))
                        .await;
                    Ok(result_msg.to_string())
                }
            }
            _ => {
                let error_msg = format!("Unknown action type: {}", action.action_type);
                self.log("ERROR", &format!("❌ {}", error_msg), Some("unknown"))
                    .await;
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    async fn execute_vision_action(&mut self, action: &BrowserAction) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        self.log(
            "INFO",
            "👁️ Using AI Vision to understand the page",
            Some("vision"),
        )
        .await;

        // Take screenshot for vision analysis
        let screenshot_data = driver.screenshot_as_png().await?;
        let screenshot_base64 = general_purpose::STANDARD.encode(&screenshot_data);

        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.url {
                    self.log(
                        "INFO",
                        &format!("🌐 Navigating to: {}", url),
                        Some("navigate"),
                    )
                    .await;
                    driver.goto(url).await?;
                    tokio::time::sleep(Duration::from_millis(3000)).await;

                    let result_msg = format!("Navigated to {}", url);
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("navigate"))
                        .await;
                    Ok(result_msg)
                } else {
                    Err(anyhow::anyhow!("Navigate action requires URL"))
                }
            }
            "click" => {
                let description = action.selector.as_deref().unwrap_or("the element");
                self.log(
                    "INFO",
                    &format!("👁️ AI Vision: Looking for '{}' to click", description),
                    Some("vision"),
                )
                .await;

                let coordinates = self
                    .get_element_coordinates_via_vision(
                        &screenshot_base64,
                        &format!("Find the coordinates to click on: {}", description),
                    )
                    .await?;

                self.log(
                    "INFO",
                    &format!(
                        "📍 AI found coordinates: ({}, {})",
                        coordinates.0, coordinates.1
                    ),
                    Some("vision"),
                )
                .await;

                // Perform click at coordinates
                self.click_at_coordinates(driver, coordinates.0, coordinates.1)
                    .await?;

                let result_msg = format!(
                    "Vision-clicked at coordinates ({}, {})",
                    coordinates.0, coordinates.1
                );
                self.log("SUCCESS", &format!("✅ {}", result_msg), Some("vision"))
                    .await;
                Ok(result_msg)
            }
            "type" => {
                if let Some(text) = &action.text {
                    let description = action.selector.as_deref().unwrap_or("the input field");
                    self.log(
                        "INFO",
                        &format!("👁️ AI Vision: Looking for '{}' to type into", description),
                        Some("vision"),
                    )
                    .await;

                    let coordinates = self
                        .get_element_coordinates_via_vision(
                            &screenshot_base64,
                            &format!("Find the coordinates of the input field: {}", description),
                        )
                        .await?;

                    // Click on the input field first
                    self.click_at_coordinates(driver, coordinates.0, coordinates.1)
                        .await?;
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    // Clear existing text and type new text
                    self.log(
                        "INFO",
                        &format!(
                            "⌨️ Vision-typing '{}' at coordinates ({}, {})",
                            text, coordinates.0, coordinates.1
                        ),
                        Some("vision"),
                    )
                    .await;
                    driver
                        .action_chain()
                        .key_down(Key::Control)
                        .send_keys("a")
                        .key_up(Key::Control)
                        .perform()
                        .await?;
                    driver.action_chain().send_keys(text).perform().await?;

                    let result_msg = format!(
                        "Vision-typed '{}' at coordinates ({}, {})",
                        text, coordinates.0, coordinates.1
                    );
                    self.log("SUCCESS", &format!("✅ {}", result_msg), Some("vision"))
                        .await;
                    Ok(result_msg)
                } else {
                    Err(anyhow::anyhow!("Type action requires text"))
                }
            }
            "screenshot" => {
                self.log(
                    "INFO",
                    "📸 Taking vision-enhanced screenshot",
                    Some("screenshot"),
                )
                .await;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("vision_screenshot_{}.png", timestamp);
                std::fs::write(&filename, screenshot_data)?;

                let result_msg = format!("Vision screenshot saved as {}", filename);
                self.log("SUCCESS", &format!("✅ {}", result_msg), Some("screenshot"))
                    .await;
                Ok(result_msg)
            }
            "wait" => {
                // Vision mode handles wait actions the same as DOM mode
                self.execute_dom_action(action).await
            }
            _ => {
                let error_msg = format!("Vision mode: Unknown action type: {}", action.action_type);
                self.log("ERROR", &format!("❌ {}", error_msg), Some("vision"))
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
                                    "INFO",
                                    &format!(
                                        "📍 Parsed coordinates using pattern '{}': ({}, {})",
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

        // Ultimate fallback - return center of screen
        self.log(
            "ERROR",
            &format!(
                "❌ Could not parse coordinates from: '{}'. Using screen center as fallback.",
                cleaned
            ),
            Some("vision"),
        )
        .await;

        Ok((960, 540)) // Center of 1920x1080 screen
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
