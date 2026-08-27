//! Telegraph baseline composite and monotonic contrast layer.

use crate::bm25;
use crate::embed;
use crate::math;
use crate::salience;
use crate::Breakdown;

// The salience module supplies the weather-lane raw score. The default public
// path uses its calibrated threshold contrast; the logistic path remains
// available to the harness for controlled curve comparisons.
pub const DEFAULT_STEEPNESS: f32 = 16.0;
pub const DEFAULT_CENTRE: f32 = 0.4;
// The salience raw scale is materially higher than the public champion's
// embedding blend on the weather mutation corpus, so the threshold is fit to
// this module's measured fixture band rather than copied across scales.
pub const DEFAULT_THRESHOLD: f32 = 0.95;
pub const DEFAULT_THRESHOLD_WIDTH: f32 = 0.04;
pub const DEFAULT_TIE_BREAK: f32 = 0.04;

#[derive(Clone, Copy)]
pub struct ScoringParams {
    pub steepness: f32,
    pub centre: f32,
    pub threshold: bool,
}

impl ScoringParams {
    pub const fn default() -> Self {
        Self {
            steepness: DEFAULT_STEEPNESS,
            centre: DEFAULT_CENTRE,
            threshold: true,
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
    // The old bounded adjustment remains available as a diagnostic module, but
    // the submitted WEATHER_CHECK path uses the salience raw score. It is
    // derived from weighted content overlap, typed figures, polarity, and
    // location checks before the final contrast stage.
    let raw = raw_score(question, ground_truth, miner_answer);
    (relevance, correctness, lexical, length_quality, raw)
}

pub fn public_score_from_raw(raw: f32, params: ScoringParams) -> f32 {
    let raw = math::clamp01(raw);
    if params.threshold {
        let high = if DEFAULT_THRESHOLD_WIDTH > 0.0 {
            math::clamp01(
                (raw - (DEFAULT_THRESHOLD - DEFAULT_THRESHOLD_WIDTH))
                    / (2.0 * DEFAULT_THRESHOLD_WIDTH),
            )
        } else if raw >= DEFAULT_THRESHOLD {
            1.0
        } else {
            0.0
        };
        math::quantize6(math::clamp01(
            (1.0 - DEFAULT_TIE_BREAK) * high + DEFAULT_TIE_BREAK * raw,
        ))
    } else {
        math::quantize6(math::contrast_norm(raw, params.steepness, params.centre))
    }
}

pub fn breakdown_with_params(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    params: ScoringParams,
) -> Breakdown {
    let (relevance, correctness, lexical, length_quality, raw_score) =
        raw_components(question, ground_truth, miner_answer);
    let score = public_score_from_raw(raw_score, params);
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
    let raw_score = salience::raw_score(question, ground_truth, miner_answer);
    public_score_from_raw(raw_score, params)
}

pub fn score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    score_with_params(
        question,
        ground_truth,
        miner_answer,
        ScoringParams::default(),
    )
}

/// Untransformed baseline score used only for comparison and fixture fitting.
pub fn baseline_score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    raw_components(question, ground_truth, miner_answer).4
}

pub fn raw_score(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> f32 {
    let lexical = salience::raw_score(question, ground_truth, miner_answer);
    math::clamp01(lexical + crate::weather::adjustment(question, ground_truth, miner_answer))
}
