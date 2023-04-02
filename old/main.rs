#![feature(stmt_expr_attributes)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(generators)]
#![feature(specialization)]

use std::{fs::File, io::BufWriter, path::PathBuf};

use arguments::{Args, GitSource, Verb};
use clap::Parser;
use git::clone_blog;

use crate::blog::Blog;

mod arguments;
mod blog;
mod error;
mod git;
mod parser;
mod status;
mod util;

#[tokio::main]
pub async fn main() {
    env_logger::builder().init();

    let args = Args::parse();

    match args.verb {
        Verb::Clone(repo) => clone(repo, args.working_dir, args.target_dir).await,
        Verb::Pull => pull(args).await,
        Verb::Build => build(args).await,
        Verb::Posts(q) => todo!(),
        Verb::Watch => todo!(),
        Verb::Index => todo!(),
        Verb::Publish => todo!(),
    }
}

async fn clone(source: GitSource, work_dir: PathBuf, target: PathBuf) {
    let blog = clone_blog(source.repo, work_dir).expect("unable to clone blog");
}

async fn pull(args: Args) {
    let work_dir = args.working_dir;

    let mut blog = Blog::open(work_dir).expect("unable to open blog directory");
    blog.pull();
}

async fn build(args: Args) {
    let work_dir = args.working_dir;
    let target = args.target_dir;

    let mut blog = Blog::open(work_dir).expect("unable to open blog directory");

    blog.index_files().iter().map(|(path, entry)| {
        println!("{:?}", path);
        todo!()
    });

    /*
    let posts = blog.posts(Q);

    let posts_file =
        File::create(target.join("posts_latest.json")).expect("unable to create posts file");
    let bw = BufWriter::new(posts_file);

    if args.print_output {}
    serde_json::to_writer(bw, &posts).unwrap();
    */
}
