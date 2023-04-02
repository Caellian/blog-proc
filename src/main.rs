use arguments::{Args, Verb};
use clap::Parser;
use error::BlogError;
use post::PostContent;
use pulldown_cmark::{html, Options, Parser as MarkdownParser};

use crate::{generator::Redirect, post::Post};

pub(crate) mod arguments;
pub mod error;
pub mod generator;
pub mod post;

#[tokio::main]
async fn main() {
    env_logger::builder().init();

    let args = Args::parse();

    match args.verb {
        Verb::Build => build(args).await,
        _ => todo!(),
    }
    .unwrap();
}

async fn build(args: Args) -> Result<(), BlogError> {
    let mut content = PostContent::open("./test/input.md")?;
    let info = content.take_info()?;

    let data = {
        let options = Options::all();
        let parser = MarkdownParser::new_ext(&content.inner, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        content.inner = html_output;
        Post { info, content }
    };
    println!("{:?}", data);

    let mut reg = handlebars::Handlebars::new();
    let template = std::fs::read_to_string("./test/article.hbs")?;
    let rendered = reg
        .render_template(&template, &data)
        .map_err(|err| BlogError::Format(err.into()))?;
    std::fs::write("./test/output.html", rendered);

    Ok::<(), BlogError>(())
}
