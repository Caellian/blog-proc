use std::process::Command;

use rand::Rng;

const RAND_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub fn random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..RAND_CHARSET.len());
            RAND_CHARSET[idx] as char
        })
        .collect()
}

pub fn random_id() -> String {
    random_string(8)
}

pub fn program(name: &'static str) -> Command {
    // check local
    // check PATH
    Command::new(name)
}