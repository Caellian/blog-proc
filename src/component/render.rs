use std::fmt::Result;

use super::*;

impl<'a> Style<'a> {
    pub fn render_content<W: Write>(
        &self,
        content: impl Render,
        writer: &mut W,
    ) -> std::fmt::Result {
        match self {
            Style::None => content.render_into(writer),
            Style::Link(target) => {
                write!(writer, "<a href=\"{}\">", target)?;
                content.render_into(writer)?;
                writer.write_str("</a>")
            }
            _ => {
                let tag = self.tag();
                write!(writer, "<{}>", tag)?;
                content.render_into(writer)?;
                write!(writer, "</{}>", tag)
            }
        }
    }
}

impl<'a> Component for TextPart<'a> {
    fn render(&self, target: &mut String) -> Result {
        match self {
            TextPart::Empty => Ok(()),
            TextPart::NewLine => target.write_str("<br/>"),
            TextPart::Raw(content) => target.write_str(content.as_str()),
            TextPart::Chained(content) => {
                for entry in content {
                    entry.render(target)?;
                }
                Ok(())
            }
            TextPart::Nested(inner) => inner.render_into(writer),
        }
    }
}

impl<'a> render::Render for TextPart<'a> {
    fn render_into<W: std::fmt::Write>(self, writer: &mut W) -> std::fmt::Result {
        match self {
            TextPart::Empty => Ok(()),
            TextPart::NewLine => writer.write_str("<br/>"),
            TextPart::Raw(content) => writer.write_str(content.as_str()),
            TextPart::Chained(content) => {
                for entry in content {
                    entry.render_into(writer)?;
                }
                Ok(())
            }
            TextPart::Nested(inner) => inner.render_into(writer),
        }
    }
}

impl<'a> Render for TextComponent<'a> {
    fn render_into<W: Write>(self, writer: &mut W) -> std::fmt::Result {
        if matches!(self.content, TextPart::Empty) {
            return Ok(());
        }

        self.style.render_content(self.content, writer)
    }
}

impl Alignment {
    pub fn as_css(&self) -> Option<&'static str> {
        Some(match self {
            Alignment::None => return None,
            Alignment::Left => "text-align:left;",
            Alignment::Center => "text-align:center;",
            Alignment::Right => "text-align:right;",
        })
    }
}

impl<'a> Render for TableComponent<'a> {
    fn render_into<W: Write>(self, writer: &mut W) -> std::fmt::Result {
        let mut id = None;
        let style = if self.alignment.iter().any(|it| *it != Alignment::None) {
            id = Some(random_id());

            let mut style = String::new();
            for (i, it) in self.alignment.into_iter().map(|it| it.as_css()).enumerate() {
                if let Some(alignment) = it {
                    let _ = style.write_fmt(format_args!(
                        "table#{} td:nth-child({}){{{}}}",
                        id.as_ref().unwrap(),
                        i + 1,
                        alignment
                    ));
                }
            }
            Some(style)
        } else {
            None
        };

        if style.is_some() {
            writer.write_str("<table id=\"")?;
            writer.write_str(id.unwrap().as_ref())?;
            writer.write_str("\">")?;
        } else {
            writer.write_str("<table>")?;
        };

        writer.write_str("<thead>")?;
        for header in self.headers {
            writer.write_str("<td>")?;
            header.render_into(writer)?;
            writer.write_str("</td>")?;
        }
        writer.write_str("</thead>")?;

        writer.write_str("<tbody>")?;
        for row in self.rows {
            writer.write_str("<tr>")?;
            for cell in row.into_iter() {
                writer.write_str("<td>")?;
                cell.render_into(writer)?;
                writer.write_str("</td>")?;
            }
            writer.write_str("</tr>")?;
        }
        writer.write_str("</tbody>")?;
        writer.write_str("</table>")?;

        if let Some(s) = style {
            writer.write_str(html! {<style>{s}</style>}.as_str())?;
        }

        Ok(())
    }
}

impl<'a> Render for ListComponent<'a> {
    fn render_into<W: Write>(self, writer: &mut W) -> std::fmt::Result {
        let tag = self
            .numbered
            .map(|first| {
                if first > 0 {
                    format!("ol start=\"{}\"", first)
                } else {
                    "ol".to_string()
                }
            })
            .unwrap_or("ul".to_string());
        writer.write_char('<')?;
        writer.write_str(tag.as_str())?;
        writer.write_char('>')?;

        for item in self.items {
            writer.write_str("<li>")?;
            item.render_into(writer)?;
            writer.write_str("</li>")?;
        }

        writer.write_str("</")?;
        writer.write_str(if self.numbered.is_some() { "ol" } else { "ul" })?;
        writer.write_char('>')
    }
}

impl<'a> Render for PostComponent<'a> {
    fn render_into<W: std::fmt::Write>(self, writer: &mut W) -> std::fmt::Result {
        match self {
            PostComponent::Placeholder => panic!("can't render placeholder component"),
            PostComponent::Text(text) => {
                text.render_into(writer)
            }
            PostComponent::BlockQuote(content) => {
                writer.write_str("<blockquote>")?;
                content.render_into(writer)?;
                writer.write_str("</blockquote>")
            }
            PostComponent::Image { source, alt } => {
                writer.write_str(
                    html! {
                        <img src={source} alt={alt} />
                    }
                    .as_str(),
                )
            }
            PostComponent::CodeBlock { language, content } => {
                let class = match language {
                    Some(lang) => format!("block language-{}", lang),
                    None => "block".to_string(),
                };

                writer.write_str(
                    html! {
                        <pre><code class={class}>
                        {content}
                        </code></pre>
                    }
                    .as_str(),
                )
            }
            PostComponent::List(it) => it.render_into(writer),
            PostComponent::HorizonalRule => writer.write_str("<hr/>"),
            PostComponent::Table(it) => it.render_into(writer),
            PostComponent::Footnote { id, text } => writer.write_str(
                html! {
                    <aside id={"footnote-".to_string() + id.as_str()}><span class={"fn-id"}>{(id, ":")}</span>{(" ", text)}</aside>
                }
                .as_str(),
            ),
            PostComponent::Latex(it) => it.render_into(writer),
            PostComponent::Chained(items) => {
                for item in items {
                    item.render_into(writer)?;
                }
                Ok(())
            },
            PostComponent::Raw(html) => writer.write_str(&html),
        }
    }
}

impl<'a> Render for LatexComponent<'a> {
    fn render_into<W: Write>(self, writer: &mut W) -> std::fmt::Result {
        writer.write_str("<code data-lang=\"latex\">")?;
        writer.write_str(&self.source)?;
        writer.write_str("</code>")
    }
}
