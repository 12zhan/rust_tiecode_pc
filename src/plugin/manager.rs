use std::collections::HashMap;
use std::path::PathBuf;
use tiecode_plugin_api::{PluginManifest, CommandContribution};

#[derive(Clone)]
pub struct ToolPageContribution {
    pub id: String,
    pub label: String,
    pub icon_path: Option<PathBuf>,
}

pub struct CommandRegistry {
    commands: HashMap<String, CommandContribution>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, command: CommandContribution) {
        self.commands.insert(command.command.clone(), command);
    }

    pub fn get(&self, id: &str) -> Option<&CommandContribution> {
        self.commands.get(id)
    }
    
    pub fn list(&self) -> Vec<&CommandContribution> {
        self.commands.values().collect()
    }
}

pub struct PluginManager {
    plugins: HashMap<String, PluginManifest>,
    plugin_dirs: Vec<PathBuf>,
    pub command_registry: CommandRegistry,
    pub tool_pages: Vec<ToolPageContribution>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dirs: Vec::new(),
            command_registry: CommandRegistry::new(),
            tool_pages: Vec::new(),
        }
    }

    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        self.plugin_dirs.push(path);
    }

    pub fn discover_plugins(&mut self) {
        for dir in &self.plugin_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let manifest_path = path.join("package.json");
                        if manifest_path.exists() {
                            match crate::plugin::manifest::PluginManifestLoader::load(&manifest_path) {
                                Ok(manifest) => {
                                    println!("Found plugin: {} ({})", manifest.id, manifest.version);
                                    
                                    // Auto-register commands from manifest
                                    for cmd in &manifest.contributes.commands {
                                        self.command_registry.register(cmd.clone());
                                    }
                                    
                                    self.plugins.insert(manifest.id.clone(), manifest);
                                }
                                Err(e) => {
                                    eprintln!("Failed to load plugin manifest at {:?}: {}", manifest_path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn activate_plugin(&self, plugin_id: &str) {
        if let Some(plugin) = self.plugins.get(plugin_id) {
            println!("Activating plugin: {}", plugin.name);
            // In a real implementation, this would start the plugin host/WASM module
        }
    }

    pub fn register_tool_page(&mut self, id: impl Into<String>, label: impl Into<String>, icon_path: Option<PathBuf>) {
        self.tool_pages.push(ToolPageContribution {
            id: id.into(),
            label: label.into(),
            icon_path,
        });
    }

    pub fn list_tool_pages(&self) -> &[ToolPageContribution] {
        &self.tool_pages
    }
}
