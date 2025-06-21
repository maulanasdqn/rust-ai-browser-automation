# 🤖 AI-Powered Browser Automation with Vision

Convert natural language into **real browser automation** using AI Vision and execute tests immediately with **live process logs**!

Inspired by [Skyvern](https://github.com/Skyvern-AI/skyvern), this system combines traditional DOM automation with **AI Vision** that "sees" web pages like a human.

## ✨ Key Features

### 👁️ **AI Vision Mode**

- **Visual Understanding**: AI analyzes screenshots to find elements visually
- **Robust Automation**: Works even when websites change their HTML structure
- **Natural Descriptions**: Use "click the blue login button" instead of CSS selectors
- **Human-like Interaction**: Sees pages exactly like humans do

### 🔧 **Three Automation Modes**

- **DOM Mode**: Traditional CSS selector-based (fast)
- **Vision Mode**: AI visual understanding (robust)
- **Hybrid Mode**: Smart fallback - tries DOM first, uses Vision if needed

### 📝 **Real-Time Process Logs**

- **Live Execution Logs**: Watch automation steps in real-time
- **Floating Log Panel**: See progress without scrolling
- **Color-coded Messages**: Easy to spot successes, warnings, and errors
- **Detailed Timestamps**: Track execution timing precisely

### 🚀 **Immediate Execution**

- **Real Browser Testing**: Uses ChromeDriver for actual browser interaction
- **AI-Powered Generation**: OpenRouter AI converts natural language to automation
- **Multiple Script Formats**: Generate MCP Browser, Selenium, and Playwright scripts
- **Visual Feedback**: Screenshots and detailed execution reports

## 🎯 What You Can Automate

### 🔄 **User Workflows**

- Login/registration flows
- E-commerce checkout processes
- Form submissions and validations
- Multi-step wizards

### 🎨 **Visual Interactions**

- Click buttons by description ("red submit button")
- Find inputs by visual context ("email field in top-right")
- Navigate by visual landmarks ("menu button with hamburger icon")
- Verify visual states ("success message appears")

### 📊 **Content Testing**

- Text presence verification
- Element visibility checks
- Page state validation
- Dynamic content testing

## 🛠️ Setup & Installation

### 📋 Prerequisites

1. **Install Rust**:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Install ChromeDriver**:

```bash
# macOS with Homebrew
brew install chromedriver

# Ubuntu/Debian
sudo apt-get install chromium-chromedriver

# Windows: Download from https://chromedriver.chromium.org/
```

3. **Get OpenRouter API Key** (Recommended):
   - Sign up at [openrouter.ai](https://openrouter.ai)
   - Get your API key (starts with `sk-or-v1-...`)
   - **🎉 One key works for both AI generation AND vision models!**

### 🚀 Quick Start

1. **Clone and Build**:

```bash
git clone <your-repo>
cd ai-ac-automation
cargo build --release
```

2. **Configure Environment Variables** (Recommended):

```bash
# Create .env file with your OpenRouter API key
echo "OPENROUTER_API_KEY=sk-or-v1-your-actual-key-here" > .env
```

3. **Start ChromeDriver** (in separate terminal):

```bash
chromedriver --port=9515
```

4. **Start the Web Interface**:

```bash
cargo run --bin automation-ui
```

5. **Open Browser**: Go to `http://localhost:3001`

### 🔐 **API Key Configuration**

You have **two options** for configuring your OpenRouter API key:

#### **Option 1: Environment Variables (Recommended)**

```bash
# Create .env file in project root
echo "OPENROUTER_API_KEY=sk-or-v1-your-key-here" > .env

# Start the server - API key loaded automatically!
cargo run --bin automation-ui
```

**✅ Benefits:**

- **Secure**: API key never appears in UI or logs
- **Convenient**: No need to enter key every time
- **Universal**: Works for both AI generation and vision
- **Safe**: `.env` is in `.gitignore` - won't be committed

#### **Option 2: Web Form**

- Leave `.env` empty or don't create it
- Enter API key directly in the web interface forms
- Works for individual sessions

**💡 Pro Tip:** Use Option 1 for development, Option 2 for sharing/demos!

### 📄 **Environment File (.env) Format**

Your `.env` file should contain:

```bash
# Required: OpenRouter API key for all AI features
OPENROUTER_API_KEY=sk-or-v1-your-actual-key-here

# Optional: Default models (can be changed in UI)
AI_MODEL=anthropic/claude-3.5-sonnet
VISION_MODEL=openai/gpt-4o

# Optional: Browser settings
HEADLESS=false
BROWSER_WIDTH=1920
BROWSER_HEIGHT=1080

# Optional: Server port
PORT=3001
```

**🔒 Security Notes:**

- Never commit `.env` to version control
- Keep your API keys secure and rotate them regularly
- Use different keys for development and production

## 🎯 How to Use

### 🔧 **Basic DOM Automation**

1. **Enter URL**: `https://google.com`
2. **Test Scenario**:

```
- Click on search box
- Type "browser automation"
- Press Enter
- Verify results appear
- Click on first result
```

3. **Execute**: Check "Execute immediately" → Click "Generate & Execute"

### 👁️ **AI Vision Mode** (Recommended)

1. **Enable Vision**: ✅ Check "Use AI Vision Mode (Like Skyvern)"
2. **Configure**:

   - **Mode**: Hybrid (tries DOM first, falls back to Vision)
   - **API Key**: Auto-loaded from `.env` or enter manually
   - **Model**: GPT-4 Omni (recommended)

3. **Natural Test Scenario**:

```
Website: https://example.com/login
Test:
- Find the email input field
- Type admin@test.com
- Find the password field
- Type mypassword123
- Click the blue login button
- Verify the dashboard appears
```

### 🤖 **AI-Powered Test Generation**

1. **Enable AI**: ✅ Check "Use AI-Powered Automation"
2. **API Key**: Auto-loaded from `.env` or enter your [OpenRouter key](https://openrouter.ai)
3. **Describe Naturally**:

```
Test the login functionality:
- User should be able to log in with valid credentials
- After login, dashboard should be visible
- User profile should show correct information
- Logout should work properly
```

**🎯 Example with Environment Variables:**

If you have `OPENROUTER_API_KEY` in your `.env` file:

- ✅ **No API key entry needed** - works automatically!
- ✅ **Same key** works for both AI generation and vision
- ✅ **Secure** - never appears in forms or logs
- ✅ **Fast** - instant access to all AI features

## 🧠 Supported AI Models

**🎉 All models available through [OpenRouter](https://openrouter.ai) with a single API key!**

### 👁️ **Vision Models** (Real AI Vision Integration)

| OpenRouter Model ID           | Provider      | Vision Quality | Speed      | Best For                   |
| ----------------------------- | ------------- | -------------- | ---------- | -------------------------- |
| `openai/gpt-4o`               | **OpenAI**    | ⭐⭐⭐⭐⭐     | ⭐⭐⭐⭐   | **Overall best choice** 🌟 |
| `openai/gpt-4-vision-preview` | **OpenAI**    | ⭐⭐⭐⭐       | ⭐⭐⭐     | Detailed analysis          |
| `anthropic/claude-3.5-sonnet` | **Anthropic** | ⭐⭐⭐⭐⭐     | ⭐⭐⭐⭐   | Complex reasoning          |
| `google/gemini-2.0-flash-001` | **Google**    | ⭐⭐⭐⭐       | ⭐⭐⭐⭐⭐ | **Fastest option** 🚀      |
| `google/gemini-pro-vision`    | **Google**    | ⭐⭐⭐         | ⭐⭐⭐⭐   | Cost-effective             |

### 🧠 **Text Generation Models** (AI Test Creation)

| OpenRouter Model ID           | Provider      | Quality    | Speed      | Best For                 |
| ----------------------------- | ------------- | ---------- | ---------- | ------------------------ |
| `anthropic/claude-3.5-sonnet` | **Anthropic** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐   | **Best reasoning** 🧠    |
| `openai/gpt-4o`               | **OpenAI**    | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐   | Complex automation       |
| `openai/gpt-3.5-turbo`        | **OpenAI**    | ⭐⭐⭐⭐   | ⭐⭐⭐⭐⭐ | **Fast & affordable** 💰 |
| `google/gemini-pro`           | **Google**    | ⭐⭐⭐⭐   | ⭐⭐⭐⭐   | Good alternative         |

**🔑 Single API Key Benefits:**

- **One account** for all AI providers
- **Unified billing** and usage tracking
- **Rate limiting** across all models
- **Easy model switching** in the UI
- **No separate API keys** to manage

## 📊 Real-Time Execution Logs

### 🎨 **Live Log Display**

```
[12:09:15.234] INFO: 🚀 Initializing Chrome WebDriver...
[12:09:15.456] SUCCESS: ✅ Chrome WebDriver initialized successfully
[12:09:15.567] INFO: 🔧 Running in HYBRID mode (DOM + Vision)
[12:09:15.678] INFO: 🌐 Navigating to: https://example.com
[12:09:17.123] SUCCESS: ✅ Navigated to https://example.com
[12:09:17.234] INFO: 👁️ AI Vision: Looking for 'email input field' to click
[12:09:17.456] INFO: 🧠 Analyzing screenshot with AI Vision
[12:09:18.789] SUCCESS: ✅ AI Vision found coordinates: (450, 320)
[12:09:18.890] INFO: 🖱️ Clicking at coordinates (450, 320)
[12:09:19.123] SUCCESS: ✅ Vision-clicked at coordinates (450, 320)
[12:09:19.234] INFO: ⌨️ Vision-typing 'admin@test.com' at coordinates (450, 320)
[12:09:19.567] SUCCESS: ✅ Vision-typed 'admin@test.com' at coordinates (450, 320)
```

### 🎨 **Color-Coded Messages**

- 🟢 **SUCCESS**: Operations completed successfully
- 🔵 **INFO**: General information and progress
- 🟡 **WARN**: Warnings and fallback actions
- 🔴 **ERROR**: Failures and issues

## 🔧 Advanced Configuration

### 🖥️ **Programmatic Usage**

```rust
use automation_browser::{AutomationExecutor, AutomationMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create executor with Vision Mode
    let mut executor = AutomationExecutor::new()?
        .with_vision_mode("sk-your-openai-key".to_string(), Some("gpt-4o".to_string()))
        .with_headless(false);

    // Execute workflow
    let (report, logs) = executor.execute_workflow(&workflow).await?;

    println!("Success rate: {:.1}%", report.success_rate() * 100.0);
    println!("Logs captured: {}", logs.len());

    Ok(())
}
```

### ⚙️ **Automation Modes Comparison**

| Feature              | DOM Mode   | Vision Mode | Hybrid Mode |
| -------------------- | ---------- | ----------- | ----------- |
| **Speed**            | ⭐⭐⭐⭐⭐ | ⭐⭐⭐      | ⭐⭐⭐⭐    |
| **Reliability**      | ⭐⭐⭐     | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐⭐  |
| **Setup Complexity** | ⭐⭐       | ⭐⭐⭐⭐    | ⭐⭐⭐      |
| **Website Changes**  | ⭐         | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐⭐  |
| **Natural Language** | ⭐         | ⭐⭐⭐⭐⭐  | ⭐⭐⭐⭐    |

## 🎨 Web Interface Features

### 📋 **Smart Forms**

- **Live Examples**: Click examples to auto-fill forms
- **Vision Configuration**: Easy setup for AI Vision mode
- **Real-time Validation**: Immediate feedback on inputs
- **Progress Tracking**: Live execution status

### 📊 **Enhanced Results**

- **Execution Statistics**: Success rates, timing, step counts
- **Visual Logs**: Floating panels and detailed terminals
- **Screenshot Gallery**: Automatic screenshots during execution
- **Script Export**: Download generated automation scripts

### 🔧 **Debug Features**

- **Step-by-step Breakdown**: See each action executed
- **Error Highlighting**: Clear error messages and solutions
- **Retry Logic**: Automatic retries with exponential backoff
- **Fallback Options**: Hybrid mode switches strategies automatically

## 🛡️ Security & Best Practices

### 🔐 **API Key Security**

- Store API keys securely (never commit to version control)
- Use environment variables for production
- Rotate keys regularly
- Monitor API usage and costs

### 🧪 **Testing Environment**

- Use test accounts and sandbox environments
- Avoid testing on production systems
- Set up dedicated test data
- Use headless mode for CI/CD

### 🌐 **Website Considerations**

- Respect robots.txt and website terms
- Add delays between actions to avoid rate limiting
- Handle dynamic content and loading states
- Consider website anti-automation measures

## 🆘 Troubleshooting

### 🔧 **ChromeDriver Issues**

```bash
# Check ChromeDriver status
curl http://localhost:9515/status

# Restart ChromeDriver
pkill chromedriver
chromedriver --port=9515
```

### 🔐 **Environment Variable Issues**

```bash
# Check if .env file exists and has correct format
cat .env

# Verify environment variable is loaded
echo $OPENROUTER_API_KEY

# Check server status for API key
curl http://localhost:3001/api/env-status
```

### 👁️ **Vision Mode Issues**

- **API Key**: Verify OpenRouter key is valid (`sk-or-v1-...`)
- **Environment**: Check `.env` file or form input
- **Model Access**: Ensure you have access to vision models on OpenRouter
- **Rate Limits**: Check API usage quotas on [OpenRouter dashboard](https://openrouter.ai)
- **Fallback**: Use Hybrid mode for automatic DOM fallback

### 🚫 **Common Automation Issues**

- **Element Not Found**: Try Vision mode for robust element detection
- **Timing Issues**: Add waits for dynamic content
- **Website Changes**: Vision mode adapts automatically
- **Anti-bot Detection**: Use realistic delays and human-like patterns

## 🏗️ Architecture

### 🧱 **System Components**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web UI        │    │   OpenRouter     │    │  ChromeDriver   │
│  (Axum/HTML)    │◄──►│ (Unified AI API) │    │   (Browser)     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Automation API  │    │  Vision Engine   │    │ Browser Actions │
│   (Workflow)    │◄──►│  (Screenshots)   │◄──►│ (Click/Type)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

**🔧 Integration Benefits:**

- **Single API endpoint** for all AI models
- **Environment variable** configuration (`.env`)
- **Automatic failover** between providers
- **Cost optimization** through unified billing

### 📦 **Crate Structure**

- **automation-ui**: Web interface and server
- **automation-browser**: Chrome automation with Vision support
- **automation-api**: Core workflow and data structures
- **automation-integration**: Pipeline orchestration
- **automation-ai**: AI model integration

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Add tests** for your changes
4. **Update documentation** as needed
5. **Submit** a pull request

### 🎯 **Contribution Areas**

- **Vision Model Support**: Add new AI vision providers
- **Browser Support**: Firefox, Safari automation
- **UI Enhancements**: Better visual design
- **Performance**: Optimization and caching
- **Testing**: More comprehensive test coverage

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **[Skyvern](https://github.com/Skyvern-AI/skyvern)**: Inspiration for AI Vision automation
- **OpenAI**: GPT-4 Vision capabilities
- **Anthropic**: Claude vision and reasoning
- **Selenium**: Browser automation foundation
- **Rust Community**: Amazing ecosystem and support

---

<div align="center">

**🚀 Built with ❤️ using Rust, AI Vision, and Real Browser Automation**

[🌟 Star this repo](https://github.com/your-repo) • [🐛 Report Issues](https://github.com/your-repo/issues) • [💡 Request Features](https://github.com/your-repo/discussions)

</div>
