//! `assay` — deterministic Telegraph answer scoring for `wasm32-unknown-unknown`.
//!
//! The public ABI accepts pointer/length pairs for UTF-8 byte strings. Scores
//! are returned as `f32` and quantized to six decimal places at the final
//! boundary. `breakdown_answer` returns a pointer to a stable, module-owned
//! `Breakdown` record that remains valid until the next breakdown call.

#![cfg_attr(target_arch = "wasm32", no_std)]

pub mod allocator;
pub mod bm25;
pub mod embed;
pub mod math;
#[cfg(feature = "real_weights")]
mod minilm;
pub mod scorer;
pub mod tokenizer;
pub mod weather;

use core::cell::UnsafeCell;
use core::slice;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Breakdown {
    pub relevance: f32,
    pub correctness: f32,
    pub lexical: f32,
    pub length_quality: f32,
    pub raw_score: f32,
    pub score: f32,
}

impl Breakdown {
    const fn zero() -> Self {
        Self {
            relevance: 0.0,
            correctness: 0.0,
            lexical: 0.0,
            length_quality: 0.0,
            raw_score: 0.0,
            score: 0.0,
        }
    }
}

struct BreakdownCell(UnsafeCell<Breakdown>);
unsafe impl Sync for BreakdownCell {}
static LAST_BREAKDOWN: BreakdownCell = BreakdownCell(UnsafeCell::new(Breakdown::zero()));

#[inline]
fn input(ptr: *const u8, len: usize) -> &'static [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        // The host owns validation of pointer/length pairs. The exported ABI
        // is intentionally minimal and mirrors wazero's linear-memory calls.
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

/// Allocate an input buffer in the module's deterministic arena.
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    allocator::alloc(size)
}

/// Release an input buffer previously returned by `alloc`.
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    allocator::dealloc(ptr, size);
}

/// Score an answer using the default endpoint-pinned contrast curve.
#[no_mangle]
pub extern "C" fn rank_answer(
    question_ptr: *const u8,
    question_len: usize,
    ground_truth_ptr: *const u8,
    ground_truth_len: usize,
    miner_answer_ptr: *const u8,
    miner_answer_len: usize,
) -> f32 {
    scorer::score(
        input(question_ptr, question_len),
        input(ground_truth_ptr, ground_truth_len),
        input(miner_answer_ptr, miner_answer_len),
    )
}

/// Compute component signals, the pre-contrast raw score, and the public score.
/// The returned pointer refers to module-owned storage and must not be freed.
#[no_mangle]
pub extern "C" fn breakdown_answer(
    question_ptr: *const u8,
    question_len: usize,
    ground_truth_ptr: *const u8,
    ground_truth_len: usize,
    miner_answer_ptr: *const u8,
    miner_answer_len: usize,
) -> *mut Breakdown {
    let breakdown = scorer::breakdown(
        input(question_ptr, question_len),
        input(ground_truth_ptr, ground_truth_len),
        input(miner_answer_ptr, miner_answer_len),
    );
    unsafe {
        *LAST_BREAKDOWN.0.get() = breakdown;
        LAST_BREAKDOWN.0.get()
    }
}

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic_handler(_panic: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::scorer;

    #[test]
    fn self_match_is_high() {
        let score = scorer::score(b"q", b"A deterministic answer", b"A deterministic answer");
        assert!(score >= 0.75, "self score was {score}");
    }

    #[test]
    fn contrast_is_monotonic_on_grid() {
        let mut previous = 0.0;
        let mut index = 0;
        while index <= 100 {
            let raw = index as f32 / 100.0;
            let current = crate::math::contrast_norm(raw, 8.0, 0.5);
            assert!(current >= previous, "curve decreased at {raw}");
            previous = current;
            index += 1;
        }
    }

    #[test]
    fn contrast_pins_endpoints_and_public_scores_are_finite() {
        assert_eq!(crate::math::contrast_norm(0.0, 8.0, 0.5), 0.0);
        assert_eq!(crate::math::contrast_norm(1.0, 8.0, 0.5), 1.0);
        let score = scorer::score(b"q", b"reference answer", b"unrelated answer");
        assert!(score.is_finite());
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn exported_abi_returns_breakdown_with_raw_score() {
        let question = b"What is the answer?";
        let truth = b"The answer is deterministic.";
        let answer = b"The answer is deterministic.";
        let pointer = super::breakdown_answer(
            question.as_ptr(),
            question.len(),
            truth.as_ptr(),
            truth.len(),
            answer.as_ptr(),
            answer.len(),
        );
        let breakdown = unsafe { *pointer };
        assert_eq!(breakdown.correctness, 1.0);
        assert!(breakdown.raw_score.is_finite());
        assert!((0.0..=1.0).contains(&breakdown.raw_score));
        assert!((0.0..=1.0).contains(&breakdown.score));
    }

    #[cfg(feature = "real_weights")]
    #[test]
    fn real_weights_produce_a_normalized_embedding() {
        let embedding = crate::embed::encode(b"A deterministic sentence for the model.");
        let mut norm = 0.0f32;
        for value in embedding.values {
            norm += value * value;
        }
        assert!(
            norm > 0.99 && norm < 1.01,
            "unexpected embedding norm: {norm}"
        );
    }
}
