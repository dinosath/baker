use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::{Result, Error};
use crate::config::loader::ConfigV1;

pub const META_FILENAME: &str = ".baker-meta.yaml";

#[derive(Debug, Serialize, Deserialize)]
pub struct BakerMeta {
    pub baker_version: String,
    pub template_source: String,
    pub template_root: String,
    pub git_commit: Option<String>,
    pub answers: serde_json::Value,
    pub config: ConfigV1,
}

impl BakerMeta {
    pub fn path<P: AsRef<Path>>(output_root: P) -> PathBuf { output_root.as_ref().join(META_FILENAME) }
    pub fn load<P: AsRef<Path>>(output_root: P) -> Result<Option<Self>> {
        let path = Self::path(&output_root);
        if !path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(path)?;
        let meta: BakerMeta = serde_yaml::from_str(&content)?;
        Ok(Some(meta))
    }
    pub fn save<P: AsRef<Path>>(&self, output_root: P) -> Result<()> {
        let path = Self::path(&output_root);
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let serialized = serde_yaml::to_string(self)?;
        std::fs::write(path, serialized).map_err(Error::from)
    }
}
