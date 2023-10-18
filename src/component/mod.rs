use std::{
    borrow::Cow,
    default,
    fmt::{Debug, Write},
};

use ::render::{html, rsx, Render};
use strum::EnumDiscriminants;

use crate::util::random_id;

pub mod text;
pub use text::*;

pub mod tex;
pub use tex::LatexComponent;

pub mod parser;
pub use parser::ComponentParser as Parser;

#[derive(Debug, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

impl From<pulldown_cmark::Alignment> for Alignment {
    fn from(alignment: pulldown_cmark::Alignment) -> Self {
        match alignment {
            pulldown_cmark::Alignment::None => Alignment::None,
            pulldown_cmark::Alignment::Left => Alignment::Left,
            pulldown_cmark::Alignment::Center => Alignment::Center,
            pulldown_cmark::Alignment::Right => Alignment::Right,
        }
    }
}

#[derive(Debug)]
pub struct TableComponent<'a> {
    headers: Vec<PostComponent<'a>>,
    alignment: Vec<Alignment>,
    rows: Vec<Vec<PostComponent<'a>>>,
}

#[derive(Debug)]
pub struct ListComponent<'a> {
    pub numbered: Option<usize>,
    pub items: Vec<PostComponent<'a>>,
}

#[derive(Debug, Default)]
pub enum Structured<T> {
    #[default]
    Empty,
    Value(T),
    Sequence(Vec<Self>),
}

impl<T: Component> Component for Structured<T> {
    fn render(&self, target: &mut String) -> std::fmt::Result {
        match self {
            Structured::Empty => Ok(()),
            Structured::Value(it) => it.render(target),
            Structured::Sequence(values) => {
                for value in values {
                    value.render(target)?;
                }
                Ok(())
            }
        }
    }
}

impl<T> Container<T> for Structured<T> {
    fn push(&mut self, component: T) {
        match self {
            Structured::Empty => *self = Structured::Value(component),
            Structured::Sequence(values) => values.push(Structured::Value(component)),
            _ => {
                *self =
                    Structured::Sequence(vec![std::mem::take(self), Structured::Value(component)]);
            }
        }
    }
}

pub trait Component: Debug {
    fn render(&self, target: &mut String) -> std::fmt::Result;
}

impl ToString for dyn Component {
    fn to_string(&self) -> String {
        let mut result = String::with_capacity(64);
        self.render(&mut result);
        result
    }
}

pub trait Container<T> {
    fn push(&mut self, component: T);
}

pub type PostEntry = Structured<Box<dyn Component>>;

#[derive(Debug, Default, EnumDiscriminants)]
#[strum_discriminants(derive(strum::Display))]
#[strum_discriminants(name(PostComponentKind))]
pub enum PostComponent<'a> {
    #[default]
    Placeholder,
    Text(TextComponent<'a>),
    BlockQuote(Vec<PostComponent<'a>>),
    Image {
        source: String,
        alt: Option<String>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    List(ListComponent<'a>),
    HorizonalRule,
    Table(TableComponent<'a>),
    Footnote {
        id: String,
        text: TextComponent<'a>,
    },
    Latex(LatexComponent<'a>),
    Chained(Vec<Self>),
    Raw(String),
}

impl<'a> PostComponent<'a> {
    pub const BLANK: PostComponent<'static> = PostComponent::Raw(String::new());

    pub fn discriminant(&self) -> PostComponentKind {
        PostComponentKind::from(self)
    }

    pub fn push(&mut self, other: PostComponent<'a>) {
        match (self, other) {
            (current, other) if current.discriminant() == PostComponentKind::Placeholder => {
                *current = other;
            }
            (
                PostComponent::BlockQuote(items) | PostComponent::List(ListComponent { items, .. }),
                other,
            ) => items.push(other),
            (PostComponent::Text(text_component), PostComponent::Text(other)) => {
                text_component.push(other)
            }
            (PostComponent::Text(text_component), PostComponent::Raw(raw)) => {
                text_component.push(raw)
            }
            (
                PostComponent::Footnote {
                    text: text_component,
                    ..
                },
                PostComponent::Text(other),
            ) => text_component.push(other),
            (
                PostComponent::Footnote {
                    text: text_component,
                    ..
                },
                PostComponent::Raw(raw),
            ) => text_component.push(raw),
            (current, added) => {
                let prev = std::mem::take(current);
                *current = PostComponent::Chained(vec![prev, added]);
            }
        }
    }

    pub fn push_text(&mut self, text: impl ToString) -> bool {
        match self {
            PostComponent::Placeholder => {
                *self = PostComponent::Text(TextComponent::new(text));
            }
            PostComponent::Text(component) => component.push(text.to_string()),
            PostComponent::BlockQuote(quote) => match quote.last_mut() {
                Some(PostComponent::Text(text_component)) => text_component.push(text.to_string()),
                _ => quote.push(PostComponent::from(TextPart::from(text))),
            },
            PostComponent::CodeBlock { content, .. } => content
                .write_str(text.to_string().as_str())
                .expect("unable to write text to CodeBlock"),
            PostComponent::List(ListComponent { items, .. }) => match items.last_mut() {
                Some(last) => return last.push_text(text),
                None => items.push(PostComponent::from(TextPart::from(text))),
            },
            PostComponent::Footnote {
                text: text_component,
                ..
            } => text_component.push(text.to_string()),
            PostComponent::Latex(tex) => match &mut tex.source {
                Cow::Owned(it) => {
                    it.write_str(text.to_string().as_str())
                        .expect("unable to append text to LaTeX source");
                }
                Cow::Borrowed(current) => {
                    tex.source = Cow::Owned(current.to_string() + text.to_string().as_str())
                }
            },
            PostComponent::Chained(items) => match items.last_mut() {
                Some(last) if last.discriminant() == PostComponentKind::Text => {
                    last.push_text(text);
                }
                _ => items.push(PostComponent::Text(TextComponent::new(text))),
            },
            _ => return false,
        }

        true
    }
}

impl<'a> From<TextPart<'a>> for PostComponent<'a> {
    #[inline]
    fn from(value: TextPart<'a>) -> Self {
        PostComponent::Text(TextComponent::from(value))
    }
}

pub mod render;
pub use render::*;
