use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Ensures the output directory exists and is safe to write to.
pub fn get_output_dir<P: AsRef<Path>>(output_dir: P, force: bool) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref();
    if output_dir.exists() && !force {
        return Err(Error::OutputDirectoryExistsError {
            output_dir: output_dir.display().to_string(),
        });
    }
    Ok(output_dir.to_path_buf())
}

pub fn create_dir_all<P: AsRef<Path>>(dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    Ok(std::fs::create_dir_all(dest_path)?)
}

pub fn write_file<P: AsRef<Path>>(content: &str, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_path = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_path.parent() {
        create_dir_all(parent)?;
    }
    Ok(std::fs::write(abs_path, content)?)
}

pub fn copy_file<P: AsRef<Path>>(source_path: P, dest_path: P) -> Result<()> {
    let dest_path = dest_path.as_ref();
    let source_path = source_path.as_ref();
    let base_path = std::env::current_dir().unwrap_or_default();
    let abs_dest = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        base_path.join(dest_path)
    };

    if let Some(parent) = abs_dest.parent() {
        create_dir_all(parent)?;
    }
    Ok(std::fs::copy(source_path, abs_dest).map(|_| ())?)
}

pub fn parse_string_to_json(
    buf: String,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let value = serde_json::from_str(&buf)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Ok(serde_json::Map::new()),
    }
}

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
    Ok(path.as_ref().to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "Path '{}' contains invalid Unicode characters",
            path.as_ref().display()
        )
    })?)
}
