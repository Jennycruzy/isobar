//! Deterministic 384-dimensional text embeddings.
//!
//! Projection mode is intentionally small and allocation-free so scorer
//! iteration can happen without model assets. The `real_weights` feature uses
//! the same shape with an INT8-style fixed-point accumulation path; a real
//! MiniLM payload is a separately versioned artifact and is not silently
//! fabricated in this repository.

use crate::math;
#[cfg(not(feature = "real_weights"))]
use crate::tokenizer;

pub const EMBED_DIM: usize = 384;

#[derive(Clone, Copy)]
pub struct Embedding {
    pub values: [f32; EMBED_DIM],
}

impl Embedding {
    pub const fn zero() -> Self {
        Self {
            values: [0.0; EMBED_DIM],
        }
    }

    #[cfg(not(feature = "real_weights"))]
    fn normalize(&mut self) {
        let mut sum = 0.0f32;
        let mut index = 0;
        while index < EMBED_DIM {
            sum += self.values[index] * self.values[index];
            index += 1;
        }
        let norm = libm::sqrtf(sum);
        if norm <= 1.0e-12 || norm.is_nan() || norm.is_infinite() {
            self.values = [0.0; EMBED_DIM];
            return;
        }
        index = 0;
        while index < EMBED_DIM {
            self.values[index] = math::finite_or_zero(self.values[index] / norm);
            index += 1;
        }
    }
}

#[cfg(not(feature = "real_weights"))]
#[inline]
fn mix(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[cfg(not(feature = "real_weights"))]
#[inline]
fn bucket(hash: u64, salt: u64) -> usize {
    (mix(hash ^ salt) as usize) % EMBED_DIM
}

#[cfg(not(feature = "real_weights"))]
#[inline]
fn sign(hash: u64, salt: u64) -> f32 {
    if mix(hash ^ salt) & 1 == 0 {
        1.0
    } else {
        -1.0
    }
}

#[cfg(not(feature = "real_weights"))]
fn encode_projection(bytes: &[u8]) -> Embedding {
    let tokens = tokenizer::tokenize(bytes);
    let mut output = Embedding::zero();
    let mut index = 0;
    while index < tokens.len {
        let token = tokens.tokens[index];
        let hash = mix(token ^ ((index as u64) << 32));
        let first = bucket(hash, 0x11);
        let second = bucket(hash, 0x22);
        let third = bucket(hash, 0x33);
        output.values[first] += sign(hash, 0x44);
        output.values[second] += 0.75 * sign(hash, 0x55);
        output.values[third] += 0.5 * sign(hash, 0x66);
        if index > 0 {
            let previous = tokens.tokens[index - 1];
            let pair = mix(previous.rotate_left(17) ^ token);
            output.values[bucket(pair, 0x77)] += 0.35 * sign(pair, 0x88);
        }
        index += 1;
    }
    output.normalize();
    output
}

pub fn encode(bytes: &[u8]) -> Embedding {
    #[cfg(feature = "real_weights")]
    {
        crate::minilm::encode(bytes)
    }
    #[cfg(not(feature = "real_weights"))]
    {
        encode_projection(bytes)
    }
}

#[cfg(feature = "real_weights")]
pub fn debug_token_ids(bytes: &[u8]) -> ([u32; 128], usize) {
    crate::minilm::debug_token_ids(bytes)
}

pub fn similarity(left: &[u8], right: &[u8]) -> f32 {
    let left = encode(left);
    let right = encode(right);
    cosine(&left, &right)
}

/// Baseline cosine similarity. Negative cosine values are clamped to zero;
/// unlike the former projection helper, cosine is not shifted by `+1 / 2`.
pub fn cosine(left: &Embedding, right: &Embedding) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_left = 0.0f32;
    let mut norm_right = 0.0f32;
    let mut index = 0;
    while index < EMBED_DIM {
        let left_value = left.values[index];
        let right_value = right.values[index];
        dot += left_value * right_value;
        norm_left += left_value * left_value;
        norm_right += right_value * right_value;
        index += 1;
    }
    if norm_left == 0.0 || norm_right == 0.0 {
        0.0
    } else {
        math::clamp01(math::safe_div(
            dot,
            libm::sqrtf(norm_left) * libm::sqrtf(norm_right),
            0.0,
        ))
    }
}
