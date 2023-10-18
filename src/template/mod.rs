use handlebars::{Handlebars, Template};
use std::{
    mem::MaybeUninit,
    path::Path,
    sync::{Once, RwLock},
};

use crate::error::FormatError;

pub mod article;

macro_rules! load_static_template {
    ($reg:ident, $path:literal, $name:literal) => {{
        let tpl = match Template::compile(include_str!($path)) {
            Ok(it) => it,
            Err(_) => {
                panic!("Failed to compile template: {} (path: {})", $name, $path);
            }
        };
        $reg.register_template($name, tpl);
    }};
}

fn init_engine() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();

    load_static_template!(handlebars, "./redirect.hbs", "redirect");
    load_static_template!(handlebars, "./article.hbs", "article");

    handlebars
}

pub fn engine() -> &'static mut RwLock<Handlebars<'static>> {
    static mut ENGINE: MaybeUninit<RwLock<Handlebars<'static>>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            ENGINE.write(RwLock::new(init_engine()));
        });

        ENGINE.assume_init_mut()
    }
}

pub trait Generate {
    fn generate(&self, path: impl AsRef<Path>) -> Result<String, FormatError>;
}
