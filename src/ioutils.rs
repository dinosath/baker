use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Ensures the output directory exists and is safe to write to.
///
/// # Arguments
/// * `output_dir` - The directory path to check
/// * `force` - Whether to allow overwriting an existing directory
///
/// # Returns
/// * `Result<PathBuf>` - The validated output directory path
///
/// # Errors
/// Returns `Error::OutputDirectoryExistsError` if the directory exists and `force` is false
pub fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(Error::OutputDirectoryExistsError {
            output_dir: output_dir.display().to_string(),
        });
    }
    Ok(output_dir.to_path_buf())
}

/// Create directory and all parent directories if they don't exist.
///
/// # Arguments
/// * `dest_path` - Directory path to create
///
/// # Returns
/// * `Result<()>` - Success or error
///
/// # Errors
/// Returns an error if directory creation fails
pub fn create_dir_all<P: AsRef<Path>>(dest_path: P) -> Result<()> {
    std::fs::create_dir_all(dest_path.as_ref()).map_err(Error::from)
}

/// Write content to a file, creating parent directories if needed.
///
/// # Arguments
/// * `content` - Content to write to the file
/// * `dest_path` - Path where to write the file
///
/// # Returns
/// * `Result<()>` - Success or error
///
/// # Errors
/// Returns an error if file writing fails
pub fn write_file<P: AsRef<Path>>(content: &str, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent)?;
    }

    std::fs::write(dest_path, content).map_err(Error::from)
}

/// Copy a file from source to destination, creating parent directories if needed.
///
/// # Arguments
/// * `source_path` - Source file path
/// * `dest_path` - Destination file path
///
/// # Returns
/// * `Result<()>` - Success or error
pub fn copy_file<P: AsRef<Path>>(source_path: P, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent)?;
    }

    Ok(std::fs::copy(source_path.as_ref(), dest_path).map(|_| ())?)
}

/// Parse a string into a JSON object.
///
/// # Arguments
/// * `buf` - JSON string to parse
///
/// # Returns
/// * `Result<serde_json::Map<String, serde_json::Value>>` - Parsed JSON object
pub fn parse_string_to_json(
    buf: String,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let value = serde_json::from_str(&buf)?;

    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Ok(serde_json::Map::new()),
    }
}

/// Read content from a reader into a string.
///
/// # Arguments
/// * `reader` - Anything that implements Read trait
///
/// # Returns
/// * `Result<String>` - Content as string
pub fn read_from(mut reader: impl std::io::Read) -> Result<String> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Converts a path to a string slice, returning an error if the path contains invalid Unicode characters.
///
/// # Arguments
/// * `path` - A reference to a type that can be converted to a [`Path`]
///
/// # Returns
/// * `Ok(&str)` - A string slice representing the path
/// * `Err(Error)` - If the path contains invalid Unicode characters
///
/// # Examples
/// ```
/// use baker::ioutils::path_to_str;
/// use std::path::Path;
///
/// let path = Path::new("test");
/// assert_eq!(path_to_str(path).unwrap(), "test");
///
/// // Invalid paths will return an error
/// #[cfg(unix)]
/// {
///     use std::os::unix::ffi::OsStrExt;
///     use std::ffi::OsStr;
///     let invalid_bytes = [0x80, 0x00];
///     let invalid_path = Path::new(OsStr::from_bytes(&invalid_bytes));
///     assert!(path_to_str(invalid_path).is_err());
/// }
/// ```
pub fn path_to_str<P: AsRef<Path> + ?Sized>(path: &P) -> Result<&str> {
    path.as_ref().to_str().ok_or_else(|| {
        Error::Other(anyhow::anyhow!(
            "Path '{}' contains invalid Unicode characters",
            path.as_ref().display()
        ))
    })
}
