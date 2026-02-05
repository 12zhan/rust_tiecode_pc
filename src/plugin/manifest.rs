use anyhow::{Context, Result};
use std::path::Path;
use tiecode_plugin_api::PluginManifest;

pub struct PluginManifestLoader;

impl PluginManifestLoader {
    pub fn load(path: &Path) -> Result<PluginManifest> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read plugin manifest at {:?}", path))?;
        
        let manifest: PluginManifest = serde_json::from_str(&content)
            .with_context(|| "Failed to parse plugin manifest")?;
            
        Ok(manifest)
    }
}
