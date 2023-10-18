use arguments::{Args, Verb};
use clap::Parser;
use error::BlogError;
use post::RawPostContent;

use crate::{blog::Blog, post::Post};

pub(crate) mod arguments;
pub mod blog;
pub mod component;
pub mod error;
pub mod post;
pub mod template;
pub mod util;

fn main() {
    env_logger::builder().init();

    let args = Args::parse();

    let mut blog = Blog::open(&args.working_dir).expect("unable to open blog directory");

    blog.load_target_metadata(&args.target_dir)
        .expect("unable to load blog metadata");

    /*
    let index = blog.file_index.get_or_insert_with(FileIndex::new);
    for f in blog.sources() {
        index.note(f.path());
    }
    */

    match args.verb {
        Verb::Build => build(&mut blog, &args),
        _ => todo!(),
    }
    .unwrap();

    blog.write_target_metadata(&args.target_dir)
        .expect("unable to write blog metadata");
}

fn build(blog: &mut Blog, args: &Args) -> Result<(), BlogError> {
    std::fs::create_dir_all(&args.target_dir)?;

    let reg = template::engine().read().expect("engine poisoned");

    log::info!("Loading new posts:");
    let mut errors = vec![];
    for source in blog.sources() {
        let path = source.path().to_path_buf();
        log::info!("- {}", path.to_string_lossy());

        let source_name = path
            .file_name()
            .expect("no file name")
            .to_str()
            .expect("non UTF-8 name")
            .to_string();

        let raw = match RawPostContent::open(&path) {
            Ok(it) => it,
            Err(err) => {
                errors.push((source_name, err));
                continue;
            }
        };

        let post = match Post::new(raw) {
            Ok(it) => it,
            Err(err) => {
                errors.push((source_name, err));
                continue;
            }
        };

        let data = post.template_ctx();

        let rendered = reg
            .render("article", &data)
            .map_err(|err| BlogError::Format(err.into()))?;

        let target_name = data
            .info
            .slug
            .or_else(|| source_name.split(".").next().map(|it| it.to_string()))
            .unwrap_or_else(|| "output".to_string());

        std::fs::write(
            args.target_dir.join(target_name + "." + &args.ext),
            rendered,
        );
    }

    if errors.is_empty() {
        log::info!("Following errors occurred during build:");
        for (name, err) in errors {
            log::info!("- {}: {}", name, err)
        }
    }

    Ok::<(), BlogError>(())
}
