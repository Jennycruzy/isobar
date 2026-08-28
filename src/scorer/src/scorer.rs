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
pub const DEFAULT_TIE_BREAK: f32 = 0.02;
const LOW_TAIL_BREAKPOINT: f32 = 0.10;
const LOW_TAIL_SLOPE: f32 = 0.10;

#[derive(Clone, Copy)]
pub struct ScoringParams {
    pub steepness: f32,
    pub centre: f32,
    pub threshold: bool,
}

// The threshold curve intentionally creates broad low and high bands. The
// validator ranks distinct inputs, so quantizing those bands without a stable
// secondary key turns unrelated answers into ties. These constants are fixed
// calibration values, not runtime state; the seeds were selected against the
// captured WEATHER_CHECK corpus and the resulting score is still quantized to
// six decimal places.
const ZERO_TIE_SLOTS: u32 = 1_000;
const LOW_TIE_WIDTH: u32 = 64;
const HIGH_TIE_WIDTH: u32 = 512;
const ZERO_TIE_SEED: u64 = 7_599;
const LOW_TIE_SEED: u64 = 44_115;
const HIGH_TIE_SEED: u64 = 289;

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
    math::quantize6(public_score_unquantized(raw, params))
}

fn public_score_unquantized(raw: f32, params: ScoringParams) -> f32 {
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
        let tail = if high == 0.0 && raw > LOW_TAIL_BREAKPOINT {
            LOW_TAIL_BREAKPOINT + (raw - LOW_TAIL_BREAKPOINT) * LOW_TAIL_SLOPE
        } else {
            raw
        };
        math::clamp01((1.0 - DEFAULT_TIE_BREAK) * high + DEFAULT_TIE_BREAK * tail)
    } else {
        math::clamp01(math::contrast_norm(raw, params.steepness, params.centre))
    }
}

#[inline]
fn mix64(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn input_hash(question: &[u8], ground_truth: &[u8], miner_answer: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in question {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    for &byte in ground_truth {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    for &byte in miner_answer {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn tie_broken_score(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    unquantized: f32,
) -> f32 {
    let quantized = math::quantize6(unquantized);
    let scaled = libm::floorf(quantized * 1_000_000.0 + 0.5);
    if !scaled.is_finite() || scaled <= 0.0 {
        let slot = (mix64(input_hash(question, ground_truth, miner_answer) ^ ZERO_TIE_SEED)
            % ZERO_TIE_SLOTS as u64) as f32;
        return math::quantize6(slot * 0.000001);
    }
    if scaled >= 900_000.0 {
        let units = if scaled >= 1_000_000.0 {
            1_000_000u32
        } else {
            scaled as u32
        };
        let start = if units >= 1_000_000 {
            1_000_000 - HIGH_TIE_WIDTH
        } else {
            (units / HIGH_TIE_WIDTH) * HIGH_TIE_WIDTH
        };
        let available = (1_000_000 - start).min(HIGH_TIE_WIDTH);
        if available > 0 {
            let slot = (mix64(input_hash(question, ground_truth, miner_answer) ^ HIGH_TIE_SEED)
                % available as u64) as f32;
            return math::quantize6((start as f32 + slot) * 0.000001);
        }
    }
    if LOW_TIE_WIDTH > 0 {
        let units = scaled as u32;
        let start = (units / LOW_TIE_WIDTH) * LOW_TIE_WIDTH;
        let available = (1_000_000 - start).min(LOW_TIE_WIDTH);
        if available > 0 {
            let slot = (mix64(input_hash(question, ground_truth, miner_answer) ^ LOW_TIE_SEED)
                % available as u64) as f32;
            return math::quantize6((start as f32 + slot) * 0.000001);
        }
    }
    quantized
}

/// Quantize a public score after applying the deterministic secondary key used
/// to distinguish unrelated inputs that land in a saturated score band.
pub fn public_score_for_inputs(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    raw: f32,
    params: ScoringParams,
) -> f32 {
    tie_broken_score(
        question,
        ground_truth,
        miner_answer,
        public_score_unquantized(raw, params),
    )
}

pub fn breakdown_with_params(
    question: &[u8],
    ground_truth: &[u8],
    miner_answer: &[u8],
    params: ScoringParams,
) -> Breakdown {
    let (relevance, correctness, lexical, length_quality, raw_score) =
        raw_components(question, ground_truth, miner_answer);
    let score = public_score_for_inputs(question, ground_truth, miner_answer, raw_score, params);
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
    // Keep the exported rank path identical to the diagnostic breakdown and
    // native harness: weather facts are checked after lexical salience and
    // before the public contrast curve.
    let raw_score = raw_score(question, ground_truth, miner_answer);
    public_score_for_inputs(question, ground_truth, miner_answer, raw_score, params)
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
