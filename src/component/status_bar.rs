use gpui::*;
use crate::editor::CodeEditor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub struct StatusBar {
    editor: Entity<CodeEditor>,
    git_branch: String,
    #[allow(dead_code)]
    git_check_task: Option<Task<()>>,
}

impl StatusBar {
    pub fn new(editor: Entity<CodeEditor>, cx: &mut Context<Self>) -> Self {
        let mut this = Self { 
            editor, 
            git_branch: "Checking...".to_string(),
            git_check_task: None,
        };
        this.start_git_check(cx);
        this
    }

    fn start_git_check(&mut self, cx: &mut Context<Self>) {
        self.git_check_task = Some(cx.spawn(|view: WeakEntity<StatusBar>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
            loop {
                // 1. Get current file path (doc_uri) from editor
                let doc_uri: anyhow::Result<String> = view.update(&mut cx, |this, cx: &mut Context<StatusBar>| {
                    let editor = this.editor.read(cx);
                    editor.lsp_manager.doc_uri.clone()
                });

                if let Ok(uri) = doc_uri {
                    // 2. Resolve path
                    let path: PathBuf = if uri.starts_with("file:///") {
                        PathBuf::from(uri.trim_start_matches("file:///"))
                    } else if uri.starts_with("file://") {
                         PathBuf::from(uri.trim_start_matches("file://"))
                    } else {
                        PathBuf::from(&uri)
                    };

                    // 3. Run Git check in background
                    let branch = cx.background_executor().spawn(async move {
                        Self::check_git_status(&path)
                    }).await;

                    // 4. Update UI
                    view.update(&mut cx, |this, cx: &mut Context<StatusBar>| {
                        if this.git_branch != branch {
                            this.git_branch = branch;
                            cx.notify();
                        }
                    }).ok();
                }

                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        }
        }));
    }

    fn check_git_status(path: &Path) -> String {
        // First check if git is installed
        match Command::new("git").arg("--version").output() {
            Ok(_) => {
                // Git exists, check branch
                // We need a working directory. Use file's parent or project root.
                let cwd = if path.is_file() {
                    path.parent()
                } else {
                    Some(path)
                };

                if let Some(dir) = cwd {
                    match Command::new("git")
                        .arg("rev-parse")
                        .arg("--abbrev-ref")
                        .arg("HEAD")
                        .current_dir(dir)
                        .output() 
                    {
                        Ok(output) => {
                            if output.status.success() {
                                String::from_utf8_lossy(&output.stdout).trim().to_string()
                            } else {
                                // Not a git repo or error
                                "".to_string()
                            }
                        }
                        Err(_) => "".to_string(),
                    }
                } else {
                    "".to_string()
                }
            }
            Err(_) => "未安装Git".to_string(),
        }
    }

    fn get_language(path: &str) -> &'static str {
        if path.ends_with(".rs") {
            "Rust"
        } else if path.ends_with(".c") || path.ends_with(".h") || path.ends_with(".cpp") || path.ends_with(".hpp") || path.ends_with(".cc") || path.ends_with(".hh") {
            "C++"
        } else if path.ends_with(".js") || path.ends_with(".mjs") || path.ends_with(".cjs") || path.ends_with(".jsx") {
            "JavaScript"
        } else if path.ends_with(".ts") || path.ends_with(".tsx") {
            "TypeScript"
        } else if path.ends_with(".json") || path.ends_with(".jsonc") {
            "JSON"
        } else if path.ends_with(".md") || path.ends_with(".markdown") {
            "Markdown"
        } else if path.ends_with(".toml") {
            "TOML"
        } else if path.ends_with(".yml") || path.ends_with(".yaml") {
            "YAML"
        } else if path.ends_with(".html") || path.ends_with(".htm") {
            "HTML"
        } else if path.ends_with(".css") || path.ends_with(".scss") || path.ends_with(".less") {
            "CSS"
        } else if path.ends_with(".py") {
            "Python"
        } else if path.ends_with(".sh") || path.ends_with(".bash") || path.ends_with(".zsh") {
            "Shell"
        } else if path.ends_with(".t") {
            "结绳"
        } else if path.ends_with(".cmake") || path.to_lowercase().ends_with("cmakelists.txt") {
            "CMake"
        } else {
            "Plain Text"
        }
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.read(cx);
        let core = &editor.core;
        let primary = core.primary_selection();
        let head = primary.head;
        
        // Calculate Line/Col
        // Note: Ropey byte_to_line and byte_to_char are 0-indexed
        let line = core.content.byte_to_line(head);
        let line_start = core.content.line_to_byte(line);
        // Column is char offset from start of line
        let col = core.content.byte_to_char(head) - core.content.byte_to_char(line_start);
        
        let line_display = line + 1;
        let col_display = col + 1;

        let uri = &editor.lsp_manager.doc_uri;
        let language = Self::get_language(uri);
        
        let git_branch = &self.git_branch;
        
        // Encoding placeholder (Ropey is UTF-8)
        let encoding = "UTF-8";

        let theme_bg = rgb(0xff1f2428); // Matches other dark backgrounds like titlebar/tabs
        let theme_text = rgb(0xffd1d5da);
        let theme_border = rgb(0xff3c474d);

        div()
            .w_full()
            .h(px(24.0)) // Slightly smaller than 30px for a status bar feel
            .bg(theme_bg)
            .border_t_1()
            .border_color(theme_border)
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .text_size(px(12.0))
            .text_color(theme_text)
            // Left side: Git status
            .child(
                div().flex().items_center().child(
                    div().flex().items_center().mr(px(10.0))
                        //.child(tie_svg::tie_svg().path("assets/icons/git_branch.svg").size(px(12.0)).color(theme_text).into_any_element())
                        .child(div().ml(px(4.0)).child(format!("Git: {}", git_branch)))
                )
            )
            // Right side: Info
            .child(
                div().flex().items_center()
                    .child(div().mr(px(15.0)).child(format!("Ln {}, Col {}", line_display, col_display)))
                    .child(div().mr(px(15.0)).child(encoding))
                    .child(div().mr(px(15.0)).child(language))
                    .child(div().child("LSP: Ready"))
            )
    }
}

// Need to import tie_svg for the icon
use crate::component::tie_svg;
