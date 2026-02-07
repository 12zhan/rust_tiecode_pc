use gpui::*;
use crate::editor::CodeEditor;
use std::path::Path;

pub struct StatusBar {
    editor: Entity<CodeEditor>,
}

impl StatusBar {
    pub fn new(editor: Entity<CodeEditor>) -> Self {
        Self { editor }
    }

    fn get_language(path: &str) -> &'static str {
        if path.ends_with(".rs") {
            "Rust"
        } else if path.ends_with(".c") || path.ends_with(".h") || path.ends_with(".cpp") || path.ends_with(".hpp") {
            "C++"
        } else if path.ends_with(".js") {
            "JavaScript"
        } else if path.ends_with(".ts") {
            "TypeScript"
        } else if path.ends_with(".json") {
            "JSON"
        } else if path.ends_with(".md") {
            "Markdown"
        } else if path.ends_with(".toml") {
            "TOML"
        } else if path.ends_with(".yml") || path.ends_with(".yaml") {
            "YAML"
        } else if path.ends_with(".html") {
            "HTML"
        } else if path.ends_with(".css") {
            "CSS"
        } else if path.ends_with(".py") {
            "Python"
        } else if path.ends_with(".sh") {
            "Shell"
        } else if path.ends_with(".t") {
            "结绳"
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
        
        // Git branch placeholder
        let git_branch = "main";
        
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
