//! 结绳 IDE / 编译器协议定义

pub mod common;
pub mod completion;
pub mod diagnostics;
pub mod formatting;
pub mod hover;
pub mod options;
pub mod rename;
pub mod signature;
pub mod smart_enter;
pub mod symbols;
pub mod ui_binding;

pub use common::*;
pub use completion::*;
pub use diagnostics::*;
pub use formatting::*;
pub use hover::*;
pub use options::*;
pub use rename::*;
pub use signature::*;
pub use smart_enter::*;
pub use symbols::*;
pub use ui_binding::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn completion_params_serialize_optional_fields() {
        let params = CompletionParams {
            uri: "file:///C:/a.t".to_string(),
            position: Position { line: 1, column: 2 },
            line_text: None,
            partial: "ab".to_string(),
            trigger_char: None,
        };
        let value = serde_json::to_value(params).unwrap();
        assert!(value.get("lineText").is_none());
        assert!(value.get("triggerChar").is_none());
    }

    #[test]
    fn completion_params_serialize_with_optional_fields() {
        let params = CompletionParams {
            uri: "file:///C:/a.t".to_string(),
            position: Position { line: 1, column: 2 },
            line_text: Some("println".to_string()),
            partial: "pr".to_string(),
            trigger_char: Some(".".to_string()),
        };
        let value = serde_json::to_value(params).unwrap();
        assert_eq!(value.get("lineText").unwrap(), &json!("println"));
        assert_eq!(value.get("triggerChar").unwrap(), &json!("."));
    }

    #[test]
    fn tly_entity_parse_json_format() {
        let data = json!({
            "class": { "className": "线性布局" },
            "nameProp": {
                "propName": { "name": "名称" },
                "propValue": { "value": "布局1" }
            },
            "properties": [
                {
                    "propName": { "name": "宽度" },
                    "propValue": { "value": -1 }
                }
            ],
            "children": [
                {
                    "class": { "className": "文本框" },
                    "nameProp": {
                        "propName": { "name": "名称" },
                        "propValue": { "value": "文本框1" }
                    },
                    "properties": [
                        {
                            "propName": { "name": "内容" },
                            "propValue": { "value": "你好" }
                        }
                    ],
                    "children": []
                }
            ]
        });
        let entity: TlyEntity = serde_json::from_value(data).unwrap();
        assert_eq!(entity.class.class_name, "线性布局");
        assert_eq!(entity.name_prop.unwrap().prop_value.value, json!("布局1"));
        assert_eq!(entity.properties.len(), 1);
        assert_eq!(entity.children.len(), 1);
        assert_eq!(entity.children[0].class.class_name, "文本框");
    }

    #[test]
    fn highlight_result_defaults_tags() {
        let data = json!({
            "highlights": [
                {
                    "range": {
                        "start": { "line": 0, "column": 1 },
                        "end": { "line": 0, "column": 2 }
                    },
                    "kind": 2
                }
            ]
        });
        let result: HighlightResult = serde_json::from_value(data).unwrap();
        assert_eq!(result.highlights.len(), 1);
        assert!(result.highlights[0].tags.is_empty());
    }

    #[test]
    fn signature_help_params_serialize_trigger_char() {
        let params = SignatureHelpParams {
            uri: "file:///C:/a.t".to_string(),
            position: Position { line: 3, column: 4 },
            trigger_char: Some("(".to_string()),
        };
        let value = serde_json::to_value(params).unwrap();
        assert_eq!(value.get("triggerChar").unwrap(), &json!("("));
    }

    #[test]
    fn workspace_elements_parse_map() {
        let data = json!({
            "elements": {
                "file:///C:/a.t": [
                    {
                        "kind": 1,
                        "tags": [],
                        "name": "主窗口",
                        "detail": "类",
                        "range": {
                            "start": { "line": 0, "column": 0 },
                            "end": { "line": 10, "column": 0 }
                        },
                        "identifierRange": {
                            "start": { "line": 0, "column": 1 },
                            "end": { "line": 0, "column": 4 }
                        }
                    }
                ]
            }
        });
        let result: WorkspaceElementsResult = serde_json::from_value(data).unwrap();
        assert_eq!(result.elements.len(), 1);
        let items = result.elements.get("file:///C:/a.t").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "主窗口");
    }
}
