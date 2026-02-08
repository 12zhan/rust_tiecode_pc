
#[cfg(test)]
mod tests {
    use crate::editor::grammar::CPP_GRAMMAR;
    use crate::editor::CodeEditor;
    use tiecode::sweetline::{Engine, Document, DocumentAnalyzer};

    #[test]
    fn test_jiesheng_single_line_embedded_cpp() {
        use crate::editor::grammar::JIESHENG_GRAMMAR;
        let engine = Engine::new(true);
        engine.compile_json(CPP_GRAMMAR).expect("Failed to compile CPP");
        engine.compile_json(JIESHENG_GRAMMAR).expect("Failed to compile JIESHENG");

        // Test single-line embed code
        let code = "code int main() {}";
        let doc = Document::new("test_single.t", code);
        let analyzer = engine.load_document(&doc);
        let raw_result = analyzer.analyze();
        let spans = DocumentAnalyzer::parse_result(&raw_result, false);

        let mut found_int_keyword = false;
        for span in spans {
            let style_name = engine.get_style_name(span.style_id).unwrap_or_default();
            let token = &code[span.start_index as usize..span.end_index as usize];
            println!("Token: '{}', Style: {}", token, style_name);
            if token == "int" && style_name == "keyword" {
                found_int_keyword = true;
            }
        }
        assert!(found_int_keyword, "Should find 'int' keyword in single-line embedded C++ block");
    }

    #[test]
    fn test_cpp_highlighting() {
        let engine = Engine::new(true);
        let result = engine.compile_json(CPP_GRAMMAR);
        assert!(result.is_ok(), "Grammar compilation failed: {:?}", result.err());

        let code = "int main() {\n    return 0;\n}";
        let doc = Document::new("test.cpp", code);
        let analyzer = engine.load_document(&doc);
        let raw_result = analyzer.analyze();
        
        println!("Raw result size: {}", raw_result.len());
        println!("Raw result: {:?}", raw_result);

        let spans = DocumentAnalyzer::parse_result(&raw_result, false);
    println!("Parsed spans: {}", spans.len());

    for span in spans {
        let style_name = engine.get_style_name(span.style_id);
        println!("Span: {:?}, StyleID: {}, Name: {:?}", span, span.style_id, style_name);
        assert!(style_name.is_some(), "Style name should not be None for ID {}", span.style_id);
        
        let name = style_name.unwrap();
        // Check specific tokens
        let token = &code[span.start_index as usize..span.end_index as usize];
        if token == "int" || token == "return" {
            assert_eq!(name, "keyword", "Token '{}' should be keyword, got {}", token, name);
        } else if token == "main" {
            assert_eq!(name, "function", "Token '{}' should be function, got {}", token, name);
        } else if token == "0" {
            assert_eq!(name, "number", "Token '{}' should be number, got {}", token, name);
        }
    }
    
    assert!(!raw_result.is_empty(), "Analyze returned empty result");
}

    #[test]
    fn test_engine_remove_document_allows_reloading_same_uri() {
        let engine = Engine::new(true);
        engine.compile_json(CPP_GRAMMAR).unwrap();

        let code_v1 = "int a = 1;";
        let doc_v1 = Document::new("same.cpp", code_v1);
        let analyzer_v1 = engine.load_document(&doc_v1);
        let spans_v1 = DocumentAnalyzer::parse_result(&analyzer_v1.analyze(), false);
        assert!(
            spans_v1
                .iter()
                .any(|s| engine.get_style_name(s.style_id).as_deref() == Some("keyword")),
            "Expected keyword highlight in v1"
        );

        engine.remove_document("same.cpp").unwrap();

        let code_v2 = "return 0;";
        let doc_v2 = Document::new("same.cpp", code_v2);
        let analyzer_v2 = engine.load_document(&doc_v2);
        let spans_v2 = DocumentAnalyzer::parse_result(&analyzer_v2.analyze(), false);

        let mut saw_return_keyword = false;
        for span in spans_v2 {
            let name = engine.get_style_name(span.style_id).unwrap_or_default();
            let token = &code_v2[span.start_index as usize..span.end_index as usize];
            if token == "return" && name == "keyword" {
                saw_return_keyword = true;
                break;
            }
        }
        assert!(saw_return_keyword, "Expected 'return' to be keyword in v2");
    }

    #[test]
    fn test_byte_offset_for_char_offset_is_utf8_boundary() {
        let text = "类 启动类";
        let offsets = [
            CodeEditor::byte_offset_for_char_offset(text, 0),
            CodeEditor::byte_offset_for_char_offset(text, 1),
            CodeEditor::byte_offset_for_char_offset(text, 2),
            CodeEditor::byte_offset_for_char_offset(text, 3),
        ];
        for off in offsets {
            assert!(
                text.is_char_boundary(off),
                "offset {} must be char boundary for '{}'",
                off,
                text
            );
        }
        assert_eq!(CodeEditor::byte_offset_for_char_offset(text, 1), 3);
        assert_eq!(CodeEditor::byte_offset_for_char_offset(text, 2), 4);
    }

    #[test]
    fn test_jiesheng_embedded_cpp() {
        use crate::editor::grammar::JIESHENG_GRAMMAR;
        let engine = Engine::new(true);
        engine.compile_json(CPP_GRAMMAR).expect("Failed to compile CPP");
        engine.compile_json(JIESHENG_GRAMMAR).expect("Failed to compile JIESHENG");

        // Test multi-line embed code
        let code = "@code\nint main() {}\n@end";
        let doc = Document::new("test.t", code);
        let analyzer = engine.load_document(&doc);
        let raw_result = analyzer.analyze();
        let spans = DocumentAnalyzer::parse_result(&raw_result, false);

        let mut found_int_keyword = false;
        for span in spans {
            let style_name = engine.get_style_name(span.style_id).unwrap_or_default();
            let token = &code[span.start_index as usize..span.end_index as usize];
            println!("Token: '{}', Style: {}", token, style_name);
            if token == "int" && style_name == "keyword" {
                found_int_keyword = true;
            }
        }
        assert!(found_int_keyword, "Should find 'int' keyword in embedded C++ block");
    }

    #[test]
    fn test_java_highlighting() {
        use crate::editor::grammar::JAVA_GRAMMAR;
        let engine = Engine::new(true);
        let result = engine.compile_json(JAVA_GRAMMAR);
        assert!(result.is_ok(), "Grammar compilation failed: {:?}", result.err());

        let code = "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello\");\n    }\n}";
        let doc = Document::new("Main.java", code);
        let analyzer = engine.load_document(&doc);
        let raw_result = analyzer.analyze();
        
        let spans = DocumentAnalyzer::parse_result(&raw_result, false);

        for span in spans {
            let style_name = engine.get_style_name(span.style_id);
            if let Some(name) = style_name {
                let token = &code[span.start_index as usize..span.end_index as usize];
                
                if token == "public" || token == "class" || token == "static" || token == "void" {
                    assert_eq!(name, "keyword", "Token '{}' should be keyword", token);
                } else if token == "Main" || token == "String" || token == "System" {
                    assert_eq!(name, "type", "Token '{}' should be type", token);
                } else if token == "\"Hello\"" {
                    assert_eq!(name, "string", "Token '{}' should be string", token);
                }
            }
        }
    }

    #[test]
    fn test_core_replace_range() {
        use crate::editor::core::EditorCore;
        use ropey::Rope;
        
        let mut core = EditorCore::new();
        core.content = Rope::from("test");
        let range = 4..4;
        let text = "a";
        
        core.replace_range(range.clone(), text);
        
        assert_eq!(core.content.len_bytes(), 5);
        assert_eq!(core.content.to_string(), "testa");
        
        // Test removing
        core.replace_range(4..5, "");
        assert_eq!(core.content.len_bytes(), 4);
        assert_eq!(core.content.to_string(), "test");
        
        // Test replace in middle
        core.replace_range(1..3, "oo");
        assert_eq!(core.content.to_string(), "toot");

        // Test replace clears marked_range
        core.marked_range = Some(0..4);
        core.replace_range(4..4, "s");
        assert!(core.marked_range.is_none(), "marked_range should be cleared after replace_range");
    }

    #[test]
    fn test_jiesheng_incremental_edit_crash() {
        use crate::editor::grammar::JIESHENG_GRAMMAR;
        let engine = Engine::new(true);
        engine.compile_json(CPP_GRAMMAR).expect("Failed to compile CPP");
        engine.compile_json(JIESHENG_GRAMMAR).expect("Failed to compile JIESHENG");

        let code = "@code\nint main() {\n}\n@end";
        let doc = Document::new("test_crash.t", code);
        let analyzer = engine.load_document(&doc);
        let _ = analyzer.analyze();

        // Simulate inserting text inside the embedded C++ block
        // Insert "    return 0;\n" inside main()
        // Line 1 is "int main() {"
        let start_line = 1;
        let start_col = 12; // After '{'
        let end_line = 1;
        let end_col = 12;
        let new_text = "\n    return 0;";

        println!("Starting incremental analysis...");
        let result = analyzer.analyze_incremental(start_line, start_col, end_line, end_col, new_text);
        println!("Incremental analysis finished. Result size: {}", result.len());
        
        let spans = DocumentAnalyzer::parse_result(&result, false);
        for span in spans {
             let style_name = engine.get_style_name(span.style_id).unwrap_or_default();
             let token = if span.start_line == 2 {
                 "return" // approximated check
             } else {
                 ""
             };
             println!("Span line {}: {:?}", span.start_line, style_name);
        }
    }
}
