# 🎯 AC to Automation Converter - Web UI

Transform your Acceptance Criteria into automated browser tests with a beautiful web interface.

## 🚀 Quick Start

1. **Start the server:**

   ```bash
   cargo run --bin automation-ui
   ```

2. **Open your browser:**

   ```
   http://localhost:3001
   ```

3. **Convert your AC:**
   - Select a sample or enter your own acceptance criteria
   - Choose format: Gherkin (BDD) or plain text
   - Click "Convert & Generate"
   - Download generated scripts or execute simulation

## 📋 Supported Formats

### Gherkin/BDD Format

```gherkin
Scenario: User login
Given I am on the login page
When I enter valid credentials
And I click the login button
Then I should be redirected to dashboard
```

### Plain Text Format

```
User should be able to submit contact form
Navigate to contact page, fill form, submit, verify success
```

## 🔧 Generated Outputs

- **MCP Browser**: JavaScript automation for Browser MCP
- **Selenium**: Python automation scripts
- **Playwright**: JavaScript automation scripts
- **Execution Reports**: Success rates, timing, step details

## 🎨 UI Features

- **Modern Interface**: Responsive design with tabbed navigation
- **Sample Data**: Pre-loaded examples for quick testing
- **Real-time Processing**: Immediate feedback and results
- **Script Downloads**: One-click export in multiple formats
- **Workflow Management**: Save and execute automation workflows

## 🏗️ Technical Stack

- **Backend**: Rust + Axum web framework
- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Processing**: Intelligent AC parsing and action mapping
- **Simulation**: Realistic browser automation simulation

## 🎯 Use Cases

- **QA Engineers**: Convert test cases to automation
- **Developers**: Generate browser tests from requirements
- **Test Automation**: Create scripts from acceptance criteria
- **BDD Teams**: Transform Gherkin scenarios to executable tests

## 📖 API Endpoints

- `GET /` - Web interface
- `POST /api/process` - Convert acceptance criteria
- `GET /api/workflows` - List all workflows
- `POST /api/execute/:id` - Execute workflow
- `GET /api/script/:id` - Download script

---

**Built with ❤️ using Rust, Axum, and modern web technologies**
