use std::{borrow::Cow, collections::VecDeque};

use pulldown_cmark::{
    CowStr, Event as CMEvent, EventData as CMEventData, InlineStr, Span, Tag as CMTag,
    TagEnd as CMTagEnd,
};
use render::html;
use strum::EnumDiscriminants;

use crate::component::{
    text::{Style, TextComponent, TextPart},
    ListComponent, PostComponent, PostComponentKind,
};

use super::{LatexComponent, TableComponent};

#[derive(Debug, Clone, Copy, PartialEq, EnumDiscriminants)]
#[strum_discriminants(name(TagEnd))]
pub enum Tag {
    /// Expression wrapped in `==equal signs==` is <mark>highlighted</mark>.
    Highlight,
    /// LaTeX math mode.
    ///
    /// If parameter is `true`, it's `$$multiline context$$`, otherwise it's
    /// `$inline$`.
    Tex(bool),
}

impl Tag {
    const ALL: [Tag; 3] = [Tag::Highlight, Tag::Tex(false), Tag::Tex(true)];

    pub fn delimiter(&self) -> &'static str {
        match self {
            Tag::Highlight => "==",
            Tag::Tex(multiline) => {
                if *multiline {
                    "$$"
                } else {
                    "$"
                }
            }
        }
    }

    pub fn is_multiline(&self) -> bool {
        match self {
            Tag::Tex(true) => true,
            _ => false,
        }
    }
}

fn find_wrapped(text: &str, wrapper: &str) -> Option<(usize, usize)> {
    if let Some(pos) = text.find(wrapper) {
        let start = pos + wrapper.len();
        if let Some(end) = text[start..].rfind(wrapper) {
            if end == 0 {
                // empty content which means repeated delimiter so this must be
                // None because there's no difference between $$,... and $,$,...
                // otherwise
                return None;
            }
            Some((pos, start + end + wrapper.len()))
        } else {
            None
        }
    } else {
        None
    }
}

pub enum Event<'a> {
    CommonMark(CMEvent<'a>),
    Start(Tag),
    End(TagEnd),
}

struct MultilineDelimiterCtx {
    delimiter: &'static str,
    start: usize,
    closing_tag: TagEnd,
}

/// Markdown parser that extends `pulldown_cmark` one with tags/elements that
/// aren't part of CommonMark specification.
pub struct MarkdownParser<'input> {
    source: &'input str,
    inner: pulldown_cmark::Parser<'input, 'input>,
    remainder: VecDeque<Event<'input>>,
    multiline: Option<MultilineDelimiterCtx>,
}

impl<'input> MarkdownParser<'input> {
    pub fn new(source: &'input str) -> Self {
        MarkdownParser {
            source,
            inner: pulldown_cmark::Parser::new_ext(source, pulldown_cmark::Options::all()),
            remainder: VecDeque::with_capacity(4),
            multiline: None,
        }
    }
}

trait SubstrCowStr<'a> {
    fn slice<R>(&self, range: R) -> Self
    where
        str: std::ops::Index<R, Output = str>;
}
impl<'a> SubstrCowStr<'a> for CowStr<'a> {
    fn slice<R>(&self, range: R) -> Self
    where
        str: std::ops::Index<R, Output = str>,
    {
        match self.clone() {
            CowStr::Boxed(boxed) => CowStr::Boxed(boxed[range].to_string().into_boxed_str()),
            CowStr::Borrowed(borrow) => CowStr::Borrowed(&borrow[range]),
            CowStr::Inlined(inlined) => {
                CowStr::Inlined(InlineStr::try_from(&inlined[range]).unwrap())
            }
        }
    }
}

impl<'input> Iterator for MarkdownParser<'input> {
    type Item = Event<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        Some('result: loop {
            let current: Event<'input> = if !self.remainder.is_empty() {
                self.remainder.pop_front().unwrap()
            } else {
                Event::CommonMark(self.inner.next()?)
            };

            break match current {
                Event::CommonMark(cm) => match (&*cm, &mut self.multiline) {
                    (CMEventData::Text(text), None) => {
                        for t in Tag::ALL {
                            let delimiter = t.delimiter();
                            let len = delimiter.len();
                            if t.is_multiline() {
                                if let Some(start) = text.as_ref().find(delimiter) {
                                    self.multiline = Some(MultilineDelimiterCtx {
                                        delimiter,
                                        start: cm.span().start() + start + len,
                                        closing_tag: TagEnd::from(t),
                                    });
                                    self.remainder.push_back(Event::Start(t));
                                    break 'result Event::CommonMark(CMEvent::new(
                                        CMEventData::Text(text.slice(..start)),
                                        cm.span().cap_length(start),
                                    ));
                                } else {
                                    break 'result Event::CommonMark(cm);
                                }
                            } else if let Some((start, end)) =
                                find_wrapped(text.as_ref(), delimiter)
                            {
                                self.remainder.extend([
                                    Event::Start(t),
                                    Event::CommonMark(CMEvent::new(
                                        CMEventData::Text(text.slice((start + len)..(end - len))),
                                        cm.span()
                                            .offset_start((start + len) as isize)
                                            .cap_length(end - start - len * 2),
                                    )),
                                    Event::End(TagEnd::from(t)),
                                    Event::CommonMark(CMEvent::new(
                                        CMEventData::Text(text.slice(end..)),
                                        cm.span().offset_start(end as isize),
                                    )),
                                ]);
                                break 'result Event::CommonMark(CMEvent::new(
                                    CMEventData::Text(text.slice(..start)),
                                    cm.span().cap_length(start),
                                ));
                            }
                        }

                        Event::CommonMark(cm)
                    }
                    (_, multiline) => match multiline {
                        Some(multiline) => {
                            let text = &self.source[cm.span()];
                            if let Some(end) = text.find(multiline.delimiter) {
                                let len = multiline.delimiter.len();
                                self.remainder.extend([
                                    Event::End(multiline.closing_tag),
                                    Event::CommonMark(CMEvent::new(
                                        CMEventData::Text(CowStr::Borrowed(&text[(end + len)..])),
                                        cm.span().offset_start((end + len) as isize),
                                    )),
                                ]);
                                let span = Span::from(multiline.start..(cm.span().start() + end));
                                self.multiline = None;
                                Event::CommonMark(CMEvent::new(
                                    CMEventData::Text(CowStr::Borrowed(&self.source[span])),
                                    span,
                                ))
                            } else {
                                continue;
                            }
                        }
                        None => Event::CommonMark(cm),
                    },
                },
                other => other,
            };
        })
    }
}

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
    inner: MarkdownParser<'input>,
    options: ParserOptions,
    stack: Vec<PostComponent<'input>>,
    stage: ParseStage<'input>,
}

impl<'input> ComponentParser<'input> {
    pub fn new(source: &'input str) -> Self {
        ComponentParser {
            inner: MarkdownParser::new(source),
            options: ParserOptions::default(),
            stack: Vec::with_capacity(8),
            stage: ParseStage::None,
        }
    }

    #[inline]
    fn push_start(&mut self, tag: Tag) {
        match tag {
            Tag::Highlight => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Highlight,
                ..Default::default()
            })),
            Tag::Tex(multiline) => self.stack.push(PostComponent::Latex(LatexComponent {
                format: if multiline {
                    super::tex::Format::Multiline
                } else {
                    super::tex::Format::Inline
                },
                source: Cow::Owned(String::new()),
                ..Default::default()
            })),
        }
    }

    #[inline]
    fn push_end(&mut self, tag: TagEnd) -> Option<PostComponent<'input>> {
        match tag {
            _ => self.stack.pop(),
        }
    }

    #[inline]
    fn push_cm_start(&mut self, tag: CMTag<'input>) {
        match tag {
            CMTag::MetadataBlock(_kind) => unimplemented!(),
            CMTag::Paragraph => self
                .stack
                .push(PostComponent::Text(TextComponent::new_styled(
                    Style::Paragraph,
                ))),
            CMTag::Heading { level, .. } => {
                self.stack
                    .push(PostComponent::Text(TextComponent::new_styled(Style::from(
                        level,
                    ))))
            }
            CMTag::BlockQuote => self.stack.push(PostComponent::BlockQuote(vec![])),
            CMTag::CodeBlock(kind) => self.stack.push(PostComponent::CodeBlock {
                language: match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                    pulldown_cmark::CodeBlockKind::Indented => None,
                },
                content: String::with_capacity(256),
            }),
            CMTag::List(numbered) => self.stack.push(PostComponent::List(ListComponent {
                numbered: numbered.map(|it| it as usize),
                items: Vec::with_capacity(4),
            })),
            CMTag::Item => self.stack.push(PostComponent::Placeholder),
            CMTag::FootnoteDefinition(label) => self.stack.push(PostComponent::Footnote {
                id: label.to_string(),
                text: TextComponent::EMPTY,
            }),
            CMTag::Table(alignment) => self.stack.push(PostComponent::Table(TableComponent {
                headers: vec![],
                alignment: alignment.into_iter().map(Into::into).collect(),
                rows: vec![],
            })),
            CMTag::TableHead => {
                self.stage = ParseStage::Table(TableParseStage {
                    header: true,
                    row: vec![],
                })
            }
            CMTag::TableRow => {
                // row storage is initialized with CMTag::TableHead
                // and cleared in CMTag::TableRow event with default (empty Vec)
                // so, do nothing
            }
            CMTag::TableCell => self.stack.push(PostComponent::Placeholder),
            CMTag::Emphasis => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Emphasis,
                ..Default::default()
            })),
            CMTag::Strong => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Strong,
                ..Default::default()
            })),
            CMTag::Strikethrough => self.stack.push(PostComponent::Text(TextComponent {
                style: Style::Strikethrough,
                ..Default::default()
            })),
            // TODO: Handle link types
            CMTag::Link {
                dest_url, title, ..
            } => self.stack.push(PostComponent::Text(TextComponent::new_link(
                dest_url, title,
            ))),
            CMTag::Image {
                dest_url, title, ..
            } => self.stack.push(PostComponent::Image {
                source: dest_url.to_string(),
                alt: if !title.is_empty() {
                    Some(title.to_string())
                } else {
                    None
                },
            }),
            CMTag::HtmlBlock => todo!(),
        }
    }

    #[inline]
    fn push_cm_end(&mut self, tag: CMTagEnd) -> Option<PostComponent<'input>> {
        /*
            let last = match self.stack.last_mut() {
                Some(it) => it,
                None => panic!("can't close unopened tag: {:?}", tag),
            };
        */

        match (tag, &mut self.stage) {
            (CMTagEnd::Item, _)
                if self
                    .stack
                    .last()
                    .map(|it| it.discriminant() == PostComponentKind::Placeholder)
                    .unwrap_or_default() =>
            {
                self.stack.pop();
                Some(PostComponent::BLANK)
            }
            (CMTagEnd::TableHead, ParseStage::Table(stage)) => {
                let table = match self.stack.last_mut() {
                    Some(PostComponent::Table(it)) => it,
                    _ => panic!("expected table on stack"),
                };
                std::mem::swap(&mut table.headers, &mut stage.row);
                stage.header = false;
                None
            }
            (CMTagEnd::TableRow, ParseStage::Table(stage)) => {
                let table = match self.stack.last_mut() {
                    Some(PostComponent::Table(it)) => it,
                    _ => panic!("expected table on stack"),
                };
                table.rows.push(std::mem::take(&mut stage.row));
                None
            }
            (CMTagEnd::TableCell, ParseStage::Table(stage)) => {
                stage.row.push(match self.stack.pop() {
                    Some(PostComponent::Placeholder) => PostComponent::BLANK,
                    Some(it) => it,
                    None => {
                        unreachable!("can't end table cell with no components on the stack")
                    }
                });
                None
            }
            (CMTagEnd::Table, stage) => {
                if let ParseStage::Table(_) = stage {
                    *stage = ParseStage::None;
                    self.stack.pop()
                } else {
                    panic!("expected a table parse stage when closing a table")
                }
            }
            (CMTagEnd::TableHead | CMTagEnd::TableRow | CMTagEnd::TableCell, _) => {
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
                Event::CommonMark(event) => match &*event {
                    CMEventData::Start(tag) => {
                        self.push_cm_start(tag.clone());
                        None
                    }
                    CMEventData::End(tag) => self.push_cm_end(tag.clone()),
                    CMEventData::Text(value) => {
                        self.push_text(value);
                        None
                    }
                    CMEventData::Code(value) => {
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
                    CMEventData::Html(raw) => Some(PostComponent::Raw(raw.to_string())),
                    CMEventData::InlineHtml(raw) => Some(PostComponent::Raw(raw.to_string())),
                    CMEventData::FootnoteReference(label) => {
                        Some(PostComponent::Text(TextComponent {
                            style: Style::Link(Cow::Owned(
                                "#footnote-".to_string() + label.as_ref(),
                            )),
                            content: TextPart::Nested(Box::new(TextComponent {
                                style: Style::Superscript,
                                content: TextPart::Raw(format!("[{}]", label.as_ref())),
                            })),
                        }))
                    }
                    CMEventData::SoftBreak if self.options.newline_soft_break => {
                        Some(PostComponent::from(TextPart::NewLine))
                    }
                    CMEventData::SoftBreak => {
                        self.push_text(" ");
                        None
                    }
                    CMEventData::HardBreak if self.options.newline_soft_break => {
                        Some(PostComponent::from(TextPart::Chained(vec![
                            TextPart::NewLine,
                            TextPart::NewLine,
                        ])))
                    }
                    CMEventData::HardBreak => Some(PostComponent::from(TextPart::NewLine)),
                    CMEventData::Rule => Some(PostComponent::HorizonalRule),
                    CMEventData::TaskListMarker(is_checked) => Some(if *is_checked {
                        PostComponent::Raw(html! {
                            <input type={"checkbox"} disabled={true} checked={} />
                        })
                    } else {
                        PostComponent::Raw(html! {
                            <input type={"checkbox"} disabled={true} />
                        })
                    }),
                },
                Event::Start(tag) => {
                    self.push_start(tag);
                    None
                }
                Event::End(tag) => self.push_end(tag),
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
