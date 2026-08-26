//! Telegraph baseline composite and monotonic contrast layer.

use crate::bm25;
use crate::embed;
use crate::math;
use crate::weather;
use crate::Breakdown;

// Keep the first fitted curve deliberately moderate until it is re-fit against
// an independent baseline score vector. A steep curve is not evidence of
// improvement when the agreement measurement is invalid.
pub const DEFAULT_STEEPNESS: f32 = 16.0;
pub const DEFAULT_CENTRE: f32 = 0.4;

#[derive(Clone, Copy)]
pub struct ScoringParams {
    pub steepness: f32,
    pub centre: f32,
}

impl ScoringParams {
    pub const fn default() -> Self {
        Self {
            steepness: DEFAULT_STEEPNESS,
            centre: DEFAULT_CENTRE,
        }
    }
}

#[inline]
fn empty_answer(bytes: &[u8]) -> bool {
    match core::str::from_utf8(bytes) {
        Ok(text) => text.trim().is_empty(),
        Err(_) => bytes.is_empty(),
    }
}

/// Return the four signals and the untransformed Telegraph baseline score.
///
/// The question relevance, ground-truth correctness, BM25 lexical overlap,
/// and answer-length quality weights match the published baseline exactly.
fn raw_components(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
) -> (f32, f32, f32, f32, f32) {
    if empty_answer(miner_answer) {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let question_embedding = embed::encode(question);
    let ground_truth_embedding = embed::encode(ground_truth);
    let answer_embedding = embed::encode(miner_answer);
    raw_components_from_embeddings(
        &question_embedding,
        &ground_truth_embedding,
        &answer_embedding,
        question,
        ground_truth,
        miner_answer,
    )
}

/// Compute a baseline score from precomputed embeddings. This is used by the
/// native harness to avoid re-encoding repeated question/reference pairs; the
/// exported scorer continues to take the uncached path above.
pub fn raw_score_from_embeddings(
    question_embedding: &embed::Embedding,
    ground_truth_embedding: &embed::Embedding,
    answer_embedding: &embed::Embedding,
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
) -> f32 {
    raw_components_from_embeddings(
        question_embedding,
        ground_truth_embedding,
        answer_embedding,
        question,
        ground_truth,
        miner_answer,
    )
    .4
}

fn raw_components_from_embeddings(
    question_embedding: &embed::Embedding,
    ground_truth_embedding: &embed::Embedding,
    answer_embedding: &embed::Embedding,
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
) -> (f32, f32, f32, f32, f32) {
    if empty_answer(miner_answer) {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let relevance = embed::cosine(question_embedding, answer_embedding);
    let correctness = embed::cosine(ground_truth_embedding, answer_embedding);
    let lexical = bm25::similarity(ground_truth, miner_answer);
    let length_quality = math::sigmoid((miner_answer.len() as f32 - 50.0) / 20.0);
    let raw = math::clamp01(
        0.25 * relevance + 0.50 * correctness + 0.15 * lexical + 0.10 * length_quality,
    );
    let typed = weather::adjustment(question, ground_truth, miner_answer);
    let raw = math::clamp01(raw + typed);
    (relevance, correctness, lexical, length_quality, raw)
}

pub fn breakdown_with_params(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    params: ScoringParams,
) -> Breakdown {
    let (relevance, correctness, lexical, length_quality, raw_score) =
        raw_components(question, ground_truth, miner_answer);
    let score = math::quantize6(math::contrast_norm(
        raw_score,
        params.steepness,
        params.centre,
    ));
    Breakdown {
        relevance,
        correctness,
        lexical,
        length_quality,
        raw_score,
        score,
    }
}

pub fn breakdown(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> Breakdown {
    breakdown_with_params(
        question,
        ground_truth,
        miner_answer,
        ScoringParams::default(),
    )
}

pub fn score_with_params(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    params: ScoringParams,
) -> f32 {
    breakdown_with_params(question, ground_truth, miner_answer, params).score
}

pub fn score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    breakdown(question, ground_truth, miner_answer).score
}

/// Untransformed baseline score used only for comparison and fixture fitting.
pub fn baseline_score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    raw_components(question, ground_truth, miner_answer).4
}

pub fn raw_score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    baseline_score(question, ground_truth, miner_answer)
}
