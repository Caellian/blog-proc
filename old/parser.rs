use crate::util::WriteEvent;
use crate::{
    blog::{Field, PostInfo},
    error::{BlogError, FormatError},
};
use async_lock::{Mutex, RwLock, RwLockReadGuard};
use chrono::DateTime;
use futures::{
    pin_mut,
    stream::{self, StreamExt},
    Stream,
};
use nym::transform;
use pulldown_cmark::{CodeBlockKind, CowStr, Event as MDEvent, Parser, Tag};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HTML(pub String);

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum TransformerError {}

#[derive(Debug)]
#[repr(u8)]
enum TransformerState<'a> {
    Passthrough,
    InQuote {
        content: Mutex<String>,
    },
    InCode {
        lang: Box<CowStr<'a>>,
        content: Mutex<String>,
    },
    Error(TransformerError),
}

impl<'a> Default for TransformerState<'a> {
    fn default() -> Self {
        TransformerState::Passthrough
    }
}

type StatePointer<'a> = Arc<RwLock<TransformerState<'a>>>;

pub async fn process_md_event<'a>(
    state: StatePointer<'a>,
    ev: MDEvent<'a>,
) -> Result<Option<MDEvent<'a>>, TransformerError> {
    let mut new_state = None;

    let result = match &*(state.read().await) {
        TransformerState::Passthrough => match ev {
            MDEvent::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                new_state = Some(TransformerState::InQuote {
                    content: Mutex::new(String::with_capacity(512)),
                });
                Ok(None)
            }
            MDEvent::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(lang))) => {
                new_state = Some(TransformerState::InCode {
                    lang: Box::new(lang),
                    content: Mutex::new(String::with_capacity(1024)),
                });
                Ok(None)
            }
            ev => Ok(Some(ev)),
        },
        TransformerState::InQuote { content } => match ev {
            MDEvent::End(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                let mut c = content.lock().await;
                c.insert_str(0, "<p class=\"quote\">");
                c.push_str("</p>");

                new_state = Some(TransformerState::Passthrough);
                Ok(Some(MDEvent::Html(c.to_owned().into())))
            }
            it => {
                let mut c = content.lock().await;
                (*c).write_event(&it).unwrap();
                Ok(None)
            }
        },
        TransformerState::InCode { lang, content } => match ev {
            MDEvent::End(Tag::CodeBlock(CodeBlockKind::Fenced(end_lang))) => {
                if *end_lang == ***lang {
                    let mut c = content.lock().await;
                    c.insert_str(
                        0,
                        &("<pre><code class=\"language-".to_owned() + lang.as_ref() + "\">\n"),
                    );
                    c.push_str("\n</code></pre>");

                    new_state = Some(TransformerState::Passthrough);

                    Ok(Some(MDEvent::Html(c.to_owned().into())))
                } else {
                    Ok(None)
                }
            }
            it => {
                let mut c = content.lock().await;
                c.write_event(&it).unwrap();
                Ok(None)
            }
        },
        TransformerState::Error(err) => Err((*err).clone()),
    };
    if let Err(err) = result {
        return Err(err);
    }
    if let Some(s) = new_state {
        let mut state = state.write().await;
        *state = s;
    }
    result
}

pub struct StreamTransformer<'a> {
    state: StatePointer<'a>,
}

impl<'a> StreamTransformer<'a> {
    pub fn new() -> StreamTransformer<'a> {
        StreamTransformer {
            state: Arc::new(RwLock::new(TransformerState::Passthrough)),
        }
    }

    pub async fn process_stream(
        self,
        ev_stream: impl Stream<Item = MDEvent<'a>> + 'a,
    ) -> impl Stream<Item = MDEvent<'a>> {
        let state = self.state;

        async_stream::stream! {
            pin_mut!(ev_stream);

            while let Some(event) = ev_stream.next().await {
                let proc_result = process_md_event(state.clone(), event).await;
                if let Ok(Some(result)) = proc_result {
                    yield result;
                } else if let Err(err) = proc_result {
                    break;
                }
            }
        }
    }
}

async fn push_markdown_as_html<'a, S>(s: &mut String, stream: S)
where
    S: Stream<Item = MDEvent<'a>> + 'a,
{
    let transformer: StreamTransformer<'a> = StreamTransformer::new();
    let transformed = transformer.process_stream(stream).await;

    pin_mut!(transformed);
    while let Some(ev) = transformed.next().await {
        pulldown_cmark::html::push_html(s, std::iter::once(ev))
    }
}

pub(crate) async fn process_markdown(markdown: &str) -> Result<HTML, FormatError> {
    let parser = stream::iter(pulldown_cmark::Parser::new(markdown));

    let mut result = String::with_capacity(1024);
    push_markdown_as_html(&mut result, parser).await;
    Ok(HTML(result))
}

pub(crate) async fn read_post(content: &str) -> Result<(PostInfo, HTML), FormatError> {
    let split: Vec<&str> = content.split("---").map(|s| s.trim()).collect();

    let (info, md) = match split.len() {
        1 => (None, content),
        2 => {
            if let Ok(info) = PostInfo::from_str(split[0]) {
                (Some(info), split[1])
            } else {
                (None, content)
            }
        }
        _ => split
            .iter()
            .enumerate()
            .skip_while(|(_, it)| it.len() == 0)
            .find_map(|(i, source)| {
                PostInfo::from_str(source).ok().map(|info| {
                    (
                        Some(info),
                        content
                            .split_at(
                                split
                                    .iter()
                                    .enumerate()
                                    .take_while(|(j, _)| *j <= i)
                                    .map(|(_, el)| el.len())
                                    .sum(),
                            )
                            .1,
                    )
                })
            })
            .unwrap_or((None, content)),
        0 => unreachable!(),
    };

    Ok((info.unwrap_or_default(), process_markdown(md).await?))
}
