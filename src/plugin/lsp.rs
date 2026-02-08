use anyhow::Result;
use serde_json::Value;

pub struct LspPlugin {
    name: String,
}

impl LspPlugin {
    pub unsafe fn load_default() -> Result<Option<Self>> {
        // Placeholder implementation
        Ok(Some(Self {
            name: "Default LSP".to_string(),
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initialize(&mut self, _root_uri: &str, _doc_uri: &str, _content: &str) -> Result<()> {
        Ok(())
    }

    pub fn did_change(&mut self, _doc_uri: &str, _version: i32, _content: &str) -> Result<()> {
        Ok(())
    }

    pub fn completion(&mut self, _doc_uri: &str, _line: usize, _character: usize) -> Result<Value> {
        // Return empty completion list
        Ok(serde_json::json!({
            "items": []
        }))
    }
}
