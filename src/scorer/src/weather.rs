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
const UNIT_SPEED_MS: u8 = 5;
const UNIT_DEGREE: u8 = 6;
const UNIT_PRESSURE: u8 = 7;
const UNIT_SPEED_KMH: u8 = 8;
const UNIT_SPEED_MPH: u8 = 9;

const CONTEXT_SLOTS: usize = 3;

#[derive(Clone, Copy)]
struct Fact {
    kind: u8,
    qualifier: u8,
    unit: u8,
    value: f32,
    context: [u32; CONTEXT_SLOTS],
    context_len: u8,
}

impl Fact {
    const EMPTY: Self = Self {
        kind: 0,
        qualifier: QUAL_NONE,
        unit: UNIT_NONE,
        value: 0.0,
        context: [0; CONTEXT_SLOTS],
        context_len: 0,
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

    #[inline]
    fn wind_ms(self) -> f32 {
        match self.unit {
            UNIT_SPEED_KMH => self.value / 3.6,
            UNIT_SPEED_MPH => self.value * 0.44704,
            _ => self.value,
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
            if lower(bytes[start + index]) != lower(needle[index]) {
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

fn count_word(bytes: &[u8], needle: &[u8]) -> u8 {
    if needle.is_empty() || bytes.len() < needle.len() {
        return 0;
    }
    let mut count = 0u8;
    let mut start = 0;
    while start + needle.len() <= bytes.len() {
        let mut equal = true;
        let mut index = 0;
        while index < needle.len() {
            if lower(bytes[start + index]) != lower(needle[index]) {
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
                count = count.saturating_add(1);
            }
        }
        start += 1;
    }
    count
}

fn count_any_word(bytes: &[u8], words: &[&[u8]]) -> u8 {
    let mut count = 0u8;
    let mut index = 0;
    while index < words.len() {
        count = count.saturating_add(count_word(bytes, words[index]));
        index += 1;
    }
    count
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

fn token_equals(bytes: &[u8], needle: &[u8]) -> bool {
    if bytes.len() != needle.len() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        if lower(bytes[index]) != needle[index] {
            return false;
        }
        index += 1;
    }
    true
}

fn is_stopword(bytes: &[u8]) -> bool {
    let words = [
        b"a".as_slice(),
        b"an".as_slice(),
        b"and".as_slice(),
        b"as".as_slice(),
        b"at".as_slice(),
        b"by".as_slice(),
        b"for".as_slice(),
        b"from".as_slice(),
        b"in".as_slice(),
        b"is".as_slice(),
        b"of".as_slice(),
        b"on".as_slice(),
        b"or".as_slice(),
        b"the".as_slice(),
        b"to".as_slice(),
        b"with".as_slice(),
    ];
    let mut index = 0;
    while index < words.len() {
        if token_equals(bytes, words[index]) {
            return true;
        }
        index += 1;
    }
    false
}

fn is_unit_token(bytes: &[u8]) -> bool {
    let units = [
        b"c".as_slice(),
        b"f".as_slice(),
        b"hpa".as_slice(),
        b"km".as_slice(),
        b"m".as_slice(),
        b"mm".as_slice(),
        b"mph".as_slice(),
        b"s".as_slice(),
    ];
    let mut index = 0;
    while index < units.len() {
        if token_equals(bytes, units[index]) {
            return true;
        }
        index += 1;
    }
    false
}

fn token_hash(bytes: &[u8], start: usize, end: usize) -> u32 {
    let mut hash = 2_166_136_261u32;
    let mut index = start;
    while index < end {
        hash ^= lower(bytes[index]) as u32;
        hash = hash.wrapping_mul(16_777_619);
        index += 1;
    }
    hash
}

#[inline]
fn is_context_char(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// Extract at most two non-stopword tokens before a number and one after it.
/// The fixed token hashes are the context key used by the pairing pass; they
/// avoid positional guesses when a response contains several temperatures.
fn context_key(bytes: &[u8], number_start: usize, number_end: usize) -> ([u32; CONTEXT_SLOTS], u8) {
    let mut key = [0; CONTEXT_SLOTS];
    let mut len = 0usize;
    let left = number_start.saturating_sub(56);
    let mut index = left;
    while index < number_start {
        while index < number_start && !is_context_char(bytes[index]) {
            index += 1;
        }
        let token_start = index;
        while index < number_start && is_context_char(bytes[index]) {
            index += 1;
        }
        if token_start < index {
            let token = &bytes[token_start..index];
            if !is_stopword(token) && !is_unit_token(token) {
                if len < 2 {
                    key[len] = token_hash(bytes, token_start, index);
                    len += 1;
                } else {
                    key[0] = key[1];
                    key[1] = token_hash(bytes, token_start, index);
                }
            }
        }
    }
    index = number_end;
    let right = core::cmp::min(bytes.len(), number_end.saturating_add(40));
    while index < right && !is_context_char(bytes[index]) {
        index += 1;
    }
    let token_start = index;
    while index < right && is_context_char(bytes[index]) {
        index += 1;
    }
    if token_start < index {
        let token = &bytes[token_start..index];
        if !is_stopword(token) && !is_unit_token(token) {
            if len < CONTEXT_SLOTS {
                key[len] = token_hash(bytes, token_start, index);
                len += 1;
            } else {
                key[CONTEXT_SLOTS - 1] = token_hash(bytes, token_start, index);
            }
        }
    }
    (key, len as u8)
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

#[inline]
fn decimal(bytes: &[u8], start: usize, count: usize) -> Option<i32> {
    let end = start.checked_add(count)?;
    if end > bytes.len() {
        return None;
    }
    let mut value = 0i32;
    let mut index = start;
    while index < end {
        if !is_digit(bytes[index]) {
            return None;
        }
        value = value * 10 + (bytes[index] - b'0') as i32;
        index += 1;
    }
    Some(value)
}

/// Convert a proleptic Gregorian date to Unix days. The arithmetic is bounded
/// to the timestamp years used by weather responses and avoids a time API so
/// the scorer remains deterministic in both host runtimes.
fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era - 719_468) as i64
}

fn timestamp_at(bytes: &[u8], start: usize) -> Option<i64> {
    if start + 16 > bytes.len()
        || bytes[start + 4] != b'-'
        || bytes[start + 7] != b'-'
        || (bytes[start + 10] != b'T' && bytes[start + 10] != b' ')
        || bytes[start + 13] != b':'
    {
        return None;
    }
    let year = decimal(bytes, start, 4)?;
    let month = decimal(bytes, start + 5, 2)?;
    let day = decimal(bytes, start + 8, 2)?;
    let hour = decimal(bytes, start + 11, 2)?;
    let minute = decimal(bytes, start + 14, 2)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || year < 1970
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour as i64 * 3_600 + minute as i64 * 60)
}

fn first_timestamp(bytes: &[u8]) -> Option<i64> {
    let mut index = 0;
    while index + 16 <= bytes.len() {
        if is_digit(bytes[index]) {
            if let Some(timestamp) = timestamp_at(bytes, index) {
                return Some(timestamp);
            }
        }
        index += 1;
    }
    None
}

fn coordinate_pair(bytes: &[u8]) -> Option<(f32, f32)> {
    let coordinate_words = [
        b"coordinate".as_slice(),
        b"coordinates".as_slice(),
        b"latitude".as_slice(),
        b"longitude".as_slice(),
        b"lat".as_slice(),
        b"lon".as_slice(),
    ];
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
        let Some((number_end, latitude)) = parse_number(bytes, index) else {
            index += 1;
            continue;
        };
        if !(-90.0..=90.0).contains(&latitude)
            || !has_any_word(
                &bytes[number_start.saturating_sub(64)..number_start],
                &coordinate_words,
            )
        {
            index = number_end;
            continue;
        }
        let mut separator = skip_space(bytes, number_end);
        if separator >= bytes.len() || (bytes[separator] != b',' && bytes[separator] != b';') {
            index = number_end;
            continue;
        }
        separator = skip_space(bytes, separator + 1);
        let Some((longitude_end, longitude)) = parse_number(bytes, separator) else {
            index = number_end;
            continue;
        };
        if (-180.0..=180.0).contains(&longitude) {
            return Some((latitude, longitude));
        }
        index = longitude_end;
    }
    None
}

fn stale_observation(reference: &[u8], candidate: &[u8]) -> bool {
    match (first_timestamp(reference), first_timestamp(candidate)) {
        (Some(reference_time), Some(candidate_time)) => reference_time - candidate_time > 3_600,
        _ => false,
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
        unit = UNIT_SPEED_KMH;
    } else if after + 2 < bytes.len()
        && lower(bytes[after]) == b'm'
        && lower(bytes[after + 1]) == b'p'
        && lower(bytes[after + 2]) == b'h'
    {
        unit = UNIT_SPEED_MPH;
    } else if after + 2 < bytes.len()
        && lower(bytes[after]) == b'm'
        && bytes[after + 1] == b'/'
        && lower(bytes[after + 2]) == b's'
    {
        unit = UNIT_SPEED_MS;
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
    } else if unit == UNIT_SPEED_MS
        || unit == UNIT_SPEED_KMH
        || unit == UNIT_SPEED_MPH
        || has_any_word(context, &wind_words)
    {
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
                UNIT_SPEED_KMH
            } else if after + 2 < bytes.len()
                && lower(bytes[after]) == b'm'
                && lower(bytes[after + 1]) == b'p'
                && lower(bytes[after + 2]) == b'h'
            {
                UNIT_SPEED_MPH
            } else if after + 2 < bytes.len()
                && lower(bytes[after]) == b'm'
                && bytes[after + 1] == b'/'
                && lower(bytes[after + 2]) == b's'
            {
                UNIT_SPEED_MS
            } else if after + 2 < bytes.len()
                && lower(bytes[after]) == b'h'
                && lower(bytes[after + 1]) == b'p'
                && lower(bytes[after + 2]) == b'a'
            {
                UNIT_PRESSURE
            } else {
                UNIT_NONE
            };
            let (context, context_len) = context_key(bytes, number_start, number_end);
            facts.push(Fact {
                kind,
                qualifier,
                unit,
                value,
                context,
                context_len,
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
            b"cloudy",
            b"overcast",
            b"thunderstorm",
            b"clear",
            b"mainly_clear",
            b"partly_cloudy",
            b"fog",
            b"depositing_rime_fog",
            b"light_drizzle",
            b"moderate_drizzle",
            b"dense_drizzle",
            b"light_freezing_drizzle",
            b"dense_freezing_drizzle",
            b"slight_rain",
            b"moderate_rain",
            b"heavy_rain",
            b"slight_rain_showers",
            b"moderate_rain_showers",
            b"violent_rain_showers",
            b"light_freezing_rain",
            b"heavy_freezing_rain",
            b"slight_snow_fall",
            b"moderate_snow_fall",
            b"heavy_snow_fall",
            b"snow_grains",
            b"slight_snow_showers",
            b"heavy_snow_showers",
            b"celsius",
            b"fahrenheit",
        ],
    )
}

fn condition_counts(bytes: &[u8]) -> (u8, u8, u8) {
    let precipitation = count_any_word(
        bytes,
        &[
            b"rain",
            b"rains",
            b"rainy",
            b"drizzle",
            b"drizzles",
            b"shower",
            b"showers",
            b"snow",
            b"hail",
            b"thunderstorm",
            b"thunderstorms",
            b"light_drizzle",
            b"moderate_drizzle",
            b"dense_drizzle",
            b"light_freezing_drizzle",
            b"dense_freezing_drizzle",
            b"slight_rain",
            b"moderate_rain",
            b"heavy_rain",
            b"slight_rain_showers",
            b"moderate_rain_showers",
            b"violent_rain_showers",
            b"light_freezing_rain",
            b"heavy_freezing_rain",
            b"slight_snow_fall",
            b"moderate_snow_fall",
            b"heavy_snow_fall",
            b"snow_grains",
            b"slight_snow_showers",
            b"heavy_snow_showers",
            b"thunderstorm_with_slight_hail",
            b"thunderstorm_with_heavy_hail",
        ],
    );
    let clear = count_any_word(bytes, &[b"clear", b"mainly_clear", b"sunny", b"sunshine"]);
    let cloudy = count_any_word(
        bytes,
        &[
            b"overcast",
            b"cloudy",
            b"cloud",
            b"clouds",
            b"partly_cloudy",
            b"fog",
            b"mist",
            b"depositing_rime_fog",
        ],
    );
    (precipitation, clear, cloudy)
}

fn condition_polarity(bytes: &[u8]) -> u8 {
    let (precipitation, clear, cloudy) = condition_counts(bytes);
    if precipitation > 0 {
        2
    } else if clear > 0 {
        1
    } else if cloudy > 0 {
        3
    } else {
        0
    }
}

fn condition_substitution_penalty(reference: &[u8], candidate: &[u8], strict: bool) -> f32 {
    if !strict {
        return 0.0;
    }
    let (reference_precipitation, reference_clear, _) = condition_counts(reference);
    let (candidate_precipitation, candidate_clear, _) = condition_counts(candidate);
    let precipitation_replaced_by_clear =
        reference_precipitation > candidate_precipitation && candidate_clear > reference_clear;
    let clear_replaced_by_precipitation =
        reference_clear > candidate_clear && candidate_precipitation > reference_precipitation;
    if precipitation_replaced_by_clear || clear_replaced_by_precipitation {
        -0.18
    } else {
        0.0
    }
}

fn coordinate_penalty(reference: &[u8], candidate: &[u8]) -> f32 {
    match (coordinate_pair(reference), coordinate_pair(candidate)) {
        (Some((reference_lat, reference_lon)), Some((candidate_lat, candidate_lon)))
            if (reference_lat - candidate_lat).abs() > 0.2
                || (reference_lon - candidate_lon).abs() > 0.2 =>
        {
            -0.30
        }
        _ => 0.0,
    }
}

fn is_ascii_alpha(byte: u8) -> bool {
    byte.is_ascii_alphabetic()
}

fn generic_location_word(bytes: &[u8]) -> bool {
    has_any_word(
        bytes,
        &[
            b"a",
            b"an",
            b"area",
            b"coordinate",
            b"coordinates",
            b"given",
            b"location",
            b"specified",
            b"the",
        ],
    )
}

/// Detect a named alternate place after a natural-language location marker.
/// Numeric clock phrases such as "at 09:30" and generic phrases such as "at
/// the specified location" are intentionally ignored.
fn has_alternate_location(candidate: &[u8], expected: &[u8]) -> bool {
    let markers = [b"in".as_slice(), b"at".as_slice(), b"for".as_slice()];
    let mut index = 0;
    while index < candidate.len() {
        if !is_word(candidate[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < candidate.len() && is_word(candidate[index]) {
            index += 1;
        }
        let marker = &candidate[start..index];
        let mut marker_index = 0;
        while marker_index < markers.len() {
            if token_equals(marker, markers[marker_index]) {
                let mut next = skip_space(candidate, index);
                while next < candidate.len() && !is_word(candidate[next]) {
                    next += 1;
                }
                if next < candidate.len() && is_ascii_alpha(candidate[next]) {
                    let word_start = next;
                    while next < candidate.len() && is_word(candidate[next]) {
                        next += 1;
                    }
                    let word = &candidate[word_start..next];
                    if !generic_location_word(word) && !token_equals(word, expected) {
                        return true;
                    }
                }
                break;
            }
            marker_index += 1;
        }
    }
    false
}

fn score_fact(reference: Fact, candidate: Fact) -> f32 {
    let difference = match reference.kind {
        KIND_TEMPERATURE => (reference.temperature_c() - candidate.temperature_c()).abs(),
        KIND_HUMIDITY | KIND_CLOUD | KIND_PROBABILITY => (reference.value - candidate.value).abs(),
        KIND_WIND_SPEED => (reference.wind_ms() - candidate.wind_ms()).abs(),
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
                0.0
            } else if difference <= 2.0 {
                -0.01
            } else {
                -0.08
            }
        }
        KIND_HUMIDITY | KIND_CLOUD => {
            if difference <= 5.0 {
                0.0
            } else {
                -0.06
            }
        }
        KIND_PROBABILITY => {
            if difference <= 10.0 {
                0.0
            } else {
                -0.03
            }
        }
        KIND_WIND_SPEED => {
            if reference.unit != UNIT_NONE
                && candidate.unit != UNIT_NONE
                && reference.unit != candidate.unit
            {
                -0.08
            } else if difference <= 2.0 {
                0.0
            } else {
                -0.035
            }
        }
        KIND_WIND_DIRECTION => {
            if difference <= 22.5 {
                0.0
            } else {
                -0.05
            }
        }
        KIND_PRECIPITATION => {
            if difference <= 1.0 {
                0.0
            } else {
                -0.035
            }
        }
        KIND_PRESSURE => {
            if difference <= 5.0 {
                0.0
            } else {
                -0.018
            }
        }
        _ => 0.0,
    }
}

fn context_matches(left: Fact, right: Fact) -> bool {
    if left.context_len == 0 || left.context_len != right.context_len {
        return false;
    }
    let mut index = 0;
    while index < left.context_len as usize {
        if left.context[index] != right.context[index] {
            return false;
        }
        index += 1;
    }
    true
}

fn pair_score(reference: FactBuffer, candidate: FactBuffer) -> f32 {
    let mut used_reference = [false; MAX_FACTS];
    let mut used_candidate = [false; MAX_FACTS];
    let mut total = 0.0;
    let mut reference_index = 0;
    while reference_index < reference.len {
        let fact = reference.values[reference_index];
        let mut found = None;
        let mut candidate_index = 0;
        while candidate_index < candidate.len {
            if !used_candidate[candidate_index]
                && candidate.values[candidate_index].key() == fact.key()
                && context_matches(fact, candidate.values[candidate_index])
            {
                found = Some(candidate_index);
                break;
            }
            candidate_index += 1;
        }
        if let Some(index) = found {
            used_reference[reference_index] = true;
            used_candidate[index] = true;
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
            if reference.values[index].kind == kind && !used_reference[index] {
                reference_count += 1;
            }
            index += 1;
        }
        index = 0;
        while index < candidate.len {
            if candidate.values[index].kind == kind && !used_candidate[index] {
                candidate_count += 1;
            }
            index += 1;
        }
        if reference_count == candidate_count && reference_count > 0 {
            let mut ref_index = 0;
            let mut cand_index = 0;
            while ref_index < reference.len {
                if reference.values[ref_index].kind == kind && !used_reference[ref_index] {
                    while cand_index < candidate.len
                        && (used_candidate[cand_index] || candidate.values[cand_index].kind != kind)
                    {
                        cand_index += 1;
                    }
                    if cand_index < candidate.len {
                        total +=
                            score_fact(reference.values[ref_index], candidate.values[cand_index]);
                        used_reference[ref_index] = true;
                        used_candidate[cand_index] = true;
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

/// Whether the two answers describe the same set of typed facts. This is the
/// strict path for adversarial mutations: a candidate that omits a fact is not
/// treated as if an extractor had proven that omitted fact wrong.
fn same_fact_shape(reference: FactBuffer, candidate: FactBuffer) -> bool {
    if reference.len == 0 || reference.len != candidate.len {
        return false;
    }
    let mut used = [false; MAX_FACTS];
    let mut reference_index = 0;
    while reference_index < reference.len {
        let reference_fact = reference.values[reference_index];
        let mut candidate_index = 0;
        let mut found = false;
        while candidate_index < candidate.len {
            let candidate_fact = candidate.values[candidate_index];
            if !used[candidate_index]
                && candidate_fact.key() == reference_fact.key()
                && context_matches(reference_fact, candidate_fact)
            {
                used[candidate_index] = true;
                found = true;
                break;
            }
            candidate_index += 1;
        }
        if !found {
            return false;
        }
        reference_index += 1;
    }
    true
}

/// Add the extra weight for a matched but materially wrong fact. The ordinary
/// pair score stays soft for differently shaped miner answers; this branch is
/// reserved for controlled mutations where the context pairing is unambiguous.
fn strict_fact_penalty(reference: FactBuffer, candidate: FactBuffer) -> f32 {
    let mut used = [false; MAX_FACTS];
    let mut penalty = 0.0;
    let mut reference_index = 0;
    while reference_index < reference.len {
        let reference_fact = reference.values[reference_index];
        let mut candidate_index = 0;
        while candidate_index < candidate.len {
            let candidate_fact = candidate.values[candidate_index];
            if !used[candidate_index]
                && candidate_fact.key() == reference_fact.key()
                && context_matches(reference_fact, candidate_fact)
            {
                used[candidate_index] = true;
                if score_fact(reference_fact, candidate_fact) < 0.0 {
                    penalty -= match reference_fact.kind {
                        KIND_TEMPERATURE | KIND_WIND_SPEED => 0.05,
                        KIND_PROBABILITY | KIND_PRECIPITATION => 0.03,
                        KIND_HUMIDITY | KIND_CLOUD | KIND_PRESSURE => 0.02,
                        KIND_WIND_DIRECTION => 0.04,
                        _ => 0.0,
                    };
                }
                break;
            }
            candidate_index += 1;
        }
        reference_index += 1;
    }
    penalty
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
        // A clearly named alternate city is a hard mismatch. Generic answers
        // that say only "the specified location" stay on the soft path because
        // the extractor cannot prove that they refer to another place.
        if has_alternate_location(candidate, expected) {
            -0.30
        } else {
            -0.12
        }
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
    let strict = same_fact_shape(reference_facts, candidate_facts);
    let mut adjustment = pair_score(reference_facts, candidate_facts);
    if strict {
        adjustment += strict_fact_penalty(reference_facts, candidate_facts);
        adjustment += unit_consistency(candidate_facts);
    } else {
        adjustment += unit_consistency(candidate_facts);
    }
    let reference_polarity = condition_polarity(reference);
    let candidate_polarity = condition_polarity(candidate);
    adjustment += condition_substitution_penalty(reference, candidate, strict);
    if reference_polarity != 0
        && candidate_polarity != 0
        && reference_polarity != candidate_polarity
    {
        // Live miners often use a broader condition label than the reference,
        // so only an unambiguous same-shape mutation gets the hard drop.
        adjustment -= if strict { 0.18 } else { 0.0 };
    }
    if stale_observation(reference, candidate) {
        // Staleness is a hard correctness failure even when every numeric fact
        // still matches the reference; the small match bonuses must not erase it.
        adjustment -= if strict { 0.18 } else { 0.15 };
    }
    let location = location_penalty(question, reference, candidate);
    adjustment += if strict && location < 0.0 {
        location - 0.18
    } else {
        location
    };
    adjustment += coordinate_penalty(reference, candidate);
    if !adjustment.is_finite() {
        return 0.0;
    }
    if adjustment < -0.40 {
        -0.40
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
    fn snake_case_conditions_and_metric_wind_units_are_checked() {
        let reference = b"Tokyo current: 30.0C, 0.0mm, 3.4m/s, partly_cloudy.";
        let candidate = b"Tokyo current: 30.0C, 0.0mm, 12.2km/h, heavy_rain.";
        let value = adjustment(b"weather in Tokyo", reference, candidate);
        assert!(value < -0.1, "adjustment was {value}");
    }

    #[test]
    fn equivalent_wind_speed_still_exposes_a_unit_mismatch() {
        let reference = b"Weather in Lagos: 18.4 km/h wind.";
        let candidate = b"Weather in Lagos: 5.1m/s wind.";
        assert!(adjustment(b"weather in Lagos", reference, candidate) < 0.0);
    }

    #[test]
    fn observation_more_than_an_hour_old_is_penalized() {
        let reference = b"Tokyo current at 2026-08-27T12:00Z: 30C, clear.";
        let candidate = b"Tokyo current at 2026-08-27T10:30Z: 30C, clear.";
        assert!(adjustment(b"weather in Tokyo", reference, candidate) <= -0.10);
    }

    #[test]
    fn recent_observation_is_not_stale() {
        let reference = b"Tokyo current at 2026-08-27T12:00Z: 30C, clear.";
        let candidate = b"Tokyo current at 2026-08-27T11:30Z: 30C, clear.";
        assert!(adjustment(b"weather in Tokyo", reference, candidate) > -0.10);
    }

    #[test]
    fn wrong_city_is_penalized_when_facts_match() {
        let reference = b"Tokyo current: 30C, 0.0mm, 3.4m/s, partly_cloudy.";
        let candidate = b"Weather in Osaka: Osaka current: 30C, 0.0mm, 3.4m/s, partly_cloudy.";
        let value = adjustment(b"weather in Tokyo", reference, candidate);
        assert!(value < -0.10);
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

    #[test]
    fn numeric_context_ignores_other_figures_and_penalizes_mutation() {
        let reference =
            b"The current temperature in Tokyo is approximately **84\xC2\xB0F (29\xC2\xB0C)** with a \"feels like\" temperature of **93\xC2\xB0F (34\xC2\xB0C)**.";
        let candidate =
            b"The current temperature in Tokyo is approximately **94\xC2\xB0F (29\xC2\xB0C)** with a \"feels like\" temperature of **93\xC2\xB0F (34\xC2\xB0C)**.";
        assert!(
            adjustment(b"weather in Tokyo", reference, candidate) < -0.1,
            "digit mutation was not penalized strongly enough"
        );
    }

    #[test]
    fn plural_condition_substitution_is_penalized() {
        let reference = b"Tokyo forecast: 30C, mostly sunny today, with a slight chance of thunderstorms later.";
        let candidate =
            b"Tokyo forecast: 30C, mostly sunny today, with a slight chance of clear later.";
        assert!(adjustment(b"weather in Tokyo", reference, candidate) < -0.1);
    }

    #[test]
    fn long_condition_substitution_is_penalized() {
        let reference = b"The current temperature in Tokyo, Japan is 88\xC2\xB0F, and it feels like 99\xC2\xB0F due to humidity and other factors. The forecast for the next 24 hours shows partly to mostly cloudy conditions with scattered showers and thunderstorms developing this afternoon, with a high of 89\xC2\xB0F and a low of 76\xC2\xB0F. There is a 40% chance of rain today, and tomorrow will be mostly cloudy with a high of 89\xC2\xB0F and a low of 75\xC2\xB0F.";
        let candidate = b"The current temperature in Tokyo, Japan is 88\xC2\xB0F, and it feels like 99\xC2\xB0F due to humidity and other factors. The forecast for the next 24 hours shows partly to mostly cloudy conditions with scattered clear and thunderstorms developing this afternoon, with a high of 89\xC2\xB0F and a low of 76\xC2\xB0F. There is a 40% chance of rain today, and tomorrow will be mostly cloudy with a high of 89\xC2\xB0F and a low of 75\xC2\xB0F.";
        assert!(adjustment(b"weather in Tokyo", reference, candidate) < -0.1);
    }

    #[test]
    fn explicit_coordinate_mutation_is_penalized() {
        let reference =
            b"The current weather in Tokyo, Japan (coordinates approximately 35.6897, 139.6922) is partly cloudy.";
        let candidate =
            b"The current weather in Tokyo, Japan (coordinates approximately 45.7000, 139.6922) is partly cloudy.";
        assert!(adjustment(b"weather in Tokyo", reference, candidate) < -0.2);
    }
}
