
#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::grammar::CPP_GRAMMAR;
    use tiecode::sweetline::{Engine, Document, DocumentAnalyzer};
    use std::collections::HashMap;

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

}
