use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]

pub enum TemplateMetadata {
    GitMetadata(GitMetadata),
    FileMetadata(FileMetadata),
}
#[derive(Serialize, Deserialize)]

pub struct GitMetadata {
    pub template_url: String,
    pub branch: String,
    pub tag: String,
    pub commit: String,
}
#[derive(Serialize, Deserialize)]

pub struct FileMetadata {
    pub directory: String,
}

#[derive(Serialize, Deserialize)]
pub struct GenerationMetadata {
    #[serde(flatten)]
    pub template_metadata: TemplateMetadata,
    pub answers: serde_json::Value,
}

impl GenerationMetadata {
    pub fn new(template_metadata: TemplateMetadata, answers: serde_json::Value) -> Self {
        Self { template_metadata, answers }
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}
