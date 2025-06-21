use automation_ui::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting AC Automation Web UI...");
    println!("🎯 This provides a web interface for converting Acceptance Criteria to automation");

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    start_server(port).await?;

    Ok(())
}
