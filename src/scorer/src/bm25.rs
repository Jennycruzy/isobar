//! Allocation-free BM25 lexical scorer matching Telegraph's published baseline.
//!
//! The production baseline uses a single-document BM25 variant. It treats
//! inverse document frequency as constant, uses `k1 = 1.5` and `b = 0.75`,
//! and normalizes by the average of the query and document lengths. The
//! reference implementation compares string tokens; this module stores a
//! deterministic FNV-1a token fingerprint in fixed arrays so the same rule
//! remains usable in `no_std` WASM without a heap or hash map.

use crate::math;

const MAX_TERMS: usize = 256;
const K1: f32 = 1.5;
const B: f32 = 0.75;

#[derive(Clone, Copy)]
struct Terms {
    hashes: [u64; MAX_TERMS],
    len: usize,
}

impl Terms {
    const fn empty() -> Self {
        Self {
            hashes: [0; MAX_TERMS],
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, hash: u64) {
        if self.len < MAX_TERMS {
            self.hashes[self.len] = hash;
            self.len += 1;
        }
    }
}

#[inline]
fn lower_ascii(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + (b'a' - b'A')
    } else {
        byte
    }
}

#[inline]
fn hash_token(bytes: &[u8]) -> u64 {
    let mut hash = 14_695_981_039_346_656_037u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= lower_ascii(bytes[index]) as u64;
        hash = hash.wrapping_mul(1_099_511_628_211);
        index += 1;
    }
    hash
}

/// Tokenize exactly as the published baseline BM25 implementation does:
/// Unicode alphanumeric runs, ASCII lowercasing, and a two-byte minimum.
fn tokenize(bytes: &[u8]) -> Terms {
    let Ok(text) = core::str::from_utf8(bytes) else {
        return Terms::empty();
    };

    let mut terms = Terms::empty();
    let mut start = None;
    for (offset, character) in text.char_indices() {
        if character.is_alphanumeric() {
            if start.is_none() {
                start = Some(offset);
            }
        } else if let Some(begin) = start.take() {
            if offset - begin >= 2 {
                terms.push(hash_token(&text.as_bytes()[begin..offset]));
            }
        }
    }
    if let Some(begin) = start {
        if text.len() - begin >= 2 {
            terms.push(hash_token(&text.as_bytes()[begin..]));
        }
    }
    terms
}

#[inline]
fn count(document: &Terms, token: u64) -> usize {
    let mut total = 0;
    let mut index = 0;
    while index < document.len {
        if document.hashes[index] == token {
            total += 1;
        }
        index += 1;
    }
    total
}

/// Score a document against a ground-truth query using the baseline formula.
pub fn similarity(query: &[u8], document: &[u8]) -> f32 {
    let query = tokenize(query);
    let document = tokenize(document);
    if query.len == 0 || document.len == 0 {
        return 0.0;
    }

    let document_length = document.len as f32;
    let average_length = ((query.len + document.len) as f32) / 2.0;
    let length_norm = 1.0 - B + B * document_length / average_length;
    let mut raw = 0.0f32;
    let mut max_raw = 0.0f32;

    let mut index = 0;
    while index < query.len {
        let term_frequency = count(&document, query.hashes[index]) as f32;
        let tf_norm = (term_frequency * (K1 + 1.0)) / (term_frequency + K1 * length_norm);
        raw += tf_norm;
        max_raw += K1 + 1.0;
        index += 1;
    }

    math::clamp01(math::safe_div(raw, max_raw, 0.0))
}

#[cfg(test)]
mod tests {
    use super::similarity;

    #[test]
    fn exact_match_has_baseline_scale() {
        // The published single-document normalization divides by the
        // asymptotic TF upper bound, so an exact one-occurrence match is
        // deliberately below 1.0.
        assert!(
            similarity(
                b"the capital of france is paris",
                b"the capital of france is paris"
            ) > 0.35
        );
    }

    #[test]
    fn no_overlap_is_zero() {
        assert_eq!(
            similarity(b"france paris capital", b"banana mango tropical fruit"),
            0.0
        );
    }

    #[test]
    fn empty_inputs_are_zero() {
        assert_eq!(similarity(b"", b"some document"), 0.0);
        assert_eq!(similarity(b"some query", b""), 0.0);
    }
}
