//! Deterministic typed-fact checks for WEATHER_CHECK / WEATHER_FORECAST text.
//!
//! The embedding and BM25 signals remain the primary scorer. This module adds
//! a small, bounded correction for facts that text similarity cannot verify:
//! temperatures, percentages, wind, precipitation, condition polarity, and
//! the requested location. It is deliberately allocation-free and uses fixed
//! arrays so compiler and interpreter runtimes see the same operations.

use crate::math;

const MAX_FACTS: usize = 48;
const KIND_TEMPERATURE: u8 = 1;
const KIND_HUMIDITY: u8 = 2;
const KIND_CLOUD: u8 = 3;
const KIND_WIND_SPEED: u8 = 4;
const KIND_WIND_DIRECTION: u8 = 5;
const KIND_PRECIPITATION: u8 = 6;
const KIND_PRESSURE: u8 = 7;
const KIND_PROBABILITY: u8 = 8;

const QUAL_NONE: u8 = 0;
const QUAL_HIGH: u8 = 1;
const QUAL_LOW: u8 = 2;
const QUAL_APPARENT: u8 = 3;

const UNIT_NONE: u8 = 0;
const UNIT_CELSIUS: u8 = 1;
const UNIT_FAHRENHEIT: u8 = 2;
const UNIT_PERCENT: u8 = 3;
const UNIT_MM: u8 = 4;
const UNIT_SPEED: u8 = 5;
const UNIT_DEGREE: u8 = 6;
const UNIT_PRESSURE: u8 = 7;

#[derive(Clone, Copy)]
struct Fact {
    kind: u8,
    qualifier: u8,
    unit: u8,
    value: f32,
}

impl Fact {
    const EMPTY: Self = Self {
        kind: 0,
        qualifier: QUAL_NONE,
        unit: UNIT_NONE,
        value: 0.0,
    };

    #[inline]
    fn key(self) -> u16 {
        ((self.kind as u16) << 8) | self.qualifier as u16
    }

    #[inline]
    fn temperature_c(self) -> f32 {
        if self.unit == UNIT_FAHRENHEIT {
            self.value * 5.0 / 9.0 - 32.0 * 5.0 / 9.0
        } else {
            self.value
        }
    }
}

#[derive(Clone, Copy)]
struct FactBuffer {
    values: [Fact; MAX_FACTS],
    len: usize,
}

impl FactBuffer {
    const fn empty() -> Self {
        Self {
            values: [Fact::EMPTY; MAX_FACTS],
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, fact: Fact) {
        if fact.kind != 0 && self.len < MAX_FACTS && fact.value.is_finite() {
            self.values[self.len] = fact;
            self.len += 1;
        }
    }
}

#[inline]
fn lower(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + (b'a' - b'A')
    } else {
        byte
    }
}

#[inline]
fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

#[inline]
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

fn has_word(bytes: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    let mut start = 0;
    while start + needle.len() <= bytes.len() {
        let mut equal = true;
        let mut index = 0;
        while index < needle.len() {
            if lower(bytes[start + index]) != needle[index] {
                equal = false;
                break;
            }
            index += 1;
        }
        if equal {
            let before_ok = start == 0 || !is_word(bytes[start - 1]);
            let after = start + needle.len();
            let after_ok = after == bytes.len() || !is_word(bytes[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        start += 1;
    }
    false
}

fn has_any_word(bytes: &[u8], words: &[&[u8]]) -> bool {
    let mut index = 0;
    while index < words.len() {
        if has_word(bytes, words[index]) {
            return true;
        }
        index += 1;
    }
    false
}

fn window(bytes: &[u8], start: usize, end: usize) -> (usize, usize) {
    let left = start.saturating_sub(56);
    let right = core::cmp::min(bytes.len(), end.saturating_add(40));
    (left, right)
}

fn skip_space(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn parse_number(bytes: &[u8], start: usize) -> Option<(usize, f32)> {
    let mut index = start;
    let mut negative = false;
    if index < bytes.len() && (bytes[index] == b'-' || bytes[index] == b'+') {
        negative = bytes[index] == b'-';
        index += 1;
    }
    let mut whole = 0.0f32;
    let mut digits = 0;
    while index < bytes.len() && is_digit(bytes[index]) {
        whole = whole * 10.0 + (bytes[index] - b'0') as f32;
        index += 1;
        digits += 1;
    }
    let mut fraction = 0.0f32;
    let mut scale = 0.1f32;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && is_digit(bytes[index]) {
            fraction += (bytes[index] - b'0') as f32 * scale;
            scale *= 0.1;
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    let value = if negative {
        -(whole + fraction)
    } else {
        whole + fraction
    };
    if value.is_finite() {
        Some((index, value))
    } else {
        None
    }
}

fn unit_and_kind(bytes: &[u8], start: usize, end: usize, context: &[u8]) -> (u8, u8) {
    let mut after = skip_space(bytes, end);
    let mut unit = UNIT_NONE;
    if after < bytes.len() && bytes[after] == b'%' {
        unit = UNIT_PERCENT;
    } else if after + 1 < bytes.len() && bytes[after] == 0xc2 && bytes[after + 1] == 0xb0 {
        unit = UNIT_DEGREE;
        after += 2;
        if after < bytes.len() && (lower(bytes[after]) == b'c' || lower(bytes[after]) == b'f') {
            unit = if lower(bytes[after]) == b'c' {
                UNIT_CELSIUS
            } else {
                UNIT_FAHRENHEIT
            };
        }
    } else if after < bytes.len() && (lower(bytes[after]) == b'c' || lower(bytes[after]) == b'f') {
        let next = after + 1;
        if next == bytes.len() || !is_word(bytes[next]) {
            unit = if lower(bytes[after]) == b'c' {
                UNIT_CELSIUS
            } else {
                UNIT_FAHRENHEIT
            };
        }
    } else if after + 1 < bytes.len()
        && lower(bytes[after]) == b'm'
        && lower(bytes[after + 1]) == b'm'
    {
        unit = UNIT_MM;
    } else if after + 3 < bytes.len()
        && lower(bytes[after]) == b'k'
        && lower(bytes[after + 1]) == b'm'
        && bytes[after + 2] == b'/'
        && lower(bytes[after + 3]) == b'h'
    {
        unit = UNIT_SPEED;
    } else if after + 3 < bytes.len()
        && lower(bytes[after]) == b'm'
        && lower(bytes[after + 1]) == b'p'
        && lower(bytes[after + 2]) == b'h'
    {
        unit = UNIT_SPEED;
    } else if after + 2 < bytes.len()
        && lower(bytes[after]) == b'h'
        && lower(bytes[after + 1]) == b'p'
        && lower(bytes[after + 2]) == b'a'
    {
        unit = UNIT_PRESSURE;
    }

    let temperature_words = [
        b"temperature".as_slice(),
        b"temp".as_slice(),
        b"feeling".as_slice(),
        b"feels".as_slice(),
        b"heat".as_slice(),
        b"dewpoint".as_slice(),
    ];
    let humidity_words = [b"humidity".as_slice(), b"humid".as_slice()];
    let cloud_words = [b"cloud".as_slice(), b"overcast".as_slice()];
    let wind_words = [
        b"wind".as_slice(),
        b"gust".as_slice(),
        b"bearing".as_slice(),
    ];
    let precipitation_words = [
        b"precipitation".as_slice(),
        b"rain".as_slice(),
        b"snow".as_slice(),
        b"drizzle".as_slice(),
    ];
    let pressure_words = [b"pressure".as_slice(), b"barometric".as_slice()];
    let probability_words = [
        b"chance".as_slice(),
        b"probability".as_slice(),
        b"likelihood".as_slice(),
    ];

    let kind = if unit == UNIT_CELSIUS
        || unit == UNIT_FAHRENHEIT
        || has_any_word(context, &temperature_words)
    {
        KIND_TEMPERATURE
    } else if unit == UNIT_PERCENT && has_any_word(context, &humidity_words) {
        KIND_HUMIDITY
    } else if unit == UNIT_PERCENT && has_any_word(context, &cloud_words) {
        KIND_CLOUD
    } else if unit == UNIT_PERCENT && has_any_word(context, &probability_words) {
        KIND_PROBABILITY
    } else if (unit == UNIT_MM || has_any_word(context, &precipitation_words))
        && has_any_word(context, &precipitation_words)
    {
        KIND_PRECIPITATION
    } else if unit == UNIT_SPEED || has_any_word(context, &wind_words) {
        KIND_WIND_SPEED
    } else if unit == UNIT_DEGREE && has_any_word(context, &wind_words) {
        KIND_WIND_DIRECTION
    } else if unit == UNIT_PRESSURE || has_any_word(context, &pressure_words) {
        KIND_PRESSURE
    } else {
        0
    };
    let qualifier = if has_any_word(context, &[b"high", b"maximum", b"max"]) {
        QUAL_HIGH
    } else if has_any_word(context, &[b"low", b"minimum", b"min"]) {
        QUAL_LOW
    } else if has_any_word(context, &[b"apparent", b"feels", b"feeling"]) {
        QUAL_APPARENT
    } else {
        QUAL_NONE
    };
    let _ = start;
    (kind, qualifier)
}

fn collect_facts(bytes: &[u8]) -> FactBuffer {
    let mut facts = FactBuffer::empty();
    let mut index = 0;
    while index < bytes.len() {
        let starts_number = is_digit(bytes[index])
            || ((bytes[index] == b'-' || bytes[index] == b'+')
                && index + 1 < bytes.len()
                && (is_digit(bytes[index + 1]) || bytes[index + 1] == b'.'))
            || (bytes[index] == b'.' && index + 1 < bytes.len() && is_digit(bytes[index + 1]));
        if !starts_number {
            index += 1;
            continue;
        }
        let number_start = index;
        let Some((number_end, value)) = parse_number(bytes, index) else {
            index += 1;
            continue;
        };
        let (left, right) = window(bytes, number_start, number_end);
        let (kind, qualifier) = unit_and_kind(bytes, number_start, number_end, &bytes[left..right]);
        if kind != 0 {
            let mut after = skip_space(bytes, number_end);
            let unit = if after < bytes.len() && bytes[after] == b'%' {
                UNIT_PERCENT
            } else if after + 1 < bytes.len() && bytes[after] == 0xc2 && bytes[after + 1] == 0xb0 {
                after += 2;
                if after < bytes.len() && lower(bytes[after]) == b'f' {
                    UNIT_FAHRENHEIT
                } else if after < bytes.len() && lower(bytes[after]) == b'c' {
                    UNIT_CELSIUS
                } else {
                    UNIT_DEGREE
                }
            } else if after + 1 < bytes.len()
                && lower(bytes[after]) == b'm'
                && lower(bytes[after + 1]) == b'm'
            {
                UNIT_MM
            } else if after + 3 < bytes.len()
                && lower(bytes[after]) == b'k'
                && lower(bytes[after + 1]) == b'm'
                && bytes[after + 2] == b'/'
                && lower(bytes[after + 3]) == b'h'
            {
                UNIT_SPEED
            } else if after + 2 < bytes.len()
                && lower(bytes[after]) == b'm'
                && lower(bytes[after + 1]) == b'p'
                && lower(bytes[after + 2]) == b'h'
            {
                UNIT_SPEED
            } else if after + 2 < bytes.len()
                && lower(bytes[after]) == b'h'
                && lower(bytes[after + 1]) == b'p'
                && lower(bytes[after + 2]) == b'a'
            {
                UNIT_PRESSURE
            } else {
                UNIT_NONE
            };
            facts.push(Fact {
                kind,
                qualifier,
                unit,
                value,
            });
        }
        index = if number_end > index {
            number_end
        } else {
            index + 1
        };
    }
    facts
}

fn weather_signal(bytes: &[u8]) -> bool {
    has_any_word(
        bytes,
        &[
            b"weather",
            b"forecast",
            b"temperature",
            b"humidity",
            b"precipitation",
            b"wind",
            b"rain",
            b"snow",
            b"cloud",
            b"overcast",
            b"thunderstorm",
            b"celsius",
            b"fahrenheit",
        ],
    )
}

fn condition_polarity(bytes: &[u8]) -> u8 {
    if has_any_word(
        bytes,
        &[
            b"rain",
            b"rainy",
            b"drizzle",
            b"shower",
            b"snow",
            b"hail",
            b"thunderstorm",
        ],
    ) {
        2
    } else if has_any_word(bytes, &[b"clear", b"sunny", b"sunshine"]) {
        1
    } else if has_any_word(bytes, &[b"overcast", b"cloudy", b"cloud", b"fog", b"mist"]) {
        3
    } else {
        0
    }
}

fn score_fact(reference: Fact, candidate: Fact) -> f32 {
    let difference = match reference.kind {
        KIND_TEMPERATURE => (reference.temperature_c() - candidate.temperature_c()).abs(),
        KIND_HUMIDITY | KIND_CLOUD | KIND_PROBABILITY => (reference.value - candidate.value).abs(),
        KIND_WIND_SPEED => (reference.value - candidate.value).abs(),
        KIND_WIND_DIRECTION => {
            let delta = (reference.value - candidate.value).abs();
            if delta > 180.0 {
                360.0 - delta
            } else {
                delta
            }
        }
        KIND_PRECIPITATION => (reference.value - candidate.value).abs(),
        KIND_PRESSURE => (reference.value - candidate.value).abs(),
        _ => 0.0,
    };
    if !difference.is_finite() {
        return 0.0;
    }
    match reference.kind {
        KIND_TEMPERATURE => {
            if difference <= 0.5 {
                0.025
            } else if difference <= 2.0 {
                0.0
            } else {
                -0.08
            }
        }
        KIND_HUMIDITY | KIND_CLOUD => {
            if difference <= 5.0 {
                0.018
            } else {
                -0.06
            }
        }
        KIND_PROBABILITY => {
            if difference <= 10.0 {
                0.008
            } else {
                -0.03
            }
        }
        KIND_WIND_SPEED => {
            if difference <= 2.0 {
                0.012
            } else {
                -0.035
            }
        }
        KIND_WIND_DIRECTION => {
            if difference <= 22.5 {
                0.012
            } else {
                -0.05
            }
        }
        KIND_PRECIPITATION => {
            if difference <= 1.0 {
                0.012
            } else {
                -0.035
            }
        }
        KIND_PRESSURE => {
            if difference <= 5.0 {
                0.008
            } else {
                -0.018
            }
        }
        _ => 0.0,
    }
}

fn pair_score(reference: FactBuffer, candidate: FactBuffer) -> f32 {
    let mut used = [false; MAX_FACTS];
    let mut total = 0.0;
    let mut reference_index = 0;
    while reference_index < reference.len {
        let fact = reference.values[reference_index];
        let mut found = None;
        let mut candidate_index = 0;
        while candidate_index < candidate.len {
            if !used[candidate_index] && candidate.values[candidate_index].key() == fact.key() {
                found = Some(candidate_index);
                break;
            }
            candidate_index += 1;
        }
        if let Some(index) = found {
            used[index] = true;
            total += score_fact(fact, candidate.values[index]);
        }
        reference_index += 1;
    }

    // Positional fallback is allowed only when the remaining counts are equal.
    let mut kind = 1;
    while kind <= 8 {
        let mut reference_count = 0;
        let mut candidate_count = 0;
        let mut index = 0;
        while index < reference.len {
            if reference.values[index].kind == kind {
                reference_count += 1;
            }
            index += 1;
        }
        index = 0;
        while index < candidate.len {
            if candidate.values[index].kind == kind && !used[index] {
                candidate_count += 1;
            }
            index += 1;
        }
        if reference_count == candidate_count && reference_count > 0 {
            let mut ref_index = 0;
            let mut cand_index = 0;
            while ref_index < reference.len {
                if reference.values[ref_index].kind == kind {
                    while cand_index < candidate.len
                        && (used[cand_index] || candidate.values[cand_index].kind != kind)
                    {
                        cand_index += 1;
                    }
                    if cand_index < candidate.len {
                        total +=
                            score_fact(reference.values[ref_index], candidate.values[cand_index]);
                        used[cand_index] = true;
                        cand_index += 1;
                    }
                }
                ref_index += 1;
            }
        }
        kind += 1;
    }
    math::finite_or_zero(total)
}

fn location_penalty(question: &[u8], reference: &[u8], candidate: &[u8]) -> f32 {
    // Keep this conservative: only enforce an explicit city/region token from
    // a question containing "weather ... in/at/for". Coordinate-only queries
    // have no lexical location to compare.
    let mut marker = 0;
    while marker + 3 <= question.len() {
        if lower(question[marker]) == b' '
            && lower(question[marker + 1]) == b'i'
            && lower(question[marker + 2]) == b'n'
            && (marker + 3 == question.len() || !is_word(question[marker + 3]))
        {
            break;
        }
        if lower(question[marker]) == b' '
            && lower(question[marker + 1]) == b'a'
            && lower(question[marker + 2]) == b't'
            && (marker + 3 == question.len() || !is_word(question[marker + 3]))
        {
            break;
        }
        if lower(question[marker]) == b' '
            && lower(question[marker + 1]) == b'f'
            && lower(question[marker + 2]) == b'o'
            && marker + 4 < question.len()
            && lower(question[marker + 3]) == b'r'
            && !is_word(question[marker + 4])
        {
            break;
        }
        marker += 1;
    }
    if marker + 3 >= question.len() {
        return 0.0;
    }
    let start = marker
        + 3
        + if lower(question[marker + 1]) == b'f' {
            1
        } else {
            0
        };
    let mut end = start;
    while end < question.len()
        && question[end] != b'?'
        && question[end] != b'!'
        && question[end] != b'\n'
    {
        end += 1;
    }
    let phrase = &question[start..end];
    let mut word_start = 0;
    while word_start < phrase.len() && !is_word(phrase[word_start]) {
        word_start += 1;
    }
    let mut word_end = word_start;
    while word_end < phrase.len() && is_word(phrase[word_end]) {
        word_end += 1;
    }
    if word_end <= word_start || word_end - word_start < 3 {
        return 0.0;
    }
    let expected = &phrase[word_start..word_end];
    if has_word(reference, expected) && !has_word(candidate, expected) {
        -0.12
    } else {
        0.0
    }
}

fn unit_consistency(facts: FactBuffer) -> f32 {
    let mut index = 0;
    while index < facts.len {
        if facts.values[index].kind == KIND_TEMPERATURE {
            let mut other = index + 1;
            while other < facts.len {
                if facts.values[other].kind == KIND_TEMPERATURE
                    && facts.values[other].qualifier == facts.values[index].qualifier
                    && facts.values[other].unit != facts.values[index].unit
                {
                    let c = facts.values[index].temperature_c();
                    let d = facts.values[other].temperature_c();
                    if (c - d).abs() > 1.0 {
                        return -0.10;
                    }
                }
                other += 1;
            }
        }
        index += 1;
    }
    0.0
}

/// Return a bounded typed-fact adjustment to add to the raw baseline score.
///
/// Unmatched reference facts intentionally contribute zero. This avoids
/// penalising a valid concise answer merely because the extractor found no
/// candidate counterpart.
pub fn adjustment(question: &[u8], reference: &[u8], candidate: &[u8]) -> f32 {
    if reference == candidate || !weather_signal(reference) || !weather_signal(candidate) {
        return 0.0;
    }
    let reference_facts = collect_facts(reference);
    let candidate_facts = collect_facts(candidate);
    let mut adjustment = pair_score(reference_facts, candidate_facts);
    adjustment += unit_consistency(candidate_facts);
    let reference_polarity = condition_polarity(reference);
    let candidate_polarity = condition_polarity(candidate);
    if reference_polarity != 0
        && candidate_polarity != 0
        && reference_polarity != candidate_polarity
    {
        adjustment -= 0.12;
    }
    adjustment += location_penalty(question, reference, candidate);
    if !adjustment.is_finite() {
        return 0.0;
    }
    if adjustment < -0.25 {
        -0.25
    } else if adjustment > 0.12 {
        0.12
    } else {
        adjustment
    }
}

#[cfg(test)]
mod tests {
    use super::adjustment;

    #[test]
    fn identical_weather_text_has_no_adjustment() {
        let text = b"The current temperature in Gujranwala is 34.2\xC2\xB0C (93.6\xC2\xB0F) with 41% humidity under partly cloudy skies.";
        assert_eq!(adjustment(b"weather in Gujranwala", text, text), 0.0);
    }

    #[test]
    fn wrong_temperature_and_condition_are_penalized() {
        let reference = b"The current temperature in Gujranwala is 34.2\xC2\xB0C (93.6\xC2\xB0F) with 41% humidity under partly cloudy skies.";
        let candidate = b"The current temperature in Gujranwala is 24.2\xC2\xB0C (75.6\xC2\xB0F) with 41% humidity under heavy rain skies.";
        assert!(adjustment(b"weather in Gujranwala", reference, candidate) < -0.1);
    }

    #[test]
    fn consistent_celsius_and_fahrenheit_are_not_penalized() {
        let reference =
            b"Weather in Lagos: 25.8\xC2\xB0C (78.4\xC2\xB0F), 77% humidity and 18.4 km/h wind.";
        let candidate =
            b"Weather in Lagos: 25.8\xC2\xB0C (78.4\xC2\xB0F), 77% humidity and 18.4 km/h wind.";
        assert_eq!(adjustment(b"weather in Lagos", reference, candidate), 0.0);
    }

    #[test]
    fn generic_non_weather_text_is_untouched() {
        assert_eq!(
            adjustment(
                b"boiling point",
                b"100 degrees Celsius",
                b"20 degrees Celsius"
            ),
            0.0
        );
    }
}
