//! Fixed-capacity, allocation-free tokenization for lexical signals.

pub const MAX_TOKENS: usize = 256;

#[derive(Clone, Copy)]
pub struct TokenBuffer {
    pub tokens: [u64; MAX_TOKENS],
    pub len: usize,
}

impl TokenBuffer {
    pub const fn empty() -> Self {
        Self {
            tokens: [0; MAX_TOKENS],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, token: u64) {
        if self.len < MAX_TOKENS {
            self.tokens[self.len] = token;
            self.len += 1;
        }
    }

    #[inline]
    pub fn count(&self, token: u64) -> usize {
        let mut count = 0;
        let mut index = 0;
        while index < self.len {
            if self.tokens[index] == token {
                count += 1;
            }
            index += 1;
        }
        count
    }
}

#[inline]
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
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
fn hash_word(bytes: &[u8], start: usize, end: usize) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mut index = start;
    while index < end {
        hash ^= lower_ascii(bytes[index]) as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
        index += 1;
    }
    // Keep the empty hash distinct from the sentinel zero value.
    hash ^ 0x9e3779b97f4a7c15u64
}

pub fn tokenize(bytes: &[u8]) -> TokenBuffer {
    let mut output = TokenBuffer::empty();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !is_word_byte(bytes[index]) {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let start = index;
        while index < bytes.len() && is_word_byte(bytes[index]) {
            index += 1;
        }
        output.push(hash_word(bytes, start, index));
    }
    output
}

pub fn normalized_equal(left: &[u8], right: &[u8]) -> bool {
    let left = tokenize(left);
    let right = tokenize(right);
    if left.len != right.len {
        return false;
    }
    let mut index = 0;
    while index < left.len {
        if left.tokens[index] != right.tokens[index] {
            return false;
        }
        index += 1;
    }
    true
}
