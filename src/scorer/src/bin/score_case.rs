//! Score one captured question/reference/answer triple with the local replica.

use isobar_scorer::scorer::{self, ScoringParams};
use std::env;

fn usage() {
    eprintln!("usage: isobar-score-case [--k VALUE] [--c VALUE] QUESTION GROUND_TRUTH ANSWER");
}

fn main() {
    let mut args = env::args().skip(1);
    let mut steepness = scorer::DEFAULT_STEEPNESS;
    let mut centre = scorer::DEFAULT_CENTRE;
    let mut positional = Vec::new();
    while let Some(value) = args.next() {
        match value.as_str() {
            "--k" => {
                steepness = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(scorer::DEFAULT_STEEPNESS);
            }
            "--c" => {
                centre = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(scorer::DEFAULT_CENTRE);
            }
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => positional.push(value),
        }
    }
    if positional.len() != 3 {
        usage();
        std::process::exit(2);
    }
    let breakdown = scorer::breakdown_with_params(
        positional[0].as_bytes(),
        positional[1].as_bytes(),
        positional[2].as_bytes(),
        ScoringParams { steepness, centre },
    );
    println!("relevance       {:.6}", breakdown.relevance);
    println!("correctness     {:.6}", breakdown.correctness);
    println!("lexical         {:.6}", breakdown.lexical);
    println!("length_quality  {:.6}", breakdown.length_quality);
    println!("raw_score       {:.6}", breakdown.raw_score);
    println!("score           {:.6}", breakdown.score);
    println!(
        "typed_adjustment {:.6}",
        isobar_scorer::weather::adjustment(
            positional[0].as_bytes(),
            positional[1].as_bytes(),
            positional[2].as_bytes(),
        )
    );
}
