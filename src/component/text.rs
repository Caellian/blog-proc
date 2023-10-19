use std::{borrow::Cow, fmt::Write};

use super::{Component, PostEntry, Structured};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Style<'a> {
    #[default]
    None,
    Span,
    Paragraph,
    Heading(u8),
    Emphasis,
    Strong,
    Link(Cow<'a, str>),
    Code,
    Strikethrough,
    Highlight,
    Superscript,
    Subscript,
}

impl<'a> Style<'a> {
    pub fn tag(&self) -> &'static str {
        match self {
            Style::None => "span",
            Style::Span => "span",
            Style::Paragraph => "p",
            Style::Heading(level) => match level {
                1 => "h1",
                2 => "h2",
                3 => "h3",
                4 => "h4",
                5 => "h5",
                6 => "h6",
                _ => unimplemented!("only (1, 6) heading levels supported"),
            },
            Style::Emphasis => "em",
            Style::Strong => "strong",
            Style::Link(_) => "a",
            Style::Code => "code",
            Style::Strikethrough => "del",
            Style::Highlight => "mark",
            Style::Superscript => "sup",
            Style::Subscript => "sub",
        }
    }

    pub fn copy(&self) -> Option<Self> {
        match self {
            Style::Link(_) => None,
            _ => unsafe {
                // SAFETY: All variants except `Link` are copyable.
                let mut result = Self::default();
                std::ptr::copy_nonoverlapping(self, &mut result, 1);
                Some(result)
            },
        }
    }
}

impl From<pulldown_cmark::HeadingLevel> for Style<'_> {
    fn from(level: pulldown_cmark::HeadingLevel) -> Self {
        Style::Heading(match level {
            pulldown_cmark::HeadingLevel::H1 => 1,
            pulldown_cmark::HeadingLevel::H2 => 2,
            pulldown_cmark::HeadingLevel::H3 => 3,
            pulldown_cmark::HeadingLevel::H4 => 4,
            pulldown_cmark::HeadingLevel::H5 => 5,
            pulldown_cmark::HeadingLevel::H6 => 6,
        })
    }
}

#[derive(Debug, Default)]
pub enum SegmentContent<'a> {
    #[default]
    Empty,
    Text(Cow<'a, str>),
    Other(Box<dyn Component>),
}

#[derive(Debug, Default)]
pub struct TextSegment<'a> {
    pub style: Style<'a>,
    pub content: SegmentContent<'a>,
}

#[derive(Debug, Clone, Default)]
pub enum TextPart<'a> {
    #[default]
    Empty,
    NewLine,
    Raw(String),
    Chained(Vec<TextPart<'a>>),
    Nested(Box<TextComponent<'a>>),
}

impl<'a> TextPart<'a> {
    pub fn wrapper_ref(&self) -> Option<&Style> {
        match self {
            TextPart::Nested(text) => Some(&text.style),
            _ => None,
        }
    }

    pub fn wrapper(&self) -> Style {
        match self {
            TextPart::Nested(text) => text.style.clone(),
            _ => Style::None,
        }
    }

    pub fn children(&'a self) -> Vec<&'a TextPart> {
        match self {
            TextPart::Chained(items) => items.iter().collect(),
            TextPart::Nested(text) => {
                vec![&text.content]
            }
            _ => vec![],
        }
    }

    pub fn children_mut(&'a mut self) -> Vec<&'a mut TextPart> {
        match self {
            TextPart::Chained(items) => items.iter_mut().collect(),
            TextPart::Nested(text) => {
                vec![&mut text.content]
            }
            _ => vec![],
        }
    }

    pub fn take_inner(&mut self) -> TextPart {
        match self {
            TextPart::Chained(_) => std::mem::take(self),
            TextPart::Nested(text) => std::mem::take(&mut text.content),
            _ => TextPart::Empty,
        }
    }

    pub fn append(&mut self, child: Self) {
        match self {
            TextPart::Empty => {
                *self = child;
            }
            TextPart::Raw(_) | TextPart::NewLine => {
                let content = std::mem::take(self);
                *self = TextPart::Chained(vec![content, child]);
            }
            TextPart::Chained(items) => items.push(child),
            TextPart::Nested(component) => {
                component.content.append(child);
            }
        }
    }

    pub fn write_str(&mut self, value: impl AsRef<str>) -> Result<(), std::fmt::Error> {
        let value = value.as_ref();
        match self {
            TextPart::Empty => {
                *self = TextPart::Raw(value.to_string());
                Ok(())
            }
            TextPart::NewLine => {
                *self =
                    TextPart::Chained(vec![TextPart::NewLine, TextPart::Raw(value.to_string())]);
                Ok(())
            }
            TextPart::Raw(content) => content.write_str(value),
            TextPart::Chained(content) => {
                content.push(TextPart::Raw(value.to_string()));
                Ok(())
            }
            TextPart::Nested(text) => text.content.write_str(value),
        }
    }
}

impl<'s, S: ToString> From<S> for TextPart<'s> {
    fn from(value: S) -> Self {
        TextPart::Raw(value.to_string())
    }
}

impl<'s> From<TextComponent<'s>> for TextPart<'s> {
    fn from(value: TextComponent<'s>) -> Self {
        if value.style != Style::None {
            TextPart::Nested(Box::new(value))
        } else {
            value.content
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TextComponent<'a> {
    pub style: Style<'a>,
    pub content: TextPart<'a>,
}

impl<'a> TextComponent<'a> {
    pub const EMPTY: Self = TextComponent {
        style: Style::None,
        content: TextPart::Empty,
    };

    #[inline]
    pub fn new(content: impl ToString) -> Self {
        TextComponent {
            style: Style::None,
            content: TextPart::Raw(content.to_string()),
        }
    }

    #[inline]
    pub fn new_styled(style: Style<'a>) -> Self {
        TextComponent {
            style,
            content: TextPart::Empty,
        }
    }

    #[inline]
    pub fn new_link(target: impl ToString, content: impl ToString) -> Self {
        TextComponent {
            style: Style::Link(Cow::Owned(target.to_string())),
            content: TextPart::Raw(content.to_string()),
        }
    }

    pub fn new_chained<T: Into<TextPart<'a>>>(items: impl IntoIterator<Item = T>) -> Self {
        TextComponent {
            content: TextPart::Chained(items.into_iter().map(Into::into).collect()),
            ..Default::default()
        }
    }

    #[allow(private_bounds)]
    #[inline]
    pub fn push(&mut self, value: impl Into<TextValue<'a>>) {
        match value.into() {
            TextValue::Raw(string) => self
                .content
                .write_str(string)
                .expect("unable to push string"),
            TextValue::Segment(segment) => self.content.append(segment),
        }
    }
}

impl<'a> From<TextPart<'a>> for TextComponent<'a> {
    #[inline]
    fn from(value: TextPart<'a>) -> Self {
        match value {
            TextPart::Nested(inner) => *inner,
            content => TextComponent {
                content,
                ..Default::default()
            },
        }
    }
}

enum TextValue<'a> {
    Raw(Cow<'a, str>),
    Segment(TextPart<'a>),
}

impl From<String> for TextValue<'_> {
    #[inline]
    fn from(value: String) -> Self {
        TextValue::Raw(Cow::Owned(value))
    }
}

impl<'a> From<&'a str> for TextValue<'a> {
    #[inline]
    fn from(value: &'a str) -> Self {
        TextValue::Raw(Cow::Borrowed(value))
    }
}

impl<'a> From<TextPart<'a>> for TextValue<'a> {
    #[inline]
    fn from(value: TextPart<'a>) -> Self {
        TextValue::Segment(value)
    }
}

impl<'a> From<TextComponent<'a>> for TextValue<'a> {
    #[inline]
    fn from(value: TextComponent<'a>) -> Self {
        TextValue::Segment(TextPart::Nested(Box::new(value)))
    }
}
