use std::error::Error;
use thirtyfour::prelude::*;

pub async fn get_website_title() -> Result<String, Box<dyn Error>> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;
    driver.goto("https://www.wikipedia.org/").await?;
    let title = driver.title().await?;
    driver.quit().await?;
    Ok(title)
}
