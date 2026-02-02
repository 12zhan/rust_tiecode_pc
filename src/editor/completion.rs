use gpui::*;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompletionKind {
    Function,
    Variable,
    Class,
    Keyword,
    Text,
}

impl CompletionKind {
    #[allow(dead_code)]
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
            Self::Function => rgb(0xfabd2f).into(), // Gruvbox Yellow
            Self::Variable => rgb(0xd3869b).into(), // Gruvbox Purple
            Self::Class => rgb(0x8ec07c).into(),    // Gruvbox Aqua
            Self::Keyword => rgb(0xfb4934).into(),  // Gruvbox Red
            Self::Text => rgb(0xebdbb2).into(),     // Gruvbox Foreground
        }
    }
}
