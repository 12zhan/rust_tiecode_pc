use gpui::*;

#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompletionKind {
    Function,
    Variable,
    Class,
    Keyword,
    Text,
}

impl CompletionKind {
    pub fn icon_text(&self) -> &'static str {
        match self {
            Self::Function => "F",
            Self::Variable => "V",
            Self::Class => "T",
            Self::Keyword => "K",
            Self::Text => "abc",
        }
    }

    pub fn color(&self) -> Hsla {
        match self {
            Self::Function => rgb(0xdcb628).into(), // Yellow
            Self::Variable => rgb(0xd02a8c).into(), // Magenta
            Self::Class => rgb(0xaaaaaa).into(),    // Gray
            Self::Keyword => rgb(0x569cd6).into(),  // Blue
            Self::Text => rgb(0xcccccc).into(),     // Light Gray
        }
    }
}

pub const CPP_KEYWORDS: &[&str] = &[
    "int", "char", "float", "double", "bool", "void", "long", "short", "signed", "unsigned",
    "if", "else", "for", "while", "do", "switch", "case", "default", "break", "continue", "return", "goto",
    "struct", "class", "enum", "union", "typedef", "typename", "template", "namespace", "using",
    "public", "private", "protected", "virtual", "override", "static", "const", "inline", "friend",
    "true", "false", "nullptr", "this", "new", "delete", "sizeof", "operator", "explicit", "noexcept",
    "#include", "#define", "#ifdef", "#ifndef", "#endif", "#pragma"
];
