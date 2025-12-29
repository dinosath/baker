//! Example demonstrating Baker-generated code at compile time.
//!
//! Run with: `cargo run`

// Include the generated code from build.rs
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn main() {
    println!("=== Baker build.rs Example ===\n");

    // Use generated constants
    println!("Using generated HTTP status codes:\n");

    println!("OK status: {}", http_status::OK);
    println!("  - Code: {}", http_status::OK.code());
    println!("  - Name: {}", http_status::OK.name());
    println!("  - Description: {}", http_status::OK.description());

    println!();

    // Look up by code
    if let Some(status) = http_status::from_code(404) {
        println!("Status 404: {} - {}", status.name(), status.description());
    }

    // Look up by name
    if let Some(status) = http_status::from_name("INTERNAL_SERVER_ERROR") {
        println!("INTERNAL_SERVER_ERROR: code {}", status.code());
    }

    println!();

    // Iterate all status codes
    println!("All {} status codes:", http_status::ALL.len());
    for status in http_status::ALL {
        println!("  {} = {}", status.name(), status.code());
    }

    println!("\n=== Done ===");
}
