use std::{borrow::Cow, collections::VecDeque};

use pulldown_cmark::{CowStr, Event, InlineStr, Parser, Tag, TagEnd};

use crate::component::{
    text::{Style, TextComponent, TextPart},
    ListComponent, PostComponent, PostComponentKind,
};

use super::TableComponent;

#[derive(Debug, Default)]
pub struct ParserOptions {
    /// If true soft breaks produce newlines (`<br/>`) and hard breaks double
    /// newlines (`<br/><br/>`).
    pub newline_soft_break: bool,
}

struct TableParseStage<'a> {
    header: bool,
    row: Vec<PostComponent<'a>>,
}

enum ParseStage<'a> {
    None,
    Table(TableParseStage<'a>),
}

pub struct ComponentParser<'input> {
    inner: Parser<'input, 'input>,
    options: ParserOptions,
    stack: Vec<PostComponent<'input>>,
    stage: ParseStage<'input>,
}

impl<'input> ComponentParser<'input> {
    pub fn new(source: &'input str) -> Self {
        ComponentParser {
            inner: Parser::new(source),
            options: ParserOptions::default(),
            stack: Vec::with_capacity(8),
            stage: ParseStage::None,
        }
    }

    #[inline]
    fn push_cm_start(&mut self, tag: Tag<'input>) {
        match tag {
            Tag::MetadataBlock(_kind) => unimplemented!(),
            Tag::Paragraph => self
                .stack
                .push(PostComponent::Text(TextComponent::new_styled(
                    Style::Paragraph,
                ))),
            Tag::Heading { level, .. } => {
                self.stack
                    .push(PostComponent::Text(TextComponent::new_styled(Style::from(
                        level,
                    ))))
            }
            Tag::BlockQuote => self.stack.push(PostComponent::BlockQuote(vec![])),
            Tag::CodeBlock(kind) => self.stack.push(PostComponent::CodeBlock {
                language: match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                    pulldown_cmark::CodeBlockKind::Indented => None,
                },
                content: String::with_capacity(256),
            }),
            Tag::List(numbered) => self.stack.push(PostComponent::List(ListComponent {
                numbered: numbered.map(|it| it as usize),
                items: Vec::with_capacity(4),
            })),
            Tag::Item => self.stack.push(PostComponent::Placeholder),
            Tag::FootnoteDefinition(label) => self.stack.push(PostComponent::Footnote {
                id: label.to_string(),
                text: TextComponent::EMPTY,
            }),
            Tag::Table(alignment) => self.stack.push(PostComponent::Table(TableComponent {
                headers: vec![],
                alignment: alignment.into_iter().map(Into::into).collect(),
                rows: vec![],
            })),
            Tag::TableHead => {
                self.stage = ParseStage::Table(TableParseStage {
                    header: true,
                    row: vec![],
                })
            }
            Tag::TableRow => {
                // row storage is initialized with Tag::TableHead
                // and cleared in Tag::TableRow event with default (empty Vec)
                // so, do nothing
            }
            Tag::TableCell => self.stack.push(PostComponent::Placeholder),
            Tag::Emphasis => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Emphasis,
                ..Default::default()
            })),
            Tag::Strong => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Strong,
                ..Default::default()
            })),
            Tag::Strikethrough => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Strikethrough,
                ..Default::default()
            })),
            // TODO: Handle link types
            Tag::Link {
                dest_url, title, ..
            } => self.stack.push(PostComponent::Text(TextComponent::new_link(
                dest_url, title,
            ))),
            Tag::Image {
                dest_url, title, ..
            } => self.stack.push(PostComponent::Image {
                source: dest_url.to_string(),
                alt: if !title.is_empty() {
                    Some(title.to_string())
                } else {
                    None
                },
            }),
            Tag::HtmlBlock => todo!(),
        }
    }

    #[inline]
    fn push_cm_end(&mut self, tag: TagEnd) -> Option<PostComponent<'input>> {
        /*
            let last = match self.stack.last_mut() {
                Some(it) => it,
                None => panic!("can't close unopened tag: {:?}", tag),
            };
        */

        match (tag, &mut self.stage) {
            (TagEnd::Item, _)
                if self
                    .stack
                    .last()
                    .map(|it| it.discriminant() == PostComponentKind::Placeholder)
                    .unwrap_or_default() =>
            {
                self.stack.pop();
                Some(PostComponent::BLANK)
            }
            (TagEnd::TableHead, ParseStage::Table(stage)) => {
                let table = match self.stack.last_mut() {
                    Some(PostComponent::Table(it)) => it,
                    _ => panic!("expected table on stack"),
                };
                std::mem::swap(&mut table.headers, &mut stage.row);
                stage.header = false;
                None
            }
            (TagEnd::TableRow, ParseStage::Table(stage)) => {
                let table = match self.stack.last_mut() {
                    Some(PostComponent::Table(it)) => it,
                    _ => panic!("expected table on stack"),
                };
                table.rows.push(std::mem::take(&mut stage.row));
                None
            }
            (TagEnd::TableCell, ParseStage::Table(stage)) => {
                stage.row.push(match self.stack.pop() {
                    Some(PostComponent::Placeholder) => PostComponent::BLANK,
                    Some(it) => it,
                    None => {
                        unreachable!("can't end table cell with no components on the stack")
                    }
                });
                None
            }
            (TagEnd::Table, stage) => {
                if let ParseStage::Table(_) = stage {
                    *stage = ParseStage::None;
                    self.stack.pop()
                } else {
                    panic!("expected a table parse stage when closing a table")
                }
            }
            (TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell, _) => {
                panic!("expected a table parse stage during table element tags");
            }
            _ => self.stack.pop(),
        }
    }

    #[inline]
    fn push_text(&mut self, value: impl ToString) {
        let last = match self.stack.last_mut() {
            Some(it) => it,
            None => unimplemented!("dangling text content"),
        };
        if !last.push_text(value.to_string()) {
            let prev = std::mem::take(last);
            *last =
                PostComponent::Chained(vec![prev, PostComponent::Text(TextComponent::new(value))]);
        }
    }
}

impl<'input> Iterator for ComponentParser<'input> {
    type Item = PostComponent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(loop {
            let current = self.inner.next()?;
            let result = match current {
                Event::Start(tag) => {
                    self.push_cm_start(tag.clone());
                    None
                }
                Event::End(tag) => self.push_cm_end(tag.clone()),
                Event::Text(value) => {
                    self.push_text(value);
                    None
                }
                Event::Code(value) => {
                    Some(PostComponent::Text(TextComponent::new_chained([
                        TextPart::Nested(Box::new(TextComponent {
                            style: Style::Code,
                            content: TextPart::Raw(value.to_string()),
                        })),
                        // Terminates inline code block to prevent text after it
                        // from being appended to it
                        TextPart::Empty,
                    ])))
                }
                // TODO: Handle newlined HTML differently?
                Event::Html(raw) => Some(PostComponent::Raw(raw.to_string())),
                Event::InlineHtml(raw) => Some(PostComponent::Raw(raw.to_string())),
                Event::FootnoteReference(label) => Some(PostComponent::Text(TextComponent {
                    style: Style::Link(Cow::Owned("#footnote-".to_string() + label.as_ref())),
                    content: TextPart::Nested(Box::new(TextComponent {
                        style: Style::Superscript,
                        content: TextPart::Raw(format!("[{}]", label.as_ref())),
                    })),
                })),
                Event::SoftBreak if self.options.newline_soft_break => {
                    Some(PostComponent::from(TextPart::NewLine))
                }
                Event::SoftBreak => {
                    self.push_text(" ");
                    None
                }
                Event::HardBreak if self.options.newline_soft_break => {
                    Some(PostComponent::from(TextPart::Chained(vec![
                        TextPart::NewLine,
                        TextPart::NewLine,
                    ])))
                }
                Event::HardBreak => Some(PostComponent::from(TextPart::NewLine)),
                Event::Rule => Some(PostComponent::HorizonalRule),
                Event::TaskListMarker(is_checked) => Some(if is_checked {
                    PostComponent::Raw(
                        "<input type=\"checkbox\" disabled=\"true\" checked />".to_string(),
                    )
                } else {
                    PostComponent::Raw("<input type=\"checkbox\" disabled=\"true\" />".to_string())
                }),
            };

            let result = match result {
                Some(it) => it,
                None => continue,
            };

            if let Some(last) = self.stack.last_mut() {
                last.push(result)
            } else {
                break result;
            }
        })
    }
}
