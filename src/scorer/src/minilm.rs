//! Embedded baseline-compatible INT8 MiniLM-L6-v2 inference.
//!
//! The active artifact is converted from Telegraph's published `MLM2` payload.
//! Its per-tensor scales are expanded into the fixed-array record format used
//! here, while the original scales and f32 biases are preserved. Tokenization,
//! tanh-GELU, pooling, and attention follow the published baseline so the
//! candidate's raw composite can be compared against an independent baseline
//! WASM score vector. LayerNorm parameters remain f32 and all scalar math uses
//! fixed order with `libm`.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::embed::{Embedding, EMBED_DIM};
use crate::math;

const WEIGHTS: &[u8] = include_bytes!("../weights/minilm_l6_v2_baseline.int8.bin");
const VOCAB: &[u8] = include_bytes!("../weights/minilm_vocab.bin");

const WEIGHT_MAGIC: &[u8; 8] = b"ASAYWT1\0";
const VOCAB_MAGIC: &[u8; 8] = b"ASAYVOC1";
const WEIGHT_HEADER_SIZE: usize = 24;
const WEIGHT_RECORD_SIZE: usize = 24;
const VOCAB_HEADER_SIZE: usize = 24;
const VOCAB_RECORD_SIZE: usize = 20;
const MAX_SEQ: usize = 128;
const INTERMEDIATE: usize = 1536;
const HEADS: usize = 12;
const HEAD_DIM: usize = 32;
const SCALE: f32 = 0.176_776_69; // 1 / sqrt(32)
const LN_EPSILON: f32 = 1.0e-12;

const KIND_MATRIX_INT8: u8 = 1;
const KIND_VECTOR_F32: u8 = 2;

const ID_WORD_EMBEDDINGS: u16 = 1;
const ID_POSITION_EMBEDDINGS: u16 = 2;
const ID_TOKEN_TYPE_EMBEDDINGS: u16 = 3;
const ID_EMBEDDING_LN_WEIGHT: u16 = 4;
const ID_EMBEDDING_LN_BIAS: u16 = 5;
const LAYER_BASE: u16 = 16;
const LAYER_STRIDE: u16 = 12;
const PART_QUERY: u16 = 0;
const PART_KEY: u16 = 1;
const PART_VALUE: u16 = 2;
const PART_ATTENTION_OUTPUT: u16 = 3;
const PART_INTERMEDIATE: u16 = 4;
const PART_OUTPUT: u16 = 5;
const PART_ATTENTION_LN_WEIGHT: u16 = 6;
const PART_ATTENTION_LN_BIAS: u16 = 7;
const PART_OUTPUT_LN_WEIGHT: u16 = 8;
const PART_OUTPUT_LN_BIAS: u16 = 9;

#[inline]
const fn layer_id(layer: usize, part: u16) -> u16 {
    LAYER_BASE + (layer as u16) * LAYER_STRIDE + part
}

#[inline]
fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let value = bytes.get(offset..end)?;
    Some(u16::from_le_bytes([value[0], value[1]]))
}

#[inline]
fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let value = bytes.get(offset..end)?;
    Some(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[inline]
fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let value = bytes.get(offset..end)?;
    Some(u64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

#[inline]
fn read_f32(bytes: &[u8], offset: usize) -> Option<f32> {
    Some(f32::from_bits(read_u32(bytes, offset)?))
}

#[derive(Clone, Copy)]
struct WeightRecord {
    id: u16,
    kind: u8,
    rows: usize,
    cols: usize,
    data_offset: usize,
    scale_offset: usize,
    bias_offset: usize,
}

struct WeightStore {
    bytes: &'static [u8],
    count: usize,
    valid: bool,
}

impl WeightStore {
    fn new(bytes: &'static [u8]) -> Self {
        let valid = bytes.get(..8) == Some(WEIGHT_MAGIC.as_slice())
            && read_u32(bytes, 8) == Some(1)
            && read_u32(bytes, 12) == Some(384)
            && read_u32(bytes, 16) == Some(6);
        let count = read_u32(bytes, 20).unwrap_or(0) as usize;
        Self {
            bytes,
            count,
            valid,
        }
    }

    fn record(&self, index: usize) -> Option<WeightRecord> {
        if !self.valid || index >= self.count {
            return None;
        }
        let offset = WEIGHT_HEADER_SIZE.checked_add(index.checked_mul(WEIGHT_RECORD_SIZE)?)?;
        let end = offset.checked_add(WEIGHT_RECORD_SIZE)?;
        if end > self.bytes.len() {
            return None;
        }
        Some(WeightRecord {
            id: read_u16(self.bytes, offset)?,
            kind: *self.bytes.get(offset + 2)?,
            rows: read_u32(self.bytes, offset + 4)? as usize,
            cols: read_u32(self.bytes, offset + 8)? as usize,
            data_offset: read_u32(self.bytes, offset + 12)? as usize,
            scale_offset: read_u32(self.bytes, offset + 16)? as usize,
            bias_offset: read_u32(self.bytes, offset + 20)? as usize,
        })
    }

    fn find(&self, id: u16) -> Option<WeightRecord> {
        let mut index = 0;
        while index < self.count {
            let record = self.record(index)?;
            if record.id == id {
                return Some(record);
            }
            index += 1;
        }
        None
    }

    fn matrix_value(&self, record: WeightRecord, row: usize, column: usize) -> f32 {
        if record.kind != KIND_MATRIX_INT8 || row >= record.rows || column >= record.cols {
            return 0.0;
        }
        let Some(index) = row
            .checked_mul(record.cols)
            .and_then(|value| value.checked_add(column))
            .and_then(|value| record.data_offset.checked_add(value))
        else {
            return 0.0;
        };
        let quantized = self.bytes.get(index).copied().unwrap_or(0) as i8 as f32;
        let scale = read_f32(
            self.bytes,
            record.scale_offset.saturating_add(row.saturating_mul(4)),
        )
        .unwrap_or(0.0);
        math::finite_or_zero(quantized * scale)
    }

    fn bias_value(&self, record: WeightRecord, row: usize) -> f32 {
        if record.bias_offset == 0 || row >= record.rows {
            return 0.0;
        }
        read_f32(
            self.bytes,
            record.bias_offset.saturating_add(row.saturating_mul(4)),
        )
        .map(math::finite_or_zero)
        .unwrap_or(0.0)
    }

    fn vector_value(&self, record: WeightRecord, index: usize) -> f32 {
        if record.kind != KIND_VECTOR_F32 || index >= record.rows {
            return 0.0;
        }
        read_f32(
            self.bytes,
            record.data_offset.saturating_add(index.saturating_mul(4)),
        )
        .map(math::finite_or_zero)
        .unwrap_or(0.0)
    }
}

struct Vocab {
    bytes: &'static [u8],
    slots: usize,
    valid: bool,
}

impl Vocab {
    fn new(bytes: &'static [u8]) -> Self {
        let valid = bytes.get(..8) == Some(VOCAB_MAGIC.as_slice()) && read_u32(bytes, 8) == Some(1);
        let slots = read_u32(bytes, 12).unwrap_or(0) as usize;
        Self {
            bytes,
            slots,
            valid,
        }
    }

    #[inline]
    fn hash_bytes(bytes: &[u8], start: usize, end: usize, continuation: bool) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        if continuation {
            hash ^= b'#' as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= b'#' as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let mut index = start;
        while index < end {
            let mut byte = bytes[index];
            if byte.is_ascii_uppercase() {
                byte += b'a' - b'A';
            }
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            index += 1;
        }
        let value = hash ^ 0x9e3779b97f4a7c15;
        if value == 0 {
            1
        } else {
            value
        }
    }

    #[inline]
    fn hash_exact(value: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in value {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let value = hash ^ 0x9e3779b97f4a7c15;
        if value == 0 {
            1
        } else {
            value
        }
    }

    fn entry(&self, slot: usize) -> Option<(u64, u32, usize, usize)> {
        if !self.valid || self.slots == 0 || slot >= self.slots {
            return None;
        }
        let offset = VOCAB_HEADER_SIZE.checked_add(slot.checked_mul(VOCAB_RECORD_SIZE)?)?;
        let hash = read_u64(self.bytes, offset)?;
        if hash == 0 {
            return Some((0, 0, 0, 0));
        }
        let id = read_u32(self.bytes, offset + 8)?;
        let token_offset = read_u32(self.bytes, offset + 12)? as usize;
        let token_len = read_u16(self.bytes, offset + 16)? as usize;
        Some((hash, id, token_offset, token_len))
    }

    fn matches_piece(
        &self,
        token_offset: usize,
        token_len: usize,
        bytes: &[u8],
        start: usize,
        end: usize,
        continuation: bool,
    ) -> bool {
        let expected_len = end.saturating_sub(start) + if continuation { 2 } else { 0 };
        if token_len != expected_len {
            return false;
        }
        let Some(token) = self.bytes.get(token_offset..token_offset + token_len) else {
            return false;
        };
        let mut token_index = 0;
        if continuation {
            if token.first() != Some(&b'#') || token.get(1) != Some(&b'#') {
                return false;
            }
            token_index = 2;
        }
        let mut input_index = start;
        while input_index < end {
            let mut byte = bytes[input_index];
            if byte.is_ascii_uppercase() {
                byte += b'a' - b'A';
            }
            if token.get(token_index) != Some(&byte) {
                return false;
            }
            token_index += 1;
            input_index += 1;
        }
        true
    }

    fn lookup_piece(
        &self,
        bytes: &[u8],
        start: usize,
        end: usize,
        continuation: bool,
    ) -> Option<u32> {
        if start >= end || end > bytes.len() || !self.valid {
            return None;
        }
        let hash = Self::hash_bytes(bytes, start, end, continuation);
        let mut slot = (hash as usize) % self.slots.max(1);
        let mut probes = 0;
        while probes < self.slots {
            let (stored_hash, id, token_offset, token_len) = self.entry(slot)?;
            if stored_hash == 0 {
                return None;
            }
            if stored_hash == hash
                && self.matches_piece(token_offset, token_len, bytes, start, end, continuation)
            {
                return Some(id);
            }
            slot = (slot + 1) % self.slots;
            probes += 1;
        }
        None
    }

    fn lookup_exact(&self, value: &[u8]) -> Option<u32> {
        if !self.valid || self.slots == 0 {
            return None;
        }
        let hash = Self::hash_exact(value);
        let mut slot = (hash as usize) % self.slots;
        let mut probes = 0;
        while probes < self.slots {
            let (stored_hash, id, token_offset, token_len) = self.entry(slot)?;
            if stored_hash == 0 {
                return None;
            }
            if stored_hash == hash
                && token_len == value.len()
                && self.bytes.get(token_offset..token_offset + token_len) == Some(value)
            {
                return Some(id);
            }
            slot = (slot + 1) % self.slots;
            probes += 1;
        }
        None
    }
}

#[derive(Clone, Copy)]
struct TokenIds {
    ids: [u32; MAX_SEQ],
    len: usize,
}

impl TokenIds {
    const fn empty() -> Self {
        Self {
            ids: [0; MAX_SEQ],
            len: 0,
        }
    }

    fn push(&mut self, id: u32) -> bool {
        if self.len >= MAX_SEQ {
            return false;
        }
        self.ids[self.len] = id;
        self.len += 1;
        true
    }
}

#[inline]
fn decode_codepoint(bytes: &[u8], index: usize) -> (u32, usize) {
    let first = bytes.get(index).copied().unwrap_or(0);
    let width = utf8_width(first);
    if width == 1 || index.saturating_add(width) > bytes.len() {
        return (first as u32, 1);
    }
    let mut value = (first & (0x7f >> width)) as u32;
    let mut offset = 1;
    while offset < width {
        let byte = bytes[index + offset];
        if byte & 0xc0 != 0x80 {
            return (first as u32, 1);
        }
        value = (value << 6) | (byte & 0x3f) as u32;
        offset += 1;
    }
    (value, width)
}

fn normalize_word(bytes: &[u8], start: usize, end: usize, output: &mut [u8; 256]) -> usize {
    let mut input = start;
    let mut length = 0;
    while input < end {
        let (value, width) = decode_codepoint(bytes, input);
        if value <= 0x7f {
            if length >= output.len() {
                return 0;
            }
            let mut byte = value as u8;
            if byte.is_ascii_uppercase() {
                byte += b'a' - b'A';
            }
            output[length] = byte;
            length += 1;
        } else {
            let next = input.saturating_add(width).min(end);
            if length.saturating_add(next - input) > output.len() {
                return 0;
            }
            output[length..length + next - input].copy_from_slice(&bytes[input..next]);
            length += next - input;
        }
        input = input.saturating_add(width).min(end);
    }
    length
}

#[inline]
fn utf8_width(byte: u8) -> usize {
    if byte < 0x80 {
        1
    } else if byte & 0xe0 == 0xc0 {
        2
    } else if byte & 0xf0 == 0xe0 {
        3
    } else if byte & 0xf8 == 0xf0 {
        4
    } else {
        1
    }
}

#[inline]
fn previous_boundary(bytes: &[u8], start: usize, end: usize) -> usize {
    if end <= start {
        return start;
    }
    let mut position = end - 1;
    while position > start && bytes[position] & 0xc0 == 0x80 {
        position -= 1;
    }
    position
}

fn emit_piece_bytes(
    vocab: &Vocab,
    bytes: &[u8],
    start: usize,
    end: usize,
    output: &mut TokenIds,
) -> bool {
    if start >= end {
        return true;
    }
    let unknown = vocab.lookup_exact(b"[UNK]").unwrap_or(100);
    let mut pieces = [0u32; 64];
    let mut piece_count = 0;
    let mut cursor = start;
    while cursor < end {
        let mut candidate = end;
        let mut found = None;
        while candidate > cursor {
            if let Some(id) = vocab.lookup_piece(bytes, cursor, candidate, cursor > start) {
                found = Some((candidate, id));
                break;
            }
            let next = previous_boundary(bytes, cursor, candidate);
            if next == candidate {
                break;
            }
            candidate = next;
        }
        let Some((next_cursor, id)) = found else {
            return output.push(unknown);
        };
        if piece_count >= pieces.len() {
            return output.push(unknown);
        }
        pieces[piece_count] = id;
        piece_count += 1;
        cursor = next_cursor;
    }
    let mut index = 0;
    while index < piece_count {
        if !output.push(pieces[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn emit_word(vocab: &Vocab, bytes: &[u8], start: usize, end: usize, output: &mut TokenIds) -> bool {
    let mut normalized = [0u8; 256];
    let length = normalize_word(bytes, start, end, &mut normalized);
    if length == 0 {
        return output.push(vocab.lookup_exact(b"[UNK]").unwrap_or(100));
    }
    emit_piece_bytes(vocab, &normalized, 0, length, output)
}

fn tokenize(vocab: &Vocab, bytes: &[u8]) -> TokenIds {
    let mut output = TokenIds::empty();
    let cls = vocab.lookup_exact(b"[CLS]").unwrap_or(101);
    let sep = vocab.lookup_exact(b"[SEP]").unwrap_or(102);
    if !output.push(cls) {
        return output;
    }
    let Ok(text) = core::str::from_utf8(bytes) else {
        let _ = output.push(sep);
        return output;
    };
    let mut word_start = None;
    for (offset, character) in text.char_indices() {
        if character.is_alphanumeric() {
            if word_start.is_none() {
                word_start = Some(offset);
            }
        } else if let Some(start) = word_start.take() {
            if !emit_word(vocab, bytes, start, offset, &mut output) {
                break;
            }
        }
    }
    if let Some(start) = word_start {
        let _ = emit_word(vocab, bytes, start, bytes.len(), &mut output);
    }
    if !output.push(sep) && MAX_SEQ > 0 {
        output.ids[MAX_SEQ - 1] = sep;
    }
    output
}

struct Scratch {
    hidden: [f32; MAX_SEQ * EMBED_DIM],
    query: [f32; MAX_SEQ * EMBED_DIM],
    key: [f32; MAX_SEQ * EMBED_DIM],
    value: [f32; MAX_SEQ * EMBED_DIM],
    context: [f32; MAX_SEQ * EMBED_DIM],
    scores: [f32; MAX_SEQ],
    intermediate: [f32; INTERMEDIATE],
}

impl Scratch {
    const fn empty() -> Self {
        Self {
            hidden: [0.0; MAX_SEQ * EMBED_DIM],
            query: [0.0; MAX_SEQ * EMBED_DIM],
            key: [0.0; MAX_SEQ * EMBED_DIM],
            value: [0.0; MAX_SEQ * EMBED_DIM],
            context: [0.0; MAX_SEQ * EMBED_DIM],
            scores: [0.0; MAX_SEQ],
            intermediate: [0.0; INTERMEDIATE],
        }
    }
}

struct ScratchCell(UnsafeCell<Scratch>);
unsafe impl Sync for ScratchCell {}
static SCRATCH: ScratchCell = ScratchCell(UnsafeCell::new(Scratch::empty()));
static SCRATCH_LOCK: AtomicBool = AtomicBool::new(false);

struct ScratchGuard;

impl ScratchGuard {
    fn acquire() -> Self {
        while SCRATCH_LOCK
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        Self
    }
}

impl Drop for ScratchGuard {
    fn drop(&mut self) {
        SCRATCH_LOCK.store(false, Ordering::Release);
    }
}

fn dense_row(store: &WeightStore, id: u16, input: &[f32], output: &mut [f32]) -> bool {
    let Some(record) = store.find(id) else {
        output.fill(0.0);
        return false;
    };
    if record.kind != KIND_MATRIX_INT8 || record.cols > input.len() || record.rows > output.len() {
        output.fill(0.0);
        return false;
    }
    let mut row = 0;
    while row < record.rows {
        let mut sum = 0.0f32;
        let scale_offset = record.scale_offset.saturating_add(row.saturating_mul(4));
        let scale = read_f32(store.bytes, scale_offset).unwrap_or(0.0);
        let mut column = 0;
        while column < record.cols {
            let index = record
                .data_offset
                .saturating_add(row.saturating_mul(record.cols))
                .saturating_add(column);
            let quantized = store.bytes.get(index).copied().unwrap_or(0) as i8 as f32;
            sum += (quantized * scale) * input[column];
            column += 1;
        }
        output[row] = math::finite_or_zero(sum + store.bias_value(record, row));
        row += 1;
    }
    true
}

fn layer_norm(
    store: &WeightStore,
    weight_id: u16,
    bias_id: u16,
    input: &[f32],
    output: &mut [f32],
) -> bool {
    let Some(weight) = store.find(weight_id) else {
        output.fill(0.0);
        return false;
    };
    let Some(bias) = store.find(bias_id) else {
        output.fill(0.0);
        return false;
    };
    if input.len() < EMBED_DIM || output.len() < EMBED_DIM {
        return false;
    }
    let mut mean = 0.0f32;
    let mut index = 0;
    while index < EMBED_DIM {
        mean += input[index];
        index += 1;
    }
    mean /= EMBED_DIM as f32;
    let mut variance = 0.0f32;
    index = 0;
    while index < EMBED_DIM {
        let delta = input[index] - mean;
        variance += delta * delta;
        index += 1;
    }
    variance /= EMBED_DIM as f32;
    let inverse = 1.0 / libm::sqrtf(variance + LN_EPSILON);
    index = 0;
    while index < EMBED_DIM {
        let normalized = (input[index] - mean) * inverse;
        output[index] = math::finite_or_zero(
            normalized * store.vector_value(weight, index) + store.vector_value(bias, index),
        );
        index += 1;
    }
    true
}

#[inline]
fn gelu(value: f32) -> f32 {
    const C: f32 = 0.797_884_6;
    math::finite_or_zero(
        0.5 * value * (1.0 + libm::tanhf(C * (value + 0.044_715 * value * value * value))),
    )
}

fn attention(store: &WeightStore, layer: usize, length: usize, scratch: &mut Scratch) -> bool {
    let mut token = 0;
    let mut valid = true;
    while token < length {
        let offset = token * EMBED_DIM;
        valid &= dense_row(
            store,
            layer_id(layer, PART_QUERY),
            &scratch.hidden[offset..offset + EMBED_DIM],
            &mut scratch.query[offset..offset + EMBED_DIM],
        );
        valid &= dense_row(
            store,
            layer_id(layer, PART_KEY),
            &scratch.hidden[offset..offset + EMBED_DIM],
            &mut scratch.key[offset..offset + EMBED_DIM],
        );
        valid &= dense_row(
            store,
            layer_id(layer, PART_VALUE),
            &scratch.hidden[offset..offset + EMBED_DIM],
            &mut scratch.value[offset..offset + EMBED_DIM],
        );
        token += 1;
    }

    scratch.context[..length * EMBED_DIM].fill(0.0);
    token = 0;
    while token < length {
        let mut head = 0;
        while head < HEADS {
            let query_offset = token * EMBED_DIM + head * HEAD_DIM;
            let mut key_token = 0;
            while key_token < length {
                let key_offset = key_token * EMBED_DIM + head * HEAD_DIM;
                let mut dot = 0.0f32;
                let mut dimension = 0;
                while dimension < HEAD_DIM {
                    dot += scratch.query[query_offset + dimension]
                        * scratch.key[key_offset + dimension];
                    dimension += 1;
                }
                scratch.scores[key_token] = dot * SCALE;
                key_token += 1;
            }
            let mut maximum = scratch.scores[0];
            key_token = 1;
            while key_token < length {
                if scratch.scores[key_token] > maximum {
                    maximum = scratch.scores[key_token];
                }
                key_token += 1;
            }
            let mut sum = 0.0f32;
            key_token = 0;
            while key_token < length {
                scratch.scores[key_token] = libm::expf(scratch.scores[key_token] - maximum);
                sum += scratch.scores[key_token];
                key_token += 1;
            }
            let inverse = if sum > 1.0e-12 { 1.0 / sum } else { 0.0 };
            let context_offset = token * EMBED_DIM + head * HEAD_DIM;
            let mut dimension = 0;
            while dimension < HEAD_DIM {
                let mut weighted = 0.0f32;
                key_token = 0;
                while key_token < length {
                    let weight = scratch.scores[key_token] * inverse;
                    weighted +=
                        weight * scratch.value[key_token * EMBED_DIM + head * HEAD_DIM + dimension];
                    key_token += 1;
                }
                scratch.context[context_offset + dimension] = math::finite_or_zero(weighted);
                dimension += 1;
            }
            head += 1;
        }
        token += 1;
    }

    token = 0;
    while token < length {
        let offset = token * EMBED_DIM;
        valid &= dense_row(
            store,
            layer_id(layer, PART_ATTENTION_OUTPUT),
            &scratch.context[offset..offset + EMBED_DIM],
            &mut scratch.query[offset..offset + EMBED_DIM],
        );
        let mut dimension = 0;
        while dimension < EMBED_DIM {
            scratch.query[offset + dimension] += scratch.hidden[offset + dimension];
            dimension += 1;
        }
        valid &= layer_norm(
            store,
            layer_id(layer, PART_ATTENTION_LN_WEIGHT),
            layer_id(layer, PART_ATTENTION_LN_BIAS),
            &scratch.query[offset..offset + EMBED_DIM],
            &mut scratch.hidden[offset..offset + EMBED_DIM],
        );
        token += 1;
    }
    valid
}

fn feed_forward(store: &WeightStore, layer: usize, length: usize, scratch: &mut Scratch) -> bool {
    let mut token = 0;
    let mut valid = true;
    while token < length {
        let offset = token * EMBED_DIM;
        valid &= dense_row(
            store,
            layer_id(layer, PART_INTERMEDIATE),
            &scratch.hidden[offset..offset + EMBED_DIM],
            &mut scratch.intermediate,
        );
        let mut index = 0;
        while index < INTERMEDIATE {
            scratch.intermediate[index] = gelu(scratch.intermediate[index]);
            index += 1;
        }
        valid &= dense_row(
            store,
            layer_id(layer, PART_OUTPUT),
            &scratch.intermediate,
            &mut scratch.context[offset..offset + EMBED_DIM],
        );
        index = 0;
        while index < EMBED_DIM {
            scratch.context[offset + index] += scratch.hidden[offset + index];
            index += 1;
        }
        valid &= layer_norm(
            store,
            layer_id(layer, PART_OUTPUT_LN_WEIGHT),
            layer_id(layer, PART_OUTPUT_LN_BIAS),
            &scratch.context[offset..offset + EMBED_DIM],
            &mut scratch.hidden[offset..offset + EMBED_DIM],
        );
        token += 1;
    }
    valid
}

fn embed_tokens(store: &WeightStore, tokens: &TokenIds, scratch: &mut Scratch) -> bool {
    let Some(word) = store.find(ID_WORD_EMBEDDINGS) else {
        return false;
    };
    let Some(position) = store.find(ID_POSITION_EMBEDDINGS) else {
        return false;
    };
    let Some(token_type) = store.find(ID_TOKEN_TYPE_EMBEDDINGS) else {
        return false;
    };
    let mut token = 0;
    while token < tokens.len {
        let offset = token * EMBED_DIM;
        let mut dimension = 0;
        while dimension < EMBED_DIM {
            scratch.hidden[offset + dimension] =
                store.matrix_value(word, tokens.ids[token] as usize, dimension)
                    + store.matrix_value(position, token, dimension)
                    + store.matrix_value(token_type, 0, dimension);
            dimension += 1;
        }
        token += 1;
    }
    let mut valid = true;
    token = 0;
    while token < tokens.len {
        let offset = token * EMBED_DIM;
        valid &= layer_norm(
            store,
            ID_EMBEDDING_LN_WEIGHT,
            ID_EMBEDDING_LN_BIAS,
            &scratch.hidden[offset..offset + EMBED_DIM],
            &mut scratch.context[offset..offset + EMBED_DIM],
        );
        scratch.hidden[offset..offset + EMBED_DIM]
            .copy_from_slice(&scratch.context[offset..offset + EMBED_DIM]);
        token += 1;
    }
    valid
}

fn normalize(values: &mut [f32; EMBED_DIM]) {
    let mut sum = 0.0f32;
    let mut index = 0;
    while index < EMBED_DIM {
        sum += values[index] * values[index];
        index += 1;
    }
    let norm = libm::sqrtf(sum);
    if norm <= 1.0e-12 || norm.is_nan() || norm.is_infinite() {
        *values = [0.0; EMBED_DIM];
        return;
    }
    index = 0;
    while index < EMBED_DIM {
        values[index] = math::finite_or_zero(values[index] / norm);
        index += 1;
    }
}

pub fn encode(bytes: &[u8]) -> Embedding {
    let _guard = ScratchGuard::acquire();
    let store = WeightStore::new(WEIGHTS);
    let vocab = Vocab::new(VOCAB);
    let tokens = tokenize(&vocab, bytes);
    if tokens.len == 0 || !store.valid || !vocab.valid {
        return Embedding::zero();
    }
    let scratch = unsafe { &mut *SCRATCH.0.get() };
    if !embed_tokens(&store, &tokens, scratch) {
        return Embedding::zero();
    }
    let mut layer = 0;
    while layer < 6 {
        if !attention(&store, layer, tokens.len, scratch)
            || !feed_forward(&store, layer, tokens.len, scratch)
        {
            return Embedding::zero();
        }
        layer += 1;
    }
    let mut output = Embedding::zero();
    let mut dimension = 0;
    while dimension < EMBED_DIM {
        let mut total = 0.0f32;
        let mut token = 0;
        while token < tokens.len {
            total += scratch.hidden[token * EMBED_DIM + dimension];
            token += 1;
        }
        output.values[dimension] = total / tokens.len as f32;
        dimension += 1;
    }
    normalize(&mut output.values);
    output
}

pub fn debug_token_ids(bytes: &[u8]) -> ([u32; MAX_SEQ], usize) {
    let vocab = Vocab::new(VOCAB);
    let tokens = tokenize(&vocab, bytes);
    (tokens.ids, tokens.len)
}

// The salience source retains an optional transformer branch used by other
// intents. The WEATHER_CHECK build leaves that branch disabled, but keeping a
// small adapter here makes `--all-features` compile and gives that branch a
// deterministic full-encoder fallback when it is explicitly requested.
#[cfg(feature = "minilm")]
pub struct Similarities {
    pub ca: f32,
    pub cb: f32,
    pub cq: f32,
    pub c2: f32,
    pub c4: f32,
}

#[cfg(feature = "minilm")]
pub fn embed_sims(question: &[u8], ground_truth: &[u8], answer: &[u8]) -> Similarities {
    let question_embedding = encode(question);
    let ground_truth_embedding = encode(ground_truth);
    let answer_embedding = encode(answer);
    let ca = crate::embed::cosine(&question_embedding, &ground_truth_embedding);
    let cb = crate::embed::cosine(&ground_truth_embedding, &answer_embedding);
    let cq = crate::embed::cosine(&question_embedding, &answer_embedding);
    Similarities {
        ca,
        cb,
        cq,
        c2: cb,
        c4: cb,
    }
}

#[cfg(feature = "minilm")]
pub fn embed_cos_abq(question: &[u8], ground_truth: &[u8], answer: &[u8]) -> (f32, f32, f32) {
    let sims = embed_sims(question, ground_truth, answer);
    (sims.ca, sims.cb, sims.cq)
}
