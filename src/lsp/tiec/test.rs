#[cfg(test)]
mod tests {
    use super::super::wrapper::*;
    use super::super::types::*;
    use anyhow::Result;
    use log::{info, debug};
    use url::Url;
    use serde_json::json;

    // Helper to setup logger for tests
    fn init_logger() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    // Helper to find all .t files recursively
    fn find_t_files(dir: &std::path::Path) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(find_t_files(&path));
                } else if let Some(ext) = path.extension() {
                    if ext == "t" {
                        if let Ok(s) = path.into_os_string().into_string() {
                            files.push(s);
                        }
                    }
                }
            }
        }
        files
    }

    #[test]
    fn test_ide_service_demo1() -> Result<()> {
        init_logger();
        
        let project_dir = r"C:\Users\xiaoa\.tiecode\project\Demo1";
        let file_path = r"C:\Users\xiaoa\.tiecode\project\Demo1\源代码\初始代码.t";
        
        // Use standard URI format: file:///C:/...
        let file_uri = Url::from_file_path(file_path).unwrap().to_string();
        
        println!("Project Dir: {}", project_dir);
        println!("File Path: {}", file_path);
        println!("File URI: {}", file_uri);

        // Scan for .t files first
        println!("Scanning for .t files in project...");
        let all_files = find_t_files(std::path::Path::new(project_dir));
        println!("Found {} files", all_files.len());
        for f in &all_files {
            debug!("File: {}", f);
        }

        let paths = [
            "tiec.dll",
            "bin/tiec.dll", 
            "libs/tiec.dll",
            "../tiec.dll",
            "../../tiec.dll", 
            "../../../tiec.dll",
            "c:/Users/xiaoa/Desktop/tie_rust_gpui/tiecode/tiec.dll",
            "c:/Users/xiaoa/Desktop/tie_rust_gpui/tiecode/bin/tiec.dll",
        ];
        
        let mut loader = None;
        for path in paths {
            if let Ok(l) = unsafe { TiecLoader::new(path) } {
                loader = Some(l);
                println!("Found tiec.dll at: {}", path);
                break;
            }
        }
        
        if loader.is_none() {
            println!("Skipping test: tiec.dll not found");
            return Ok(());
        }
        let loader = loader.unwrap();

        // Create options using raw JSON to match Dart demo exactly
        // Dart: {"sdk_path": "C:/tiecode/sdk", "target": "android"}
        // Note: We need to make sure C:/tiecode/sdk actually exists or is valid.
        // If not, we might need to point to a valid SDK or assume it's optional but keys are needed.
        let options = json!({
            "sdk_path": "C:/tiecode/sdk", 
            "target": "android",
            "ide_mode": false,
            "package_name": "Demo1",
            "output_dir": project_dir
        });

        println!("Creating context with options: {}", options);
        let context = loader.create_context(&options)?;
        println!("Creating IDE service...");
        let service = context.create_ide_service()?;
        
        // Note: compile_files causes access violation even with ASCII files.
        // We rely on create_source for all files.
        /*
        // Test compile_files with ASCII file
        let ascii_file = format!("{}\\test_ascii.t", project_dir);
        std::fs::write(&ascii_file, "class Ascii {}")?;
        println!("Created temp ASCII file: {}", ascii_file);
        
        println!("Testing compile_files with ASCII file...");
        if let Err(e) = service.compile_files(&[ascii_file.clone()]) {
            println!("compile_files ASCII failed: {:?}", e);
        } else {
            println!("compile_files ASCII success");
        }
        */

        println!("Registering project files via create_source...");
        for file_path_str in &all_files {
            // Read file content
            // Skip if read fails (might be directory or inaccessible)
            if let Ok(content) = std::fs::read_to_string(file_path_str) {
                // Use standard URI
                if let Ok(uri) = Url::from_file_path(file_path_str) {
                    let uri_str = uri.to_string();
                    debug!("Registering file: {}", uri_str);
                    // Call create_source
                    if let Err(e) = service.create_source(&uri_str, &content) {
                        println!("Failed to register file {}: {:?}", file_path_str, e);
                    }
                }
            }
        }
        println!("Project files registered.");

        // Proceed with completion test
        // Read file content for the target file
        let content = std::fs::read_to_string(file_path).unwrap_or_else(|_| {
            println!("Warning: Could not read file content");
            "".to_string()
        });
        
        println!("File content len: {}", content.len());

        // Ensure the main file is definitely open/updated (redundant but safe)
        println!("Creating source with URI: {}", file_uri);
        service.create_source(&file_uri, &content)?;
        println!("Source created successfully");

        // Test completion on a virtual ASCII file first to rule out encoding/content issues
        let virtual_uri = "file:///virtual_test.t";
        let virtual_content = "class A {}";
        service.create_source(virtual_uri, virtual_content)?;
        
        // Use flat JSON structure for completion params to match Dart demo
        let virtual_params = json!({
            "uri": virtual_uri,
            "line": 0,
            "column": 0
        });

        println!("Testing completion on virtual ASCII file...");
        if let Ok(res) = service.complete(&virtual_params) {
             println!("Virtual completion success: {:?}", res);
        } else {
             println!("Virtual completion failed");
        }

        // Return early to isolate virtual completion test
        // return Ok(());

        // User provided 11:10 (1-based).
        // 0-based: Line 10, Column 9.
        let line = 10;
        let column = 9;
        
        // Calculate index (byte offset) - useful for verification but not sent to DLL
        let mut index = 0;
        let mut found = false;
        let mut current_line = 0;
        let mut current_col = 0;
        let mut line_content = String::new();
        
        for (i, c) in content.char_indices() {
            if current_line == line {
                line_content.push(c);
            }
            if current_line == line && current_col == column {
                index = i;
                found = true;
            }
            if c == '\n' {
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
        }
        
        if !found {
            if current_line == line && current_col == column {
                index = content.len();
                found = true;
            } else {
                println!("Warning: Could not find exact index for {}:{}, using 0", line, column);
            }
        }
        
        println!("Calculated index: {}", index);
        println!("Line content: {}", line_content);

        // Test with ASCII URI but real content to rule out URI encoding issues
        let ascii_copy_path = std::path::Path::new(project_dir).join("test_copy.t");
        let ascii_copy_uri = Url::from_file_path(&ascii_copy_path).unwrap().to_string();
        service.create_source(&ascii_copy_uri, &content)?;
        
        let params = json!({
            "uri": ascii_copy_uri,
            "line": line,
            "column": column
        });

        println!("Requesting completion on ASCII URI copy...");
        let result = service.complete(&params)?;
        println!("Completion result: {:?}", result);
        
        Ok(())
    }
}
