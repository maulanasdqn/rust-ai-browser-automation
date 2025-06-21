use automation_ui::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    if let Err(e) = dotenvy::dotenv() {
        println!("⚠️ Warning: Could not load .env file: {}", e);
        println!("💡 Create a .env file with OPENROUTER_API_KEY for best experience");
    } else {
        println!("📄 Loaded environment variables from .env file");
    }

    println!("🚀 Starting AC Automation Web UI...");
    println!("🎯 This provides a web interface for converting Acceptance Criteria to automation");

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);

    start_server(port).await?;

    Ok(())
}
