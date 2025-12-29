# Using Baker in build.rs

This example demonstrates how to use Baker as a library in a `build.rs` file to generate Rust code at compile time.

## Use Case

Generate Rust code from templates during the build process. This is useful for:
- Generating boilerplate code from configuration files
- Creating type-safe bindings from schemas
- Building lookup tables or constant definitions
- Generating serialization/deserialization code

## Project Structure

```
build_rs/
├── baker.yaml              # Baker template configuration
├── build.rs                # Build script using Baker
├── Cargo.toml              # Project dependencies
├── src/
│   └── main.rs             # Main application that uses generated code
└── templates/
    └── generated.rs.baker.j2   # Template for generated Rust code
```

## How It Works

1. `build.rs` uses the Baker core library to render templates
2. Templates are rendered with context from `baker.yaml` or programmatic values
3. Generated code is written to `OUT_DIR` 
4. Main code includes the generated file using `include!` macro

## Running the Example

```bash
cd examples/build_rs
cargo build
cargo run
```

## Key Points

- Baker is added as a **build dependency** in `Cargo.toml`
- The `build.rs` uses `baker::renderer::new_renderer()` to create a template engine
- Templates can be loaded from files or defined inline
- Generated files go to `OUT_DIR` to avoid polluting the source tree
- Use `println!("cargo::rerun-if-changed=...")` to trigger rebuilds when templates change
