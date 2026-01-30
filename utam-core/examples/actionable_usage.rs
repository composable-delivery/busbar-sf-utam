//! Example usage of the Actionable trait
//!
//! This example demonstrates how to use the Actionable trait for basic
//! user interactions like focus, blur, scroll, and hover.
//!
//! Note: This example requires a running WebDriver instance.

use utam_core::prelude::*;

#[tokio::main]
async fn main() -> UtamResult<()> {
    // Setup WebDriver (example - requires actual browser)
    // let caps = DesiredCapabilities::chrome();
    // let driver = WebDriver::new("http://localhost:9515", caps).await?;

    // Navigate to a page
    // driver.goto("https://example.com").await?;

    // Find an element
    // let elem = driver.find(By::Id("myInput")).await?;

    // Create an ActionableElement wrapper
    // let actionable = ActionableElement::new(elem);

    // Use Actionable trait methods
    // actionable.focus().await?;  // Set focus on the element
    // actionable.scroll_to_center().await?;  // Scroll element to center of viewport
    // actionable.move_to().await?;  // Hover over the element
    // actionable.blur().await?;  // Remove focus

    println!("Example of Actionable trait usage (code commented out - requires browser)");
    Ok(())
}
