//! Dump native embeddings for parity checks against the reference model.

use std::env;

fn main() {
    let texts: Vec<String> = env::args().skip(1).collect();
    if texts.is_empty() {
        eprintln!("usage: isobar-embed-dump TEXT [TEXT ...]");
        std::process::exit(2);
    }
    if texts[0] == "--tokens" {
        for (index, text) in texts.iter().skip(1).enumerate() {
            let (ids, len) = isobar_scorer::embed::debug_token_ids(text.as_bytes());
            print!("{index}");
            for id in ids.into_iter().take(len) {
                print!("\t{id}");
            }
            println!();
        }
        return;
    }
    for (index, text) in texts.iter().enumerate() {
        print!("{index}");
        let embedding = isobar_scorer::embed::encode(text.as_bytes());
        for value in embedding.values {
            print!("\t{}", value.to_bits());
        }
        println!();
    }
}
