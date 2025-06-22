use anyhow::Result;
use automation_api::BrowserAction;
use base64::{engine::general_purpose, Engine as _};
use regex;
use reqwest;
use serde_json::json;
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

#[derive(Debug, Clone)]
pub enum VisionStrategy {
    DomInspection,   // AI analyzes HTML to find selectors (cheaper, faster)
    CoordinateBased, // AI analyzes screenshot to find coordinates (more robust)
    Adaptive,        // Tries DOM first, falls back to coordinates
}

pub struct ChromeAutomationEngine {
    driver: Option<WebDriver>,
    headless: bool,
    logs: Arc<Mutex<Vec<AutomationLog>>>,
    verbose: bool,
    mode: AutomationMode,
    vision_api_key: Option<String>,
    vision_model: String,
    vision_strategy: VisionStrategy,
}

impl ChromeAutomationEngine {
    pub fn new(headless: bool) -> Self {
        Self {
            driver: None,
            headless,
            logs: Arc::new(Mutex::new(Vec::new())),
            verbose: true,
            mode: AutomationMode::Dom,
            vision_api_key: None,
            vision_model: "anthropic/claude-3-5-sonnet-20241022".to_string(), // Latest Claude 3.5 Sonnet
            vision_strategy: VisionStrategy::Adaptive,
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

    pub fn with_vision_strategy(mut self, strategy: VisionStrategy) -> Self {
        self.vision_strategy = strategy;
        self
    }

    pub fn set_vision_strategy(&mut self, strategy: VisionStrategy) {
        self.vision_strategy = strategy;
    }

    // New method to set optimal model for specific strategy
    pub fn with_optimal_model_for_strategy(&mut self, strategy: &VisionStrategy) {
        match strategy {
            VisionStrategy::DomInspection => {
                // Claude 3.5 Sonnet is excellent for HTML analysis
                self.vision_model = "anthropic/claude-3-5-sonnet-20241022".to_string();
            }
            VisionStrategy::CoordinateBased => {
                // GPT-4o is superior for visual coordinate detection
                self.vision_model = "openai/gpt-4o-2024-11-20".to_string();
            }
            VisionStrategy::Adaptive => {
                // Claude 3.5 Sonnet is best for adaptive (DOM first, then coordinates)
                self.vision_model = "anthropic/claude-3-5-sonnet-20241022".to_string();
            }
        }
    }

    // Enhanced model configuration with latest OpenRouter models
    pub fn set_vision_model(&mut self, model: &str) {
        self.vision_model = match model.to_lowercase().as_str() {
            // Claude models (best for HTML analysis and general vision)
            "claude" | "claude-3.5" | "claude-sonnet" => "anthropic/claude-3-5-sonnet-20241022",
            "claude-haiku" => "anthropic/claude-3-5-haiku-20241022",

            // OpenAI models (excellent for coordinate detection)
            "gpt-4o" | "gpt4o" => "openai/gpt-4o-2024-11-20",
            "gpt-4o-mini" | "gpt4o-mini" => "openai/gpt-4o-mini-2024-07-18",
            "gpt-4" => "openai/gpt-4-turbo-2024-04-09",

            // Google models (good alternative)
            "gemini" | "gemini-pro" => "google/gemini-pro-1.5",
            "gemini-flash" => "google/gemini-flash-1.5",

            // Use exact model string if provided
            _ => model,
        }
        .to_string();
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

        // Anti-detection measures for better compatibility with modern websites
        caps.add_arg("--disable-blink-features=AutomationControlled")?;
        caps.add_arg("--disable-extensions")?;
        caps.add_arg("--disable-plugins")?;
        caps.add_arg("--disable-web-security")?;
        caps.add_arg("--disable-features=VizDisplayCompositor")?;
        caps.add_arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36")?;

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
                            .attr("value")
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
            &format!(
                "🧠 Using AI Vision with {:?} strategy",
                self.vision_strategy
            ),
            Some("vision"),
        )
        .await;

        // Take screenshot for AI analysis
        let screenshot_data = driver.screenshot_as_png().await?;
        let screenshot_base64 = general_purpose::STANDARD.encode(&screenshot_data);

        // Save debug screenshot
        let debug_timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let debug_filename = format!("debug_vision_{}.png", debug_timestamp);
        std::fs::write(&debug_filename, &screenshot_data)?;

        self.log(
            "INFO",
            &format!("📸 Debug screenshot saved: {}", debug_filename),
            Some("vision"),
        )
        .await;

        // Execute action based on vision strategy
        match &self.vision_strategy {
            VisionStrategy::DomInspection => {
                self.execute_vision_action_dom_strategy(action, &screenshot_base64)
                    .await
            }
            VisionStrategy::CoordinateBased => {
                self.execute_vision_action_with_coordinates(action).await
            }
            VisionStrategy::Adaptive => {
                // Try DOM inspection first, fall back to coordinates
                match self
                    .execute_vision_action_dom_strategy(action, &screenshot_base64)
                    .await
                {
                    Ok(result) => {
                        self.log(
                            "SUCCESS",
                            "✅ DOM inspection strategy succeeded",
                            Some("vision"),
                        )
                        .await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.log(
                            "WARN",
                            &format!("⚠️ DOM inspection failed: {}, trying coordinates", e),
                            Some("vision"),
                        )
                        .await;
                        self.execute_vision_action_with_coordinates(action).await
                    }
                }
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

        self.log(
            "INFO",
            &format!(
                "🔍 Waiting for element '{}' to become visible ({}s timeout)",
                selector, timeout_seconds
            ),
            Some("wait"),
        )
        .await;

        while start_time.elapsed() < timeout {
            // Check for common blocking elements first (Google consent, cookies, etc.)
            self.handle_common_blocking_elements(driver).await?;

            if let Ok(element) = self.try_find_element(driver, selector).await {
                if element.is_displayed().await.unwrap_or(false) {
                    self.log(
                        "SUCCESS",
                        &format!("✅ Element '{}' is now visible", selector),
                        Some("wait"),
                    )
                    .await;
                    return Ok(());
                }
            }

            // Log progress every few seconds
            let elapsed = start_time.elapsed().as_secs();
            if elapsed % 3 == 0 && elapsed > 0 {
                self.log(
                    "INFO",
                    &format!("⏳ Still waiting for '{}' ({}s elapsed)", selector, elapsed),
                    Some("wait"),
                )
                .await;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Before failing, try alternative selectors for Google search
        if selector.contains("input[name=\"q\"]") {
            self.log(
                "WARN",
                "🔄 Trying alternative Google search selectors",
                Some("wait"),
            )
            .await;

            let alternative_selectors = vec![
                "input[title=\"Search\"]",
                "input[aria-label*=\"Search\"]",
                "input[placeholder*=\"Search\"]",
                "textarea[name=\"q\"]",
                "#searchbox input",
                ".gLFyf", // Google's search input class
                "input[type=\"text\"]",
            ];

            for alt_selector in alternative_selectors {
                if let Ok(element) = self.try_find_element(driver, alt_selector).await {
                    if element.is_displayed().await.unwrap_or(false) {
                        self.log(
                            "SUCCESS",
                            &format!("✅ Found alternative selector: '{}'", alt_selector),
                            Some("wait"),
                        )
                        .await;
                        return Ok(());
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Element '{}' did not become visible within {} seconds",
            selector,
            timeout_seconds
        ))
    }

    // Handle common blocking elements that prevent main content from being visible
    async fn handle_common_blocking_elements(&self, driver: &WebDriver) -> Result<()> {
        // List of common blocking elements to dismiss
        let blocking_selectors = vec![
            // Google cookie consent
            "button[id*='accept']",
            "button[aria-label*='Accept']",
            "button:contains('Accept all')",
            "button:contains('I agree')",
            "#L2AGLb", // Google's "Accept all" button
            // Generic cookie banners
            "button[id*='cookie']",
            "button[class*='cookie']",
            "*[id*='consent'] button",
            // Modal dialogs
            "button[aria-label*='Close']",
            "button.close",
            ".modal button",
            // Overlay dismissal
            "[role='dialog'] button",
            ".overlay button",
        ];

        for selector in blocking_selectors {
            if let Ok(element) = self.try_find_element(driver, selector).await {
                if element.is_displayed().await.unwrap_or(false) {
                    self.log(
                        "INFO",
                        &format!("🔲 Dismissing blocking element: {}", selector),
                        Some("blocking"),
                    )
                    .await;

                    let _ = element.click().await; // Don't fail if click doesn't work
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    break; // Only dismiss one at a time
                }
            }
        }

        Ok(())
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

                // Special handling for Google search which may not change URL
                if assertion_value == "search" && current_url.contains("google.com") {
                    self.log(
                        "INFO", 
                        "🔍 Detected Google search verification, checking for search results instead of URL change",
                        Some("assertion"),
                    )
                    .await;

                    // Check for search results elements instead of URL
                    let page_source = driver.source().await?;
                    let has_search_results = page_source.contains("search-results") 
                        || page_source.contains("result-stats")
                        || page_source.contains("search_results")
                        || page_source.contains("g-blk")
                        || page_source.contains("srg")
                        || page_source.contains("ULSxyf")  // Google results container
                        || page_source.contains("VjDLd"); // Google results area

                    if has_search_results {
                        self.log(
                            "INFO",
                            "✅ Google search results detected on page",
                            Some("assertion"),
                        )
                        .await;
                        Ok(())
                    } else {
                        // Wait a bit more for results to load
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        let page_source = driver.source().await?;
                        let has_search_results = page_source.contains("search-results")
                            || page_source.contains("result-stats")
                            || page_source.contains("search_results")
                            || page_source.contains("g-blk")
                            || page_source.contains("srg")
                            || page_source.contains("ULSxyf")
                            || page_source.contains("VjDLd");

                        if has_search_results {
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!(
                                "Google search results not found. URL: '{}'. Page may still be loading or search failed.",
                                current_url
                            ))
                        }
                    }
                } else if current_url.contains(assertion_value) {
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
            "search_results_visible" => {
                self.log(
                    "INFO",
                    "🔍 Checking for search results visibility",
                    Some("assertion"),
                )
                .await;

                let page_source = driver.source().await?;
                let current_url = driver.current_url().await?.to_string();

                let has_search_results = if current_url.contains("google.com") {
                    // Google-specific search result indicators
                    page_source.contains("search-results") 
                        || page_source.contains("result-stats")
                        || page_source.contains("search_results")
                        || page_source.contains("g-blk")
                        || page_source.contains("srg")
                        || page_source.contains("ULSxyf")  // Google results container
                        || page_source.contains("VjDLd")   // Google results area
                        || page_source.contains("tF2Cxc") // Individual result containers
                } else {
                    // Generic search result indicators
                    page_source.contains("search-result")
                        || page_source.contains("result")
                        || page_source.contains("search_result")
                        || page_source.contains("results")
                };

                if has_search_results {
                    self.log(
                        "SUCCESS",
                        "✅ Search results are visible on the page",
                        Some("assertion"),
                    )
                    .await;
                    Ok(())
                } else {
                    // Wait a bit for dynamic content to load
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    let page_source = driver.source().await?;

                    let has_search_results = if current_url.contains("google.com") {
                        page_source.contains("search-results")
                            || page_source.contains("result-stats")
                            || page_source.contains("search_results")
                            || page_source.contains("g-blk")
                            || page_source.contains("srg")
                            || page_source.contains("ULSxyf")
                            || page_source.contains("VjDLd")
                            || page_source.contains("tF2Cxc")
                    } else {
                        page_source.contains("search-result")
                            || page_source.contains("result")
                            || page_source.contains("search_result")
                            || page_source.contains("results")
                    };

                    if has_search_results {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!(
                            "Search results are not visible. Current URL: {}",
                            current_url
                        ))
                    }
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
        _screenshot_base64: &str,
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

        // Enhanced HTML analysis prompt based on model type
        let (system_prompt, user_prompt) = if self.vision_model.contains("claude") {
            // Claude-optimized prompts for HTML analysis
            (
                Some("You are an expert web developer specializing in CSS selector analysis. Your task is to analyze HTML and return the most reliable CSS selector for the described element. Always prioritize specificity and reliability."),
                format!(
                    "Analyze this HTML and find the best CSS selector for: '{}'\n\n\
                    Requirements:\n\
                    - Return ONLY the CSS selector (no explanations)\n\
                    - Prefer ID selectors when available (#id)\n\
                    - Use class selectors for reliability (.class)\n\
                    - Avoid overly complex selectors\n\
                    - Ensure selector is unique and stable\n\n\
                    HTML source:\n{}", 
                    element_description, truncated_html
                )
            )
        } else if self.vision_model.contains("gpt") {
            // GPT-optimized prompts
            (
                Some("Expert CSS selector analyzer. Return only the most reliable CSS selector for the described element."),
                format!(
                    "Find CSS selector for: '{}'\n\
                    Rules: Return only CSS selector, no text. Prefer #id, then .class, then tag[attr].\n\n\
                    HTML:\n{}", 
                    element_description, truncated_html
                )
            )
        } else {
            // Generic prompts
            (
                None,
                format!(
                    r#"Find CSS selector for '{}' in this HTML. Return only the selector.\n\nHTML:\n{}\n\nSelector:"#,
                    element_description, truncated_html
                ),
            )
        };

        let mut messages = Vec::new();

        if let Some(system) = system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": user_prompt
        }));

        let mut payload = json!({
            "model": vision_model,
            "messages": messages,
            "max_tokens": 150,
            "temperature": 0.1
        });

        // Model-specific optimizations for HTML analysis
        if self.vision_model.contains("claude") {
            payload["max_tokens"] = json!(200);
            payload["temperature"] = json!(0.0);
        } else if self.vision_model.contains("gpt") {
            payload["max_tokens"] = json!(100);
            payload["temperature"] = json!(0.1);
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", vision_api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://ai-browser-automation.local")
            .header("X-Title", "AI Browser Automation HTML Analysis")
            .header("User-Agent", "AI-Browser-Automation/1.0")
            .timeout(std::time::Duration::from_secs(20))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let _error_text = response
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
        } else if desc.contains("search") {
            // Google search specific fallbacks with modern selectors
            if desc.contains("google") {
                "textarea[name=\"q\"]".to_string() // Google's current search input (changed from input to textarea)
            } else {
                "input[name=\"q\"]".to_string()
            }
        } else if desc.contains("input") || element_type == "input" {
            "input[type=\"text\"]".to_string()
        } else if desc.contains("button") || element_type == "button" {
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

    // Computer Vision Implementation
    async fn get_element_coordinates_via_vision(
        &self,
        screenshot_base64: &str,
        prompt: &str,
    ) -> Result<(i32, i32)> {
        if let Some(api_key) = &self.vision_api_key {
            self.log(
                "INFO",
                "🧠 Analyzing screenshot with AI Vision for coordinates",
                Some("vision"),
            )
            .await;

            let coordinates = self
                .call_vision_api_for_coordinates(api_key, screenshot_base64, prompt)
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

    async fn call_vision_api_for_coordinates(
        &self,
        api_key: &str,
        screenshot_base64: &str,
        prompt: &str,
    ) -> Result<(i32, i32)> {
        self.log(
            "INFO",
            &format!(
                "🤖 Calling OpenRouter {} for coordinate analysis",
                self.vision_model
            ),
            Some("vision"),
        )
        .await;

        let client = reqwest::Client::new();

        // Enhanced prompt based on model type
        let enhanced_prompt = if self.vision_model.contains("claude") {
            // Claude-optimized prompt
            format!(
                "{}\n\nYou are an expert at analyzing web page screenshots. Your task:\n\
                1. Locate the described element in the screenshot\n\
                2. Determine the center coordinates where a user should click\n\
                3. Return ONLY the coordinates in format: x,y\n\
                4. Example: 450,320\n\
                5. If not found, respond: NOT_FOUND\n\
                6. Be precise - users rely on accurate coordinates",
                prompt
            )
        } else if self.vision_model.contains("gpt") {
            // GPT-optimized prompt
            format!(
                "{}\n\nTask: Find element coordinates for clicking\n\
                Instructions:\n\
                - Analyze the screenshot carefully\n\
                - Locate the described element\n\
                - Return center click coordinates as: x,y\n\
                - Format example: 450,320\n\
                - If element not visible: NOT_FOUND\n\
                - Be accurate - this controls browser automation",
                prompt
            )
        } else {
            // Generic prompt for other models
            format!(
                "{}\n\nFind the element and return click coordinates as x,y format (example: 450,320). If not found, return NOT_FOUND.",
                prompt
            )
        };

        // Optimized payload based on model
        let mut payload = json!({
            "model": self.vision_model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": enhanced_prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", screenshot_base64)
                        }
                    }
                ]
            }],
            "max_tokens": 100,
            "temperature": 0.1
        });

        // Model-specific optimizations
        if self.vision_model.contains("claude") {
            payload["max_tokens"] = json!(150);
            payload["temperature"] = json!(0.0);
        } else if self.vision_model.contains("gpt-4o") {
            payload["max_tokens"] = json!(50);
            payload["temperature"] = json!(0.1);
        }

        self.log(
            "INFO",
            "📡 Sending screenshot to OpenRouter Vision API...",
            Some("vision"),
        )
        .await;

        let response = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://ai-browser-automation.local")
            .header("X-Title", "AI Browser Automation Vision")
            .header("User-Agent", "AI-Browser-Automation/1.0")
            .timeout(std::time::Duration::from_secs(30))
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

            // Enhanced error reporting for common OpenRouter issues
            let error_msg = if status == 401 {
                "OpenRouter API key invalid or expired. Check your OPENROUTER_API_KEY."
            } else if status == 429 {
                "OpenRouter rate limit exceeded. Please wait and try again."
            } else if status == 402 {
                "OpenRouter account has insufficient credits. Please add credits to your account."
            } else {
                "OpenRouter API error occurred"
            };

            self.log(
                "ERROR",
                &format!("❌ {}: {} - {}", error_msg, status, error_text),
                Some("vision"),
            )
            .await;
            return Err(anyhow::anyhow!("{}: {}", error_msg, error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse OpenRouter response: {}", e))?;

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or("");

        self.log(
            "SUCCESS",
            &format!("🧠 OpenRouter response: '{}'", content.trim()),
            Some("vision"),
        )
        .await;

        self.parse_coordinates_from_response(content).await
    }

    async fn parse_coordinates_from_response(&self, response: &str) -> Result<(i32, i32)> {
        let cleaned = response.trim();

        self.log(
            "INFO",
            &format!("🔍 Parsing coordinates from AI response: '{}'", cleaned),
            Some("vision"),
        )
        .await;

        // Check for NOT_FOUND response
        if cleaned.to_uppercase().contains("NOT_FOUND") {
            return Err(anyhow::anyhow!("AI Vision could not find the element"));
        }

        // Try different coordinate formats
        let patterns = vec![
            r"(\d+),\s*(\d+)",               // Standard x,y format
            r"\((\d+),\s*(\d+)\)",           // Parentheses format
            r"\[(\d+),\s*(\d+)\]",           // Bracket format
            r"x:\s*(\d+).*?y:\s*(\d+)",      // x: y: format
            r"(\d+)\s*,\s*(\d+)",            // Flexible format
            r"click.*?(\d+),\s*(\d+)",       // "click at 450,320"
            r"coordinates.*?(\d+),\s*(\d+)", // "coordinates: 450,320"
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

        // Fallback: extract first two numbers
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

        self.log(
            "ERROR",
            &format!(
                "❌ Could not parse valid coordinates from: '{}'. Coordinate-based vision failed.",
                cleaned
            ),
            Some("vision"),
        )
        .await;

        Err(anyhow::anyhow!(
            "Vision AI failed to provide valid coordinates. Response: '{}'",
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

        // Move to coordinates and click
        driver
            .action_chain()
            .move_to(x as i64, y as i64)
            .click()
            .perform()
            .await?;

        // Brief pause after click
        tokio::time::sleep(Duration::from_millis(1000)).await;

        self.log(
            "SUCCESS",
            &format!("✅ Successfully clicked at ({}, {})", x, y),
            Some("vision"),
        )
        .await;

        Ok(())
    }

    // Enhanced Vision Action with fallback to coordinates
    async fn execute_vision_action_with_coordinates(
        &mut self,
        action: &BrowserAction,
    ) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        // Take screenshot for AI analysis
        let screenshot_data = driver.screenshot_as_png().await?;
        let screenshot_base64 = general_purpose::STANDARD.encode(&screenshot_data);

        // Save debug screenshot
        let debug_timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let debug_filename = format!("debug_vision_{}.png", debug_timestamp);
        std::fs::write(&debug_filename, &screenshot_data)?;

        self.log(
            "INFO",
            &format!("📸 Debug screenshot saved: {}", debug_filename),
            Some("vision"),
        )
        .await;

        match action.action_type.as_str() {
            "click" => {
                let element_description = action
                    .element_description
                    .as_deref()
                    .unwrap_or("clickable element");

                self.log(
                    "INFO",
                    &format!("🎯 Vision click: Finding '{}'", element_description),
                    Some("vision"),
                )
                .await;

                let prompt = format!(
                    "I need to click on '{}'. Please analyze this screenshot and tell me the exact pixel coordinates where I should click.",
                    element_description
                );

                // Try to get coordinates via vision
                match self
                    .get_element_coordinates_via_vision(&screenshot_base64, &prompt)
                    .await
                {
                    Ok((x, y)) => {
                        self.click_at_coordinates(driver, x, y).await?;
                        Ok(format!(
                            "Successfully clicked '{}' at coordinates ({}, {})",
                            element_description, x, y
                        ))
                    }
                    Err(e) => {
                        self.log(
                            "WARN",
                            &format!("⚠️ Coordinate-based vision failed: {}", e),
                            Some("vision"),
                        )
                        .await;

                        // Fallback to AI DOM selector finding
                        self.log(
                            "INFO",
                            "🔄 Falling back to AI DOM selector analysis",
                            Some("vision"),
                        )
                        .await;

                        let selector = self
                            .get_ai_selector_for_element(
                                &screenshot_base64,
                                element_description,
                                "button",
                            )
                            .await?;

                        let dom_action = BrowserAction {
                            action_type: "click".to_string(),
                            selector: Some(selector),
                            element_description: None,
                            text: None,
                            url: None,
                            wait_condition: None,
                            screenshot: false,
                        };

                        self.execute_dom_action(&dom_action).await
                    }
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
                            "⌨️ Vision type: Finding '{}' to type '{}'",
                            element_description, text
                        ),
                        Some("vision"),
                    )
                    .await;

                    let prompt = format!(
                        "I need to type text into '{}'. Please analyze this screenshot and tell me the exact pixel coordinates of the input field where I should click first.",
                        element_description
                    );

                    // Try coordinate-based approach first
                    match self
                        .get_element_coordinates_via_vision(&screenshot_base64, &prompt)
                        .await
                    {
                        Ok((x, y)) => {
                            // Click on the input field first
                            self.click_at_coordinates(driver, x, y).await?;

                            // Wait for focus
                            tokio::time::sleep(Duration::from_millis(500)).await;

                            // Type the text
                            driver.action_chain().send_keys(text).perform().await?;

                            Ok(format!(
                                "Successfully typed '{}' into '{}' at coordinates ({}, {})",
                                text, element_description, x, y
                            ))
                        }
                        Err(e) => {
                            self.log(
                                "WARN",
                                &format!("⚠️ Coordinate-based typing failed: {}", e),
                                Some("vision"),
                            )
                            .await;

                            // Fallback to AI DOM selector
                            let selector = self
                                .get_ai_selector_for_element(
                                    &screenshot_base64,
                                    element_description,
                                    "input",
                                )
                                .await?;

                            let dom_action = BrowserAction {
                                action_type: "type".to_string(),
                                selector: Some(selector),
                                element_description: None,
                                text: Some(text.clone()),
                                url: None,
                                wait_condition: None,
                                screenshot: false,
                            };

                            self.execute_dom_action(&dom_action).await
                        }
                    }
                } else {
                    Err(anyhow::anyhow!("Type action requires text"))
                }
            }

            "navigate" => {
                // Navigation doesn't need vision
                self.execute_dom_action(action).await
            }

            "wait" => {
                // Wait actions don't need vision
                self.execute_dom_action(action).await
            }

            "screenshot" => {
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("vision_screenshot_{}.png", timestamp);
                std::fs::write(&filename, screenshot_data)?;
                Ok(format!("Vision screenshot saved as {}", filename))
            }

            _ => Err(anyhow::anyhow!(
                "Unsupported action type for Vision mode: {}",
                action.action_type
            )),
        }
    }

    // DOM inspection strategy - AI analyzes HTML to find CSS selectors
    async fn execute_vision_action_dom_strategy(
        &mut self,
        action: &BrowserAction,
        screenshot_base64: &str,
    ) -> Result<String> {
        let driver = self
            .driver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebDriver not initialized"))?;

        self.log(
            "INFO",
            "🧠 Using AI DOM Inspection to find precise selectors",
            Some("dom_strategy"),
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
                        Some("dom_strategy"),
                    )
                    .await;

                    let selector = self
                        .get_ai_selector_for_element(
                            screenshot_base64,
                            element_description,
                            "input",
                        )
                        .await?;

                    self.log(
                        "INFO",
                        &format!("🎯 AI found selector: {}", selector),
                        Some("dom_strategy"),
                    )
                    .await;

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
                                Some("dom_strategy"),
                            )
                            .await;
                            Ok(result)
                        }
                        Err(e) => {
                            self.log(
                                "ERROR",
                                &format!("❌ AI DOM typing failed: {}", e),
                                Some("dom_strategy"),
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
                    Some("dom_strategy"),
                )
                .await;

                let selector = self
                    .get_ai_selector_for_element(screenshot_base64, element_description, "button")
                    .await?;

                self.log(
                    "INFO",
                    &format!("🎯 AI found selector: {}", selector),
                    Some("dom_strategy"),
                )
                .await;

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
                        self.log(
                            "SUCCESS",
                            "✅ AI DOM click successful",
                            Some("dom_strategy"),
                        )
                        .await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.log(
                            "ERROR",
                            &format!("❌ AI DOM click failed: {}", e),
                            Some("dom_strategy"),
                        )
                        .await;
                        Err(e)
                    }
                }
            }

            "wait" => {
                self.log(
                    "INFO",
                    "⏱️ DOM strategy: Using standard wait implementation",
                    Some("dom_strategy"),
                )
                .await;
                self.execute_dom_action(action).await
            }

            "screenshot" => {
                self.log(
                    "INFO",
                    "📸 Taking DOM strategy enhanced screenshot",
                    Some("screenshot"),
                )
                .await;
                let screenshot_data = driver.screenshot_as_png().await?;
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let filename = format!("dom_strategy_screenshot_{}.png", timestamp);
                std::fs::write(&filename, screenshot_data)?;
                let result_msg = format!("DOM strategy screenshot saved as {}", filename);
                self.log("SUCCESS", &format!("✅ {}", result_msg), Some("screenshot"))
                    .await;
                Ok(result_msg)
            }

            _ => {
                let error_msg = format!(
                    "Unsupported action type for DOM strategy: {}",
                    action.action_type
                );
                self.log("ERROR", &format!("❌ {}", error_msg), Some("dom_strategy"))
                    .await;
                Err(anyhow::anyhow!(error_msg))
            }
        }
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
