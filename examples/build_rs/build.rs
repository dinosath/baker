//! Build script that uses Baker to generate Rust code at compile time.
//!
//! This demonstrates how to use Baker as a library in build.rs to generate
//! type-safe Rust code from templates.

use baker::renderer::{new_renderer, TemplateRenderer};
use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if these files change
    println!("cargo::rerun-if-changed=templates/");
    println!("cargo::rerun-if-changed=build.rs");

    // Get the output directory from Cargo
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // Create the template renderer
    let mut engine = new_renderer();

    // Load the template from file
    let template_content = fs::read_to_string("templates/generated.rs.baker.j2")
        .expect("Failed to read template");

    // Add template to the engine for better error messages
    engine
        .add_template("generated.rs", &template_content)
        .expect("Failed to add template");

    // Define the context for template rendering
    // This could come from a YAML/JSON config file, environment variables,
    // or be computed programmatically
    let context = json!({
        "module_name": "http_status",
        "description": "HTTP status codes and their descriptions",
        "items": [
            { "name": "OK", "value": 200, "description": "Request succeeded" },
            { "name": "CREATED", "value": 201, "description": "Resource created" },
            { "name": "ACCEPTED", "value": 202, "description": "Request accepted for processing" },
            { "name": "NO_CONTENT", "value": 204, "description": "No content to return" },
            { "name": "BAD_REQUEST", "value": 400, "description": "Invalid request syntax" },
            { "name": "UNAUTHORIZED", "value": 401, "description": "Authentication required" },
            { "name": "FORBIDDEN", "value": 403, "description": "Access denied" },
            { "name": "NOT_FOUND", "value": 404, "description": "Resource not found" },
            { "name": "INTERNAL_SERVER_ERROR", "value": 500, "description": "Server error" },
            { "name": "BAD_GATEWAY", "value": 502, "description": "Invalid gateway response" },
            { "name": "SERVICE_UNAVAILABLE", "value": 503, "description": "Service temporarily unavailable" },
        ]
    });

    // Render the template
    let generated_code = engine
        .render(&template_content, &context, Some("generated.rs"))
        .expect("Failed to render template");

    // Write the generated code to OUT_DIR
    let generated_file = out_path.join("generated.rs");
    fs::write(&generated_file, generated_code).expect("Failed to write generated file");

    println!("cargo::warning=Generated code written to: {}", generated_file.display());
}
