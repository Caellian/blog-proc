use std::borrow::Cow;

#[derive(Debug, Default)]
pub enum Format {
    Inline,
    #[default]
    Multiline,
}

#[derive(Debug)]
pub struct LatexRenderInfo {}

#[derive(Debug, Default)]
pub struct LatexComponent<'a> {
    pub format: Format,
    pub source: Cow<'a, str>,
    pub rendered: Option<LatexRenderInfo>,
}
