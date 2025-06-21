# 🎯 AC to Automation Converter

AI-powered system that converts **Acceptance Criteria (AC)** from QA specifications into automated browser testing workflows using the Model Context Protocol (MCP) and Browser MCP integration.

## 🚀 Overview

This system automatically transforms your Gherkin-style or plain text acceptance criteria into executable browser automation workflows. It leverages:

- **🦀 Rust MCP SDK** - For robust MCP server/client implementation
- **🌐 Browser MCP** - For actual browser automation execution
- **📋 Gherkin Parsing** - Native support for BDD-style acceptance criteria
- **🤖 Intelligent Conversion** - Smart mapping from natural language to automation actions

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Acceptance    │    │    AC Automation     │    │   Browser MCP       │
│   Criteria      │───▶│    Server (MCP)      │───▶│   Server            │
│   (Gherkin)     │    │                      │    │                     │
└─────────────────┘    └──────────────────────┘    └─────────────────────┘
                                  │
                                  ▼
                       ┌──────────────────────┐
                       │  Integration Layer   │
                       │  (Orchestrator)      │
                       └──────────────────────┘
                                  │
                                  ▼
                       ┌──────────────────────┐
                       │  Execution Reports   │
                       │  & Generated Scripts │
                       └──────────────────────┘
```

### Components

1. **`automation-api`** - MCP server for parsing AC and generating workflows
2. **`automation-browser`** - Browser MCP client for workflow execution
3. **`automation-integration`** - Orchestration layer connecting everything

## 📋 Prerequisites

- **Rust** (latest stable)
- **Node.js** (for Browser MCP server)
- **Browser MCP package**: `@browsermcp/mcp@latest`

## 🛠️ Setup

### 1. Install Dependencies

```bash
# Install Node.js dependencies for Browser MCP
npm install -g @browsermcp/mcp@latest

# Clone and build the project
git clone <repository-url>
cd ai-ac-automation
cargo build --release
```

### 2. Configure Browser MCP (Optional)

Add Browser MCP to your AI application (like Cursor):

```json
{
  "mcpServers": {
    "browsermcp": {
      "command": "npx",
      "args": ["@browsermcp/mcp@latest"]
    }
  }
}
```

## 🎮 Usage

### Quick Start Demo

```bash
# Run the interactive demo
cargo run --bin automation-integration

# Or run individual components
cargo run --bin automation-api  # AC automation MCP server
```

### Example: Converting Login Acceptance Criteria

**Input Acceptance Criteria:**

```gherkin
Scenario: Successful user login
Given I am on the login page
When I enter valid credentials
And I click the login button
Then I should be redirected to the dashboard
And I should see a welcome message
```

**Generated Automation Workflow:**

```json
{
  "id": "workflow_ac_12345",
  "name": "Automation for: User Login",
  "description": "Automated test workflow for Authentication - Successful user login",
  "test_steps": [
    {
      "step_type": "given",
      "description": "I am on the login page",
      "browser_actions": [
        {
          "action_type": "navigate",
          "url": "/login",
          "wait_condition": "page_load"
        }
      ]
    },
    {
      "step_type": "when",
      "description": "I enter valid credentials",
      "browser_actions": [
        {
          "action_type": "type",
          "selector": "input[name*='username']",
          "text": "valid credentials"
        }
      ]
    }
    // ... more steps
  ]
}
```

**Generated MCP Browser Script:**

```javascript
// MCP Browser Automation Script for: User Login
// Generated from: Automated test workflow for Authentication

// GIVEN: I am on the login page
await mcp.browser.navigate("/login");

// WHEN: I enter valid credentials
await mcp.browser.type('input[name*="username"]', "valid credentials");

// WHEN: I click the login button
await mcp.browser.click('button[contains(text(), "login button")]');

// THEN: I should be redirected to the dashboard
await mcp.browser.wait("page_stable");
await mcp.browser.assert("url_contains: dashboard");
```

## 📝 Supported Acceptance Criteria Formats

### 1. Gherkin/BDD Format

```gherkin
Scenario: Add item to cart
Given I am on the product page for "Laptop"
When I click "Add to Cart"
And I select quantity "2"
Then I should see cart icon showing "2 items"
```

### 2. Plain Text Format

```text
User should be able to submit contact form with valid data
Navigate to contact page, fill form fields, submit, verify success message
```

### 3. Mixed Format

```text
Test Case: Form Validation
Given I have invalid email format
When I submit the form
Then validation errors should appear
```

## 🔧 Available Browser Actions

The system intelligently maps natural language to these browser actions:

- **🧭 Navigation**: `navigate`, `go to`, `visit`
- **👆 Interactions**: `click`, `press`, `select`, `choose`
- **⌨️ Input**: `type`, `enter`, `fill`, `input`
- **⏱️ Waiting**: `wait`, `pause`, automatic conditions
- **📸 Verification**: `see`, `display`, `show`, `verify`

## 🎯 Intelligent Parsing Features

- **Smart Selector Generation** - Converts "login button" → `button[contains(text(), 'login')]`
- **URL Recognition** - Extracts URLs or generates paths from context
- **Action Context Awareness** - Different behavior for Given/When/Then steps
- **Error Handling** - Graceful fallbacks for ambiguous instructions
- **Assertion Generation** - Automatic test assertions from Then statements

## 📊 Execution Reports

```text
📋 === AC to Automation Pipeline Summary ===
📝 Acceptance Criteria: User Login
🎯 Feature: Authentication
📊 Scenario: Successful user login
📦 Given Steps: 1
⚡ When Steps: 2
✅ Then Steps: 2
🏷️  Tags: ["login", "authentication", "smoke"]

🤖 Automation Workflow: Automation for: User Login
📝 Description: Automated test workflow for Authentication - Successful user login
🔢 Total Test Steps: 5

📊 === Execution Report ===
🎯 Workflow: Automation for: User Login
📈 Success Rate: 80.0%
✅ Successful Steps: 4
❌ Failed Steps: 1
⏱️  Duration: 12.45s
```

## 🔌 MCP Integration

### AC Automation Server Tools

- `parse_acceptance_criteria` - Parse Gherkin or plain text AC
- `convert_to_automation` - Generate automation workflow
- `get_workflows` - List all generated workflows
- `execute_workflow` - Run workflow via Browser MCP
- `generate_browser_script` - Export as various script formats

### Browser MCP Client Actions

- `navigate(url)` - Navigate to page
- `click(selector)` - Click element
- `type_text(selector, text)` - Input text
- `wait_for_element(selector)` - Wait for element
- `take_screenshot()` - Capture screenshot
- `is_element_visible(selector)` - Check visibility

## 🎨 Example Use Cases

### 1. **QA Test Automation**

Convert manual test cases to automated browser tests

### 2. **BDD to Automation**

Transform Gherkin specifications into executable tests

### 3. **Regression Testing**

Generate comprehensive test suites from acceptance criteria

### 4. **Cross-browser Testing**

Create consistent test workflows across different browsers

### 5. **CI/CD Integration**

Automated test generation and execution in pipelines

## 🔍 Advanced Features

### Custom Script Generation

```rust
// Generate different script formats
let selenium_script = integration.generate_browser_script(&workflow_id, "selenium").await?;
let playwright_script = integration.generate_browser_script(&workflow_id, "playwright").await?;
let mcp_script = integration.generate_browser_script(&workflow_id, "mcp_browser").await?;
```

### Workflow Management

```rust
// Get all workflows
let workflows = integration.get_all_workflows().await?;

// Get specific workflow
let workflow = integration.get_workflow("workflow_ac_12345").await?;

// Execute specific workflow
let report = integration.execute_automation_workflow(&workflow).await?;
```

## 🚨 Troubleshooting

### Common Issues

1. **Browser MCP Connection Failed**

   ```bash
   # Ensure Node.js and Browser MCP are installed
   npm install -g @browsermcp/mcp@latest
   npx @browsermcp/mcp@latest --version
   ```

2. **MCP Server Not Starting**

   ```bash
   # Check if automation-api builds successfully
   cargo build --bin automation-api
   cargo run --bin automation-api
   ```

3. **Workflow Execution Fails**
   - Verify selectors are valid for target website
   - Check if target website is accessible
   - Ensure proper wait conditions

### Debug Mode

```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin automation-integration
```

## 🤝 Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -am 'Add amazing feature'`)
4. Push branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **[Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)** - Official Rust implementation
- **[Browser MCP](https://docs.browsermcp.io/)** - Browser automation via MCP
- **Model Context Protocol** - Standardized AI-tool communication

---

**🎯 Transform your acceptance criteria into automated tests with the power of MCP!** 🚀
