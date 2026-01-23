/// Extension traits for built-in Rust types.
///
/// This module contains various extension traits that add convenient methods
/// to standard library types. These traits provide domain-specific functionality
/// that's commonly needed throughout the Baker codebase.
///
/// # Organization
///
/// Each extension trait should be placed in its own file named after the type
/// it extends:
/// - `path.rs` - Extensions for `std::path::Path`
/// - `string.rs` - Extensions for `String` and `&str` (future)
/// - `collections.rs` - Extensions for `Vec`, `HashMap`, etc. (future)
///
/// # Adding New Extension Traits
///
/// 1. Create a new file in this directory (e.g., `string.rs`)
/// 2. Define your trait and implementation
/// 3. Add the module declaration below
/// 4. Re-export the trait in the `pub use` section
/// 5. Update any relevant documentation
///
/// # Example
///
/// ```rust
/// // In src/ext/string.rs
/// pub trait StringExt {
///     fn to_snake_case(&self) -> String;
/// }
///
/// impl StringExt for str {
///     fn to_snake_case(&self) -> String {
///         // implementation would go here
/// #       String::new() // placeholder
///     }
/// }
/// ```
pub mod path;
// pub mod string;     // Future extension traits
// pub mod collections; // Future extension traits

// Re-export all extension traits for convenience
pub use path::PathExt;
// pub use string::StringExt;         // Future
// pub use collections::{VecExt, HashMapExt}; // Future
