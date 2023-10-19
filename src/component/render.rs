use std::fmt::Result;

use super::*;

impl<'a> Style<'a> {
    pub fn render_content(&self, content: &dyn Component, writer: &mut String) -> std::fmt::Result {
        match self {
            Style::None => content.render(writer),
            Style::Link(target) => {
                write!(writer, "<a href=\"{}\">", target)?;
                content.render(writer)?;
                writer.write_str("</a>")
            }
            _ => {
                let tag = self.tag();
                write!(writer, "<{}>", tag)?;
                content.render(writer)?;
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
            TextPart::Nested(inner) => inner.render(target),
        }
    }
}

impl<'a> Component for TextComponent<'a> {
    fn render(&self, target: &mut String) -> std::fmt::Result {
        self.style.render_content(&self.content, target)
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

impl<'a> Component for TableComponent<'a> {
    fn render(&self, writer: &mut String) -> std::fmt::Result {
        let mut id = if self.alignment.iter().any(|it| *it != Alignment::None) {
            Some(random_id())
        } else {
            None
        };

        if let Some(id) = &id {
            writer.write_str("<table id=\"")?;
            writer.write_str(id)?;
            writer.write_str("\">")?;
        } else {
            writer.write_str("<table>")?;
        };

        writer.write_str("<thead>")?;
        for header in &self.headers {
            writer.write_str("<td>")?;
            header.render(writer)?;
            writer.write_str("</td>")?;
        }
        writer.write_str("</thead>")?;

        writer.write_str("<tbody>")?;
        for row in &self.rows {
            writer.write_str("<tr>")?;
            for cell in row.into_iter() {
                writer.write_str("<td>")?;
                cell.render(writer)?;
                writer.write_str("</td>")?;
            }
            writer.write_str("</tr>")?;
        }
        writer.write_str("</tbody>")?;
        writer.write_str("</table>")?;

        if let Some(id) = id {
            writer.write_str("<style>")?;
            for (i, it) in self.alignment.iter().map(|it| it.as_css()).enumerate() {
                if let Some(alignment) = it {
                    let _ = writer.write_fmt(format_args!(
                        "table#{} td:nth-child({}){{{}}}",
                        id,
                        i + 1,
                        alignment
                    ));
                }
            }
            writer.write_str("</style>")?;
        }

        Ok(())
    }
}

impl<'a> Component for ListComponent<'a> {
    fn render(&self, writer: &mut String) -> std::fmt::Result {
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

        for item in &self.items {
            writer.write_str("<li>")?;
            item.render(writer)?;
            writer.write_str("</li>")?;
        }

        writer.write_str("</")?;
        writer.write_str(if self.numbered.is_some() { "ol" } else { "ul" })?;
        writer.write_char('>')
    }
}

impl<'a> Component for PostComponent<'a> {
    fn render(&self, writer: &mut String) -> std::fmt::Result {
        match self {
            PostComponent::Placeholder => panic!("can't render placeholder component"),
            PostComponent::Text(text) => text.render(writer),
            PostComponent::BlockQuote(content) => {
                writer.write_str("<blockquote>")?;
                for it in content {
                    it.render(writer)?;
                }
                writer.write_str("</blockquote>")
            }
            PostComponent::Image { source, alt } => {
                writer.write_str("<img src=\"")?;
                writer.write_str(source)?;
                writer.write_str("\"")?;
                if let Some(alt) = alt {
                    writer.write_str(" alt=\"")?;
                    writer.write_str(alt)?;
                    writer.write_str("\"")?;
                }
                writer.write_str("/>")
            }
            PostComponent::CodeBlock { language, content } => {
                writer.write_str("<pre><code class=\"block")?;
                if let Some(language) = language {
                    writer.write_str(" language-")?;
                    writer.write_str(&language)?;
                }
                writer.write_str("\">")?;
                writer.write_str(&content)?;
                writer.write_str("</code></pre>")
            }
            PostComponent::List(it) => it.render(writer),
            PostComponent::HorizonalRule => writer.write_str("<hr/>"),
            PostComponent::Table(it) => it.render(writer),
            PostComponent::Footnote { id, text } => {
                writer.write_str("<aside id=\"footnote-")?;
                writer.write_str(id)?;
                writer.write_str("\"><span class=\"fn-id\">")?;
                writer.write_str(id)?;
                writer.write_str(":</span> ")?;
                text.render(writer)?;
                writer.write_str("</aside>")
            }
            PostComponent::Latex(it) => it.render(writer),
            PostComponent::Chained(items) => {
                for item in items {
                    item.render(writer)?;
                }
                Ok(())
            }
            PostComponent::Raw(html) => writer.write_str(&html),
        }
    }
}

impl<'a> Component for LatexComponent<'a> {
    fn render(&self, writer: &mut String) -> std::fmt::Result {
        writer.write_str("<code data-lang=\"latex\">")?;
        writer.write_str(&self.source)?;
        writer.write_str("</code>")
    }
}
