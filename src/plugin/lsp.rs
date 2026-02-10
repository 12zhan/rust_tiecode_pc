use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use log::info;
use crate::lsp::tiec::wrapper::{TiecLoader, TiecIdeService};
use crate::lsp::tiec::types::{
    CompilerOptions, CompletionParams, CursorParams, Position, SearchPrefixes,
};
use url::Url;
use std::path::PathBuf;

pub struct LspPlugin {
    name: String,
    service: Option<Arc<TiecIdeService>>,
    loader: Arc<TiecLoader>,
    root_uri: Option<String>,
    dll_path: Option<PathBuf>,
}

impl LspPlugin {
    pub unsafe fn load_default() -> Result<Option<Self>> {
        // Try to load tiec.dll from common locations
        let paths = [
            "tiec.dll",
            "bin/tiec.dll", 
            "libs/tiec.dll",
            "../tiec.dll",
        ];
        
        for path in paths {
            if let Ok(loader) = TiecLoader::new(path) {
                let dll_path = std::fs::canonicalize(path).ok();
                return Ok(Some(Self {
                    name: "Tiec LSP".to_string(),
                    service: None,
                    loader: Arc::new(loader),
                    root_uri: None,
                    dll_path,
                }));
            }
        }
        
        // If not found, return Ok(None) or Err depending on if it's critical.
        // For now, let's assume it's optional but log if not found?
        // Actually, if the user requested it, it should probably fail if not found.
        // But the original code returned Ok(None).
        Ok(None)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn find_sdk_path(&self) -> Option<String> {
        let candidates = [
            self.dll_path.as_ref().and_then(|p| p.parent()).map(|p| p.join("sdk")),
            self.dll_path.as_ref().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|p| p.join("sdk")),
            Some(PathBuf::from("C:/tiecode/sdk")),
            Some(PathBuf::from("sdk")),
        ];

        for candidate in candidates.iter().flatten() {
            if candidate.exists() && candidate.is_dir() {
                 return Some(candidate.to_string_lossy().to_string());
            }
        }
        
        // Return None if not found, let compiler try default or fail gracefully
        None
    }

    pub fn initialize(&mut self, root_uri: &str, doc_uri: &str, content: &str) -> Result<()> {
        println!("DEBUG: initialize called for root: {}, doc: {}", root_uri, doc_uri);
        // Check if root has changed
        let root_changed = self.root_uri.as_deref() != Some(root_uri);
        
        // Create context and service if not already created or if root changed
        if self.service.is_none() || root_changed {
            if root_changed {
                info!("Project root changed to: {}", root_uri);
                self.root_uri = Some(root_uri.to_string());
                self.service = None; // clear old service
            }

            let root_path = Url::parse(root_uri)
                .ok()
                .and_then(|u| u.to_file_path().ok())
                .map(|p| p.to_string_lossy().to_string());

            let package_name = root_path.as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            
            let sdk_path = self.find_sdk_path();
            if sdk_path.is_none() {
                 println!("Warning: TieCode SDK not found. Compiler may fail.");
            } else {
                 println!("Using SDK path: {:?}", sdk_path);
            }

            let mut options = CompilerOptions {
                ide_mode: true,
                output_dir: root_path.clone(),
                package_name,
                sdk_path,
                target: Some("android".to_string()),
                ..Default::default()
            };

            if let Some(path) = &root_path {
                 options.search_prefixes = Some(SearchPrefixes {
                     source: Some(vec![path.clone()]),
                     ..Default::default()
                 });
            }
            
            let context = self.loader.create_context(&serde_json::to_value(&options)?)?;
            let service = context.create_ide_service()?;
            let service = Arc::new(service);
            self.service = Some(service.clone());

            // Scan and compile project files
            if let Some(path) = root_path {
                let mut files = Vec::new();
                info!("Scanning project files in: {}", path);
                scan_files(std::path::Path::new(&path), &mut files);
                if !files.is_empty() {
                    println!("DEBUG: Compiling {} files", files.len());
                    if let Err(e) = service.compile_files(&files) {
                        println!("DEBUG: Compilation failed: {:?}", e);
                    } else {
                        println!("DEBUG: Compilation success");
                    }
                } else {
                    println!("DEBUG: No .t files found in project root");
                }
            }
        }

        if let Some(service) = &self.service {
            // Register the source file
            // Note: create_source might fail if it already exists, so we might want to try delete first or ignore error
            let _ = service.delete_source(doc_uri); // Ensure clean state
            println!("DEBUG: Calling create_source for {}", doc_uri);
            if let Err(e) = service.create_source(doc_uri, content) {
                println!("DEBUG: create_source failed: {:?}", e);
                return Err(e);
            }
            
            // Trigger initial compilation/analysis
            // tc_ide_service_compile_files could be used here if we had file path
            // But create_source should be enough for single file analysis
        }

        Ok(())
    }

    pub fn did_change(&mut self, doc_uri: &str, _version: i32, content: &str) -> Result<()> {
        if let Some(service) = &self.service {
            service.edit_source(doc_uri, content)?;
        }
        Ok(())
    }

    pub fn did_create_file(&mut self, doc_uri: &str, initial_text: &str) -> Result<()> {
        if let Some(service) = &self.service {
            service.create_source(doc_uri, initial_text)?;
        }
        Ok(())
    }

    pub fn did_delete_file(&mut self, doc_uri: &str) -> Result<()> {
        if let Some(service) = &self.service {
            service.delete_source(doc_uri)?;
        }
        Ok(())
    }

    pub fn did_rename_file(&mut self, old_uri: &str, new_uri: &str) -> Result<()> {
        if let Some(service) = &self.service {
            service.rename_source(old_uri, new_uri)?;
        }
        Ok(())
    }

    pub fn completion(&mut self, doc_uri: &str, line: usize, character: usize, _index: usize, prefix: &str, trigger_char: &str) -> Result<Value> {
        if let Some(service) = &self.service {
            // Use CompletionParams struct to ensure correct JSON structure (nested position, camelCase)
            let params = CompletionParams {
                    uri: doc_uri.to_string(),
                    position: Position {
                        line,
                        column: character,
                    },
                    line_text: None, // Kotlin demo has this nullable/optional
                    partial: prefix.to_string(),
                    trigger_char: if trigger_char.is_empty() { None } else { Some(trigger_char.to_string()) },
                };
            let json_params = serde_json::to_value(&params)?;
                println!("DEBUG: Completion params: {}", json_params);
                
                match service.complete(&json_params) {
                    Ok(result) => {
                        let result_value = serde_json::to_value(&result)?;
                        println!("DEBUG: Completion result: {}", result_value);
                        return Ok(result_value);
                    }
                    Err(e) => {
                        println!("DEBUG: Completion failed: {:?}", e);
                        return Err(anyhow::anyhow!("Completion failed: {:?}", e));
                    }
                }
            }
            
            Ok(serde_json::json!({
                "items": []
            }))
    }

    pub fn hover(&mut self, doc_uri: &str, line: usize, character: usize, _index: usize) -> Result<Value> {
        if let Some(service) = &self.service {
            let params = CursorParams {
                uri: doc_uri.to_string(),
                position: Position { line, column: character },
                line_text: None,
            };
            
            let result = service.hover(&params)?;
            return Ok(serde_json::to_value(result)?);
        }
        Ok(serde_json::Value::Null)
    }
}

fn scan_files(path: &std::path::Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_files(&path, files);
            } else if let Some(ext) = path.extension() {
                if ext == "t" {
                     files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
}
