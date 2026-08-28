//! Deterministic typed-fact checks for the WEATHER_FORECAST lane.
//!
//! Forecast answers are structurally different from current-weather answers:
//! one response can contain several days, hourly timestamps, daily high/low
//! pairs, and a mixture of conditions.  This module therefore keeps temporal
//! scope on every extracted fact.  It is compiled only for the separate
//! `forecast` artifact; the default build continues to use `weather.rs` for
//! WEATHER_CHECK.
//!
//! The correction is deliberately penalty-only.  A reference fact which
//! cannot be paired by its context contributes zero, as an extractor failure
//! must not become a false penalty for a concise but valid answer.

use crate::math;

const MAX_FACTS: usize = 128;
const MAX_CONDITIONS: usize = 32;
const MAX_DATES: usize = 32;
const MAX_DURATIONS: usize = 16;
const CONTEXT_SLOTS: usize = 3;

const KIND_TEMPERATURE: u8 = 1;
const KIND_HUMIDITY: u8 = 2;
const KIND_PROBABILITY: u8 = 3;
const KIND_PRECIPITATION: u8 = 4;
const KIND_WIND_SPEED: u8 = 5;

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
const UNIT_SPEED_KMH: u8 = 6;
const UNIT_SPEED_MPH: u8 = 7;
const UNIT_KELVIN: u8 = 8;

#[derive(Clone, Copy)]
struct Scope {
    day: i32,
    hour: i16,
    label: u32,
    kind: u8,
}

impl Scope {
    const NONE: Self = Self {
        day: 0,
        hour: -1,
        label: 0,
        kind: 0,
    };

    #[inline]
    fn same(self, other: Self) -> bool {
        if self.kind == 0 || other.kind == 0 {
            return true;
        }
        if self.day != 0 && other.day != 0 {
            if self.day != other.day {
                return false;
            }
            return self.hour < 0 || other.hour < 0 || self.hour == other.hour;
        }
        self.label != 0 && self.label == other.label
    }
}

#[derive(Clone, Copy)]
struct Fact {
    kind: u8,
    qualifier: u8,
    unit: u8,
    value: f32,
    context: [u32; CONTEXT_SLOTS],
    context_len: u8,
    scope: Scope,
}

impl Fact {
    const EMPTY: Self = Self {
        kind: 0,
        qualifier: QUAL_NONE,
        unit: UNIT_NONE,
        value: 0.0,
        context: [0; CONTEXT_SLOTS],
        context_len: 0,
        scope: Scope::NONE,
    };

    #[inline]
    fn key(self) -> u16 {
        ((self.kind as u16) << 8) | self.qualifier as u16
    }

    #[inline]
    fn temperature_c(self) -> f32 {
        match self.unit {
            UNIT_FAHRENHEIT => (self.value - 32.0) * (5.0 / 9.0),
            UNIT_KELVIN => self.value - 273.15,
            _ => self.value,
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
    fn push(&mut self, value: Fact) {
        if value.kind != 0 && value.value.is_finite() && self.len < MAX_FACTS {
            self.values[self.len] = value;
            self.len += 1;
        }
    }
}

#[derive(Clone, Copy)]
struct Condition {
    polarity: u8,
    scope: Scope,
}

#[derive(Clone, Copy)]
struct ConditionBuffer {
    values: [Condition; MAX_CONDITIONS],
    len: usize,
}

impl ConditionBuffer {
    const fn empty() -> Self {
        Self {
            values: [Condition {
                polarity: 0,
                scope: Scope::NONE,
            }; MAX_CONDITIONS],
            len: 0,
        }
    }

    fn push(&mut self, value: Condition) {
        if value.polarity == 0 || self.len == MAX_CONDITIONS {
            return;
        }
        if self.len > 0 {
            let previous = self.values[self.len - 1];
            if previous.polarity == value.polarity && previous.scope.same(value.scope) {
                return;
            }
        }
        self.values[self.len] = value;
        self.len += 1;
    }
}

#[derive(Clone, Copy)]
struct DateStamp {
    day: i32,
    hour: i16,
}

#[derive(Clone, Copy)]
struct DurationFact {
    value: i32,
    unit: u8,
}

#[derive(Clone, Copy)]
struct LocationWords {
    words: [u32; 3],
    len: u8,
}

impl LocationWords {
    const EMPTY: Self = Self {
        words: [0; 3],
        len: 0,
    };
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
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

#[inline]
fn is_context_char(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

#[inline]
fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
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

fn has_any(bytes: &[u8], words: &[&[u8]]) -> bool {
    let mut index = 0;
    while index < words.len() {
        if has_word(bytes, words[index]) {
            return true;
        }
        index += 1;
    }
    false
}

fn token_equals(bytes: &[u8], needle: &[u8]) -> bool {
    if bytes.len() != needle.len() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        if lower(bytes[index]) != lower(needle[index]) {
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
        b"weather".as_slice(),
        b"forecast".as_slice(),
        b"current".as_slice(),
        b"hourly".as_slice(),
        b"daily".as_slice(),
        b"today".as_slice(),
        b"tomorrow".as_slice(),
        b"yesterday".as_slice(),
        b"degrees".as_slice(),
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
    let words = [
        b"c".as_slice(),
        b"f".as_slice(),
        b"k".as_slice(),
        b"hpa".as_slice(),
        b"km".as_slice(),
        b"m".as_slice(),
        b"mm".as_slice(),
        b"mph".as_slice(),
        b"s".as_slice(),
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

fn has_token_hash(bytes: &[u8], expected: u32) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !is_word(bytes[index]) {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && is_word(bytes[index]) {
            index += 1;
        }
        if start < index && token_hash(bytes, start, index) == expected {
            return true;
        }
    }
    false
}

fn skip_space(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn parse_number(bytes: &[u8], start: usize) -> Option<(usize, f32)> {
    let mut index = start;
    let negative = if index < bytes.len() && (bytes[index] == b'-' || bytes[index] == b'+') {
        let value = bytes[index] == b'-';
        index += 1;
        value
    } else {
        false
    };
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
    value.is_finite().then_some((index, value))
}

#[inline]
fn starts_number(bytes: &[u8], index: usize) -> bool {
    is_digit(bytes[index])
        || ((bytes[index] == b'-' || bytes[index] == b'+')
            && index + 1 < bytes.len()
            && (is_digit(bytes[index + 1]) || bytes[index + 1] == b'.'))
        || (bytes[index] == b'.' && index + 1 < bytes.len() && is_digit(bytes[index + 1]))
}

fn context_key(bytes: &[u8], number_start: usize, number_end: usize) -> ([u32; 3], u8) {
    let mut output = [0; CONTEXT_SLOTS];
    let mut len = 0usize;
    let left = number_start.saturating_sub(96);
    let mut index = left;
    while index < number_start {
        while index < number_start && !is_context_char(bytes[index]) {
            index += 1;
        }
        let start = index;
        while index < number_start && is_context_char(bytes[index]) {
            index += 1;
        }
        if start < index {
            let token = &bytes[start..index];
            if !is_stopword(token) && !is_unit_token(token) {
                if len < 2 {
                    output[len] = token_hash(bytes, start, index);
                    len += 1;
                } else {
                    output[0] = output[1];
                    output[1] = token_hash(bytes, start, index);
                }
            }
        }
    }
    index = number_end;
    let right = core::cmp::min(bytes.len(), number_end.saturating_add(48));
    while index < right && !is_context_char(bytes[index]) {
        index += 1;
    }
    let start = index;
    while index < right && is_context_char(bytes[index]) {
        index += 1;
    }
    if start < index {
        let token = &bytes[start..index];
        if !is_stopword(token) && !is_unit_token(token) {
            if len < CONTEXT_SLOTS {
                output[len] = token_hash(bytes, start, index);
                len += 1;
            } else {
                output[CONTEXT_SLOTS - 1] = token_hash(bytes, start, index);
            }
        }
    }
    (output, len as u8)
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

fn days_from_civil(year: i32, month: i32, day: i32) -> i32 {
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
    era * 146_097 + day_of_era - 719_468
}

fn timestamp_at(bytes: &[u8], start: usize) -> Option<Scope> {
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
    Some(Scope {
        day: days_from_civil(year, month, day),
        hour: hour as i16,
        label: 0,
        kind: 2,
    })
}

fn date_at(bytes: &[u8], start: usize) -> Option<Scope> {
    if start + 10 > bytes.len() || bytes[start + 4] != b'-' || bytes[start + 7] != b'-' {
        return None;
    }
    let year = decimal(bytes, start, 4)?;
    let month = decimal(bytes, start + 5, 2)?;
    let day = decimal(bytes, start + 8, 2)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1970 {
        return None;
    }
    Some(Scope {
        day: days_from_civil(year, month, day),
        hour: -1,
        label: 0,
        kind: 1,
    })
}

fn month_number(token: &[u8]) -> Option<i32> {
    let months = [
        b"january".as_slice(),
        b"february".as_slice(),
        b"march".as_slice(),
        b"april".as_slice(),
        b"may".as_slice(),
        b"june".as_slice(),
        b"july".as_slice(),
        b"august".as_slice(),
        b"september".as_slice(),
        b"october".as_slice(),
        b"november".as_slice(),
        b"december".as_slice(),
    ];
    let mut index = 0;
    while index < months.len() {
        if token_equals(token, months[index]) {
            return Some(index as i32 + 1);
        }
        index += 1;
    }
    None
}

fn month_date_at(bytes: &[u8], start: usize, end: usize) -> Option<Scope> {
    let month = month_number(&bytes[start..end])?;
    let mut index = end;
    while index < bytes.len() && !is_digit(bytes[index]) {
        if is_word(bytes[index]) {
            return None;
        }
        index += 1;
    }
    let (day_end, day_value) = parse_number(bytes, index)?;
    if !(1.0..=31.0).contains(&day_value) {
        return None;
    }
    index = day_end;
    while index < bytes.len() && !is_digit(bytes[index]) {
        if is_word(bytes[index]) {
            return None;
        }
        index += 1;
    }
    let (year_end, year_value) = parse_number(bytes, index)?;
    if !(1970.0..=2200.0).contains(&year_value) || year_value - libm::floorf(year_value) != 0.0 {
        return None;
    }
    let _ = year_end;
    Some(Scope {
        day: days_from_civil(year_value as i32, month, day_value as i32),
        hour: -1,
        label: 0,
        kind: 1,
    })
}

fn relative_scope(bytes: &[u8], before: usize) -> Scope {
    let left = before.saturating_sub(96);
    let mut index = left;
    let mut result = Scope::NONE;
    while index < before {
        while index < before && !is_word(bytes[index]) {
            index += 1;
        }
        let start = index;
        while index < before && is_word(bytes[index]) {
            index += 1;
        }
        if start < index {
            let token = &bytes[start..index];
            if has_word(token, b"today")
                || has_word(token, b"tomorrow")
                || has_word(token, b"yesterday")
                || has_word(token, b"monday")
                || has_word(token, b"tuesday")
                || has_word(token, b"wednesday")
                || has_word(token, b"thursday")
                || has_word(token, b"friday")
                || has_word(token, b"saturday")
                || has_word(token, b"sunday")
            {
                result = Scope {
                    day: 0,
                    hour: -1,
                    label: token_hash(bytes, start, index),
                    kind: 3,
                };
            }
        }
    }
    result
}

fn scope_before(bytes: &[u8], position: usize) -> Scope {
    let left = position.saturating_sub(144);
    let mut index = left;
    let mut result = Scope::NONE;
    while index < position {
        if is_digit(bytes[index]) {
            if let Some(scope) = timestamp_at(bytes, index) {
                result = scope;
                index += 16;
                continue;
            }
            if let Some(scope) = date_at(bytes, index) {
                result = scope;
                index += 10;
                continue;
            }
        }
        if is_word(bytes[index]) {
            let start = index;
            while index < position && is_word(bytes[index]) {
                index += 1;
            }
            if let Some(scope) = month_date_at(bytes, start, index) {
                result = scope;
            }
            continue;
        }
        index += 1;
    }
    if result.kind == 0 {
        relative_scope(bytes, position)
    } else {
        result
    }
}

fn unit_after(bytes: &[u8], end: usize) -> u8 {
    let mut index = skip_space(bytes, end);
    if index < bytes.len() && bytes[index] == b'%' {
        return UNIT_PERCENT;
    }
    if index + 1 < bytes.len() && bytes[index] == 0xc2 && bytes[index + 1] == 0xb0 {
        index += 2;
        if index < bytes.len() && lower(bytes[index]) == b'c' {
            return UNIT_CELSIUS;
        }
        if index < bytes.len() && lower(bytes[index]) == b'f' {
            return UNIT_FAHRENHEIT;
        }
    }
    if index < bytes.len() && (lower(bytes[index]) == b'c' || lower(bytes[index]) == b'f') {
        let unit = lower(bytes[index]);
        if index + 1 == bytes.len() || !is_word(bytes[index + 1]) {
            return if unit == b'c' {
                UNIT_CELSIUS
            } else {
                UNIT_FAHRENHEIT
            };
        }
    }
    if index < bytes.len() && lower(bytes[index]) == b'k' {
        if index + 1 == bytes.len() || !is_word(bytes[index + 1]) {
            return UNIT_KELVIN;
        }
    }
    if index + 1 < bytes.len() && lower(bytes[index]) == b'm' && lower(bytes[index + 1]) == b'm' {
        return UNIT_MM;
    }
    if index + 3 < bytes.len()
        && lower(bytes[index]) == b'k'
        && lower(bytes[index + 1]) == b'm'
        && bytes[index + 2] == b'/'
        && lower(bytes[index + 3]) == b'h'
    {
        return UNIT_SPEED_KMH;
    }
    if index + 2 < bytes.len()
        && lower(bytes[index]) == b'm'
        && lower(bytes[index + 1]) == b'p'
        && lower(bytes[index + 2]) == b'h'
    {
        return UNIT_SPEED_MPH;
    }
    if index + 2 < bytes.len()
        && lower(bytes[index]) == b'm'
        && bytes[index + 1] == b'/'
        && lower(bytes[index + 2]) == b's'
    {
        return UNIT_SPEED_MS;
    }
    if index + 1 < bytes.len() && is_context_char(bytes[index]) {
        let start = index;
        while index < bytes.len() && is_context_char(bytes[index]) {
            index += 1;
        }
        if token_equals(&bytes[start..index], b"degrees") {
            index = skip_space(bytes, index);
            if index < bytes.len()
                && index + 1 < bytes.len()
                && bytes[index] == 0xc2
                && bytes[index + 1] == 0xb0
            {
                index += 2;
            }
            let unit_start = index;
            while index < bytes.len() && is_context_char(bytes[index]) {
                index += 1;
            }
            let unit = &bytes[unit_start..index];
            if token_equals(unit, b"celsius") || token_equals(unit, b"c") {
                return UNIT_CELSIUS;
            }
            if token_equals(unit, b"fahrenheit") || token_equals(unit, b"f") {
                return UNIT_FAHRENHEIT;
            }
            if token_equals(unit, b"kelvin") || token_equals(unit, b"k") {
                return UNIT_KELVIN;
            }
        }
    }
    UNIT_NONE
}

fn classify(unit: u8, _value: f32, context: &[u8]) -> (u8, u8) {
    let humidity_words = [b"humidity".as_slice(), b"humid".as_slice()];
    let probability_words = [
        b"chance".as_slice(),
        b"probability".as_slice(),
        b"likelihood".as_slice(),
    ];
    let precipitation_words = [
        b"precipitation".as_slice(),
        b"precip".as_slice(),
        b"rain".as_slice(),
        b"snow".as_slice(),
        b"drizzle".as_slice(),
    ];
    let wind_words = [b"wind".as_slice(), b"gust".as_slice()];
    let kind = if matches!(unit, UNIT_CELSIUS | UNIT_FAHRENHEIT | UNIT_KELVIN) {
        KIND_TEMPERATURE
    } else if unit == UNIT_PERCENT && has_any(context, &humidity_words) {
        KIND_HUMIDITY
    } else if unit == UNIT_PERCENT && has_any(context, &probability_words) {
        KIND_PROBABILITY
    } else if unit == UNIT_MM && has_any(context, &precipitation_words) {
        KIND_PRECIPITATION
    } else if matches!(unit, UNIT_SPEED_MS | UNIT_SPEED_KMH | UNIT_SPEED_MPH)
        && (has_any(context, &wind_words) || unit != UNIT_NONE)
    {
        KIND_WIND_SPEED
    } else {
        0
    };
    let qualifier = if has_any(context, &[b"high", b"maximum", b"max", b"highest"]) {
        QUAL_HIGH
    } else if has_any(context, &[b"low", b"minimum", b"min", b"lowest"]) {
        QUAL_LOW
    } else if has_any(context, &[b"apparent", b"feels", b"feeling"]) {
        QUAL_APPARENT
    } else {
        QUAL_NONE
    };
    (kind, qualifier)
}

fn collect_facts(bytes: &[u8]) -> FactBuffer {
    let mut facts = FactBuffer::empty();
    let mut index = 0;
    while index < bytes.len() {
        if !starts_number(bytes, index) {
            index += 1;
            continue;
        }
        let number_start = index;
        let Some((number_end, value)) = parse_number(bytes, index) else {
            index += 1;
            continue;
        };
        let unit = unit_after(bytes, number_end);
        let left = number_start.saturating_sub(96);
        let right = core::cmp::min(bytes.len(), number_end.saturating_add(48));
        let (kind, qualifier) = classify(unit, value, &bytes[left..right]);
        if kind != 0 {
            let (context, context_len) = context_key(bytes, number_start, number_end);
            facts.push(Fact {
                kind,
                qualifier,
                unit,
                value,
                context,
                context_len,
                scope: scope_before(bytes, number_start),
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

#[inline]
fn facts_match(left: Fact, right: Fact) -> bool {
    left.key() == right.key() && left.scope.same(right.scope) && context_matches(left, right)
}

fn fact_difference(reference: Fact, candidate: Fact) -> f32 {
    let difference = match reference.kind {
        KIND_TEMPERATURE => (reference.temperature_c() - candidate.temperature_c()).abs(),
        KIND_HUMIDITY | KIND_PROBABILITY | KIND_PRECIPITATION => {
            (reference.value - candidate.value).abs()
        }
        KIND_WIND_SPEED => (reference.wind_ms() - candidate.wind_ms()).abs(),
        _ => 0.0,
    };
    math::finite_or_zero(difference)
}

fn score_fact(reference: Fact, candidate: Fact) -> f32 {
    if reference.kind != candidate.kind {
        return 0.0;
    }
    let difference = fact_difference(reference, candidate);
    match reference.kind {
        KIND_TEMPERATURE => {
            if difference <= 0.5 {
                0.0
            } else if difference <= 2.0 {
                -0.01
            } else if reference.temperature_c() * candidate.temperature_c() < 0.0 {
                -0.12
            } else {
                -0.08
            }
        }
        KIND_HUMIDITY => {
            if difference <= 5.0 {
                0.0
            } else {
                -0.05
            }
        }
        KIND_PROBABILITY => {
            if difference <= 10.0 {
                0.0
            } else {
                -0.04
            }
        }
        KIND_PRECIPITATION => {
            if difference <= 1.0 {
                0.0
            } else {
                -0.04
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
                -0.04
            }
        }
        _ => 0.0,
    }
}

/// Pair by the stated context first.  Positional fallback is used only for a
/// key whose remaining reference/candidate counts are equal.
fn pair_facts(reference: FactBuffer, candidate: FactBuffer) -> (f32, usize) {
    let mut used_reference = [false; MAX_FACTS];
    let mut used_candidate = [false; MAX_FACTS];
    let mut total = 0.0;
    let mut matched = 0usize;
    let mut reference_index = 0;
    while reference_index < reference.len {
        let reference_fact = reference.values[reference_index];
        let mut candidate_index = 0;
        while candidate_index < candidate.len {
            if !used_candidate[candidate_index]
                && facts_match(reference_fact, candidate.values[candidate_index])
            {
                used_reference[reference_index] = true;
                used_candidate[candidate_index] = true;
                total += score_fact(reference_fact, candidate.values[candidate_index]);
                matched += 1;
                break;
            }
            candidate_index += 1;
        }
        reference_index += 1;
    }

    let mut kind = 1;
    while kind <= 5 {
        let mut qualifier = 0;
        while qualifier <= QUAL_APPARENT {
            let key = ((kind as u16) << 8) | qualifier as u16;
            let mut reference_count = 0;
            let mut candidate_count = 0;
            let mut index = 0;
            while index < reference.len {
                if !used_reference[index] && reference.values[index].key() == key {
                    reference_count += 1;
                }
                index += 1;
            }
            index = 0;
            while index < candidate.len {
                if !used_candidate[index] && candidate.values[index].key() == key {
                    candidate_count += 1;
                }
                index += 1;
            }
            if reference_count > 0 && reference_count == candidate_count {
                let mut reference_index = 0;
                let mut candidate_index = 0;
                while reference_index < reference.len {
                    if !used_reference[reference_index]
                        && reference.values[reference_index].key() == key
                    {
                        while candidate_index < candidate.len
                            && (used_candidate[candidate_index]
                                || candidate.values[candidate_index].key() != key)
                        {
                            candidate_index += 1;
                        }
                        if candidate_index < candidate.len {
                            total += score_fact(
                                reference.values[reference_index],
                                candidate.values[candidate_index],
                            );
                            used_reference[reference_index] = true;
                            used_candidate[candidate_index] = true;
                            matched += 1;
                            candidate_index += 1;
                        }
                    }
                    reference_index += 1;
                }
            }
            qualifier += 1;
        }
        kind += 1;
    }
    (math::finite_or_zero(total), matched)
}

fn same_shape(reference: FactBuffer, candidate: FactBuffer) -> bool {
    if reference.len == 0 || reference.len != candidate.len {
        return false;
    }
    let mut used = [false; MAX_FACTS];
    let mut reference_index = 0;
    while reference_index < reference.len {
        let mut candidate_index = 0;
        let mut found = false;
        while candidate_index < candidate.len {
            if !used[candidate_index]
                && facts_match(
                    reference.values[reference_index],
                    candidate.values[candidate_index],
                )
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

fn strict_fact_penalty(reference: FactBuffer, candidate: FactBuffer) -> f32 {
    let mut used = [false; MAX_FACTS];
    let mut total = 0.0;
    let mut reference_index = 0;
    while reference_index < reference.len {
        let mut candidate_index = 0;
        while candidate_index < candidate.len {
            if !used[candidate_index]
                && facts_match(
                    reference.values[reference_index],
                    candidate.values[candidate_index],
                )
            {
                used[candidate_index] = true;
                if score_fact(
                    reference.values[reference_index],
                    candidate.values[candidate_index],
                ) < 0.0
                {
                    total -= match reference.values[reference_index].kind {
                        KIND_TEMPERATURE | KIND_WIND_SPEED => 0.05,
                        KIND_PROBABILITY | KIND_PRECIPITATION => 0.035,
                        KIND_HUMIDITY => 0.025,
                        _ => 0.0,
                    };
                }
                break;
            }
            candidate_index += 1;
        }
        reference_index += 1;
    }
    math::finite_or_zero(total)
}

fn unit_consistency(facts: FactBuffer) -> f32 {
    let mut index = 0;
    while index < facts.len {
        if facts.values[index].kind == KIND_TEMPERATURE {
            let mut other = index + 1;
            while other < facts.len {
                if facts.values[other].kind == KIND_TEMPERATURE
                    && facts.values[other].qualifier == facts.values[index].qualifier
                    && facts.values[other].scope.same(facts.values[index].scope)
                    && context_matches(facts.values[index], facts.values[other])
                    && facts.values[other].unit != facts.values[index].unit
                    && (facts.values[index].temperature_c() - facts.values[other].temperature_c())
                        .abs()
                        > 1.0
                {
                    return -0.10;
                }
                other += 1;
            }
        }
        index += 1;
    }
    0.0
}

fn range_penalty(facts: FactBuffer) -> f32 {
    let mut index = 0;
    while index < facts.len {
        if facts.values[index].kind == KIND_TEMPERATURE
            && facts.values[index].qualifier == QUAL_HIGH
        {
            let mut other = 0;
            while other < facts.len {
                if facts.values[other].kind == KIND_TEMPERATURE
                    && facts.values[other].qualifier == QUAL_LOW
                    && facts.values[other].scope.same(facts.values[index].scope)
                    && facts.values[index].temperature_c() < facts.values[other].temperature_c()
                {
                    return -0.08;
                }
                other += 1;
            }
        }
        index += 1;
    }
    0.0
}

fn condition_polarity(token: &[u8]) -> u8 {
    let wet = [
        b"rain".as_slice(),
        b"rains".as_slice(),
        b"rainy".as_slice(),
        b"drizzle".as_slice(),
        b"drizzles".as_slice(),
        b"shower".as_slice(),
        b"showers".as_slice(),
        b"snow".as_slice(),
        b"hail".as_slice(),
        b"thunderstorm".as_slice(),
        b"thunderstorms".as_slice(),
        b"light_rain".as_slice(),
        b"moderate_rain".as_slice(),
        b"heavy_rain".as_slice(),
        b"light_drizzle".as_slice(),
        b"moderate_drizzle".as_slice(),
        b"thunderstorm_with_slight_hail".as_slice(),
        b"thunderstorm_with_heavy_hail".as_slice(),
    ];
    if has_any(token, &wet) {
        return 2;
    }
    let dry = [
        b"clear".as_slice(),
        b"sunny".as_slice(),
        b"sunshine".as_slice(),
        b"mainly_clear".as_slice(),
        b"mostly_clear".as_slice(),
    ];
    if has_any(token, &dry) {
        return 1;
    }
    let cloudy = [
        b"overcast".as_slice(),
        b"cloudy".as_slice(),
        b"cloud".as_slice(),
        b"clouds".as_slice(),
        b"partly_cloudy".as_slice(),
        b"fog".as_slice(),
        b"mist".as_slice(),
    ];
    if has_any(token, &cloudy) {
        3
    } else {
        0
    }
}

fn negated_before(bytes: &[u8], start: usize) -> bool {
    let left = start.saturating_sub(28);
    let prefix = &bytes[left..start];
    has_any(prefix, &[b"no", b"not", b"without", b"none"])
}

fn collect_conditions(bytes: &[u8]) -> ConditionBuffer {
    let mut conditions = ConditionBuffer::empty();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !is_word(bytes[index]) {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && is_word(bytes[index]) {
            index += 1;
        }
        if start == index {
            continue;
        }
        let mut polarity = condition_polarity(&bytes[start..index]);
        if polarity == 2 && negated_before(bytes, start) {
            polarity = 1;
        }
        conditions.push(Condition {
            polarity,
            scope: scope_before(bytes, start),
        });
    }
    conditions
}

fn condition_adjustment(reference: &[u8], candidate: &[u8], strict: bool) -> f32 {
    let _ = strict;
    let reference_conditions = collect_conditions(reference);
    let candidate_conditions = collect_conditions(candidate);
    if reference_conditions.len == 0 || reference_conditions.len != candidate_conditions.len {
        return 0.0;
    }
    let mut used = [false; MAX_CONDITIONS];
    let mut total = 0.0;
    let mut index = 0;
    while index < reference_conditions.len {
        let reference_condition = reference_conditions.values[index];
        let mut candidate_index = 0;
        while candidate_index < candidate_conditions.len {
            if !used[candidate_index]
                && reference_condition
                    .scope
                    .same(candidate_conditions.values[candidate_index].scope)
            {
                used[candidate_index] = true;
                let candidate_polarity = candidate_conditions.values[candidate_index].polarity;
                if reference_condition.polarity != candidate_polarity {
                    total -= if reference_condition.polarity == 1 || candidate_polarity == 1 {
                        0.18
                    } else {
                        0.08
                    };
                }
                break;
            }
            candidate_index += 1;
        }
        index += 1;
    }
    math::finite_or_zero(total)
}

fn coordinate_pair(bytes: &[u8]) -> Option<(f32, f32)> {
    let words = [
        b"coordinate".as_slice(),
        b"coordinates".as_slice(),
        b"latitude".as_slice(),
        b"longitude".as_slice(),
        b"lat".as_slice(),
        b"lon".as_slice(),
    ];
    let mut index = 0;
    while index < bytes.len() {
        if !starts_number(bytes, index) {
            index += 1;
            continue;
        }
        let start = index;
        let Some((end, latitude)) = parse_number(bytes, index) else {
            index += 1;
            continue;
        };
        if !(-90.0..=90.0).contains(&latitude)
            || !has_any(&bytes[start.saturating_sub(72)..start], &words)
        {
            index = end;
            continue;
        }
        let mut separator = skip_space(bytes, end);
        if separator + 1 < bytes.len() && bytes[separator] == 0xc2 && bytes[separator + 1] == 0xb0 {
            separator = skip_space(bytes, separator + 2);
            if separator < bytes.len() && is_word(bytes[separator]) {
                separator += 1;
            }
        }
        if separator >= bytes.len() || (bytes[separator] != b',' && bytes[separator] != b';') {
            index = end;
            continue;
        }
        separator = skip_space(bytes, separator + 1);
        let Some((longitude_end, longitude)) = parse_number(bytes, separator) else {
            index = end;
            continue;
        };
        if (-180.0..=180.0).contains(&longitude) {
            return Some((latitude, longitude));
        }
        index = longitude_end;
    }
    None
}

fn location_stopword(token: &[u8]) -> bool {
    has_any(
        token,
        &[
            b"a",
            b"an",
            b"and",
            b"at",
            b"before",
            b"beginning",
            b"by",
            b"coordinates",
            b"each",
            b"ending",
            b"for",
            b"from",
            b"including",
            b"latitude",
            b"location",
            b"longitude",
            b"next",
            b"the",
            b"with",
            b"weather",
            b"forecast",
            b"temperature",
            b"precipitation",
            b"probability",
            b"hourly",
            b"daily",
        ],
    )
}

fn question_location(question: &[u8]) -> LocationWords {
    let mut output = LocationWords::EMPTY;
    let mut index = 0;
    while index < question.len() {
        while index < question.len() && !is_word(question[index]) {
            index += 1;
        }
        let marker_start = index;
        while index < question.len() && is_word(question[index]) {
            index += 1;
        }
        if marker_start == index {
            continue;
        }
        let marker = &question[marker_start..index];
        if !token_equals(marker, b"in")
            && !token_equals(marker, b"at")
            && !token_equals(marker, b"for")
        {
            continue;
        }
        let mut next = skip_space(question, index);
        while next < question.len() && !is_word(question[next]) {
            next += 1;
        }
        if next >= question.len() {
            continue;
        }
        let first_start = next;
        while next < question.len() && is_word(question[next]) {
            next += 1;
        }
        if location_stopword(&question[first_start..next]) {
            continue;
        }
        output.words[0] = token_hash(question, first_start, next);
        output.len = 1;
        let mut previous_end = next;
        while output.len < 3 && next < question.len() {
            let had_comma = question[previous_end..next.min(question.len())]
                .iter()
                .any(|byte| *byte == b',');
            if had_comma {
                break;
            }
            while next < question.len() && !is_word(question[next]) {
                next += 1;
            }
            let word_start = next;
            while next < question.len() && is_word(question[next]) {
                next += 1;
            }
            if word_start == next || location_stopword(&question[word_start..next]) {
                break;
            }
            output.words[output.len as usize] = token_hash(question, word_start, next);
            output.len += 1;
            previous_end = next;
        }
        return output;
    }
    output
}

fn location_present(bytes: &[u8], location: LocationWords) -> bool {
    if location.len == 0 {
        return false;
    }
    let mut index = 0;
    while index < location.len as usize {
        if !has_token_hash(bytes, location.words[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn alternate_location(bytes: &[u8], expected: LocationWords) -> bool {
    if expected.len == 0 {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && !is_word(bytes[index]) {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && is_word(bytes[index]) {
            index += 1;
        }
        if start == index {
            continue;
        }
        let marker = &bytes[start..index];
        if token_equals(marker, b"in")
            || token_equals(marker, b"at")
            || token_equals(marker, b"for")
        {
            let mut next = index;
            while next < bytes.len() && !is_word(bytes[next]) {
                next += 1;
            }
            let word_start = next;
            while next < bytes.len() && is_word(bytes[next]) {
                next += 1;
            }
            if word_start < next {
                let word = &bytes[word_start..next];
                if !location_stopword(word)
                    && token_hash(bytes, word_start, next) != expected.words[0]
                {
                    return true;
                }
            }
        }
    }
    false
}

fn location_adjustment(question: &[u8], reference: &[u8], candidate: &[u8]) -> f32 {
    let mut total = 0.0;
    if let Some((reference_lat, reference_lon)) = coordinate_pair(reference) {
        if let Some((candidate_lat, candidate_lon)) = coordinate_pair(candidate) {
            if (reference_lat - candidate_lat).abs() > 0.2
                || (reference_lon - candidate_lon).abs() > 0.2
            {
                total -= 0.30;
            }
        }
    }
    let location = question_location(question);
    if location.len > 0
        && location_present(reference, location)
        && !location_present(candidate, location)
        && coordinate_pair(candidate).is_none()
    {
        total -= if alternate_location(candidate, location) {
            0.30
        } else {
            0.12
        };
    }
    total
}

fn collect_dates(bytes: &[u8]) -> ([DateStamp; MAX_DATES], usize) {
    let mut dates = [DateStamp { day: 0, hour: -1 }; MAX_DATES];
    let mut len = 0;
    let mut index = 0;
    while index < bytes.len() {
        let scope = if is_digit(bytes[index]) {
            timestamp_at(bytes, index).or_else(|| date_at(bytes, index))
        } else {
            None
        };
        let scope = if scope.is_none() && is_word(bytes[index]) {
            let start = index;
            while index < bytes.len() && is_word(bytes[index]) {
                index += 1;
            }
            month_date_at(bytes, start, index)
        } else {
            scope
        };
        if let Some(scope) = scope {
            let mut exists = false;
            let mut date_index = 0;
            while date_index < len {
                if dates[date_index].day == scope.day && dates[date_index].hour == scope.hour {
                    exists = true;
                    break;
                }
                date_index += 1;
            }
            if !exists && len < MAX_DATES {
                dates[len] = DateStamp {
                    day: scope.day,
                    hour: scope.hour,
                };
                len += 1;
            }
        }
        if index < bytes.len() {
            index += 1;
        }
    }
    (dates, len)
}

fn date_adjustment(reference: &[u8], candidate: &[u8], _paired_facts: usize) -> f32 {
    let (reference_dates, reference_len) = collect_dates(reference);
    let (candidate_dates, candidate_len) = collect_dates(candidate);
    if reference_len == 0 || reference_len != candidate_len {
        return 0.0;
    }
    let mut mismatches = 0;
    let mut index = 0;
    while index < reference_len {
        let mut found = false;
        let mut candidate_index = 0;
        while candidate_index < candidate_len {
            if reference_dates[index].day == candidate_dates[candidate_index].day {
                found = true;
                break;
            }
            candidate_index += 1;
        }
        if !found {
            mismatches += 1;
        }
        index += 1;
    }
    -0.025 * core::cmp::min(mismatches, 4) as f32
}

fn duration_unit_after(bytes: &[u8], end: usize) -> Option<u8> {
    let mut index = skip_space(bytes, end);
    if index < bytes.len() && bytes[index] == b'-' {
        index = skip_space(bytes, index + 1);
    }
    let start = index;
    while index < bytes.len() && is_context_char(bytes[index]) {
        index += 1;
    }
    if start == index {
        return None;
    }
    let token = &bytes[start..index];
    if token_equals(token, b"day") || token_equals(token, b"days") {
        Some(1)
    } else if token_equals(token, b"hour") || token_equals(token, b"hours") {
        Some(2)
    } else {
        None
    }
}

fn collect_durations(bytes: &[u8]) -> ([DurationFact; MAX_DURATIONS], usize) {
    let mut durations = [DurationFact { value: 0, unit: 0 }; MAX_DURATIONS];
    let mut len = 0;
    let mut index = 0;
    while index < bytes.len() {
        if !starts_number(bytes, index) {
            index += 1;
            continue;
        }
        let Some((end, value)) = parse_number(bytes, index) else {
            index += 1;
            continue;
        };
        if value - libm::floorf(value) == 0.0 && (1.0..=168.0).contains(&value) {
            if let Some(unit) = duration_unit_after(bytes, end) {
                if len < MAX_DURATIONS {
                    durations[len] = DurationFact {
                        value: value as i32,
                        unit,
                    };
                    len += 1;
                }
            }
        }
        index = if end > index { end } else { index + 1 };
    }
    (durations, len)
}

fn duration_adjustment(reference: &[u8], candidate: &[u8]) -> f32 {
    let (reference_durations, reference_len) = collect_durations(reference);
    let (candidate_durations, candidate_len) = collect_durations(candidate);
    if reference_len == 0 || reference_len != candidate_len {
        return 0.0;
    }
    let mut mismatches = 0;
    let mut index = 0;
    while index < reference_len {
        if reference_durations[index].value != candidate_durations[index].value
            || reference_durations[index].unit != candidate_durations[index].unit
        {
            mismatches += 1;
        }
        index += 1;
    }
    -0.12 * core::cmp::min(mismatches, 3) as f32
}

fn observation_timestamp(bytes: &[u8]) -> Option<i64> {
    let mut index = 0;
    while index + 16 <= bytes.len() {
        if let Some(scope) = timestamp_at(bytes, index) {
            let prefix = &bytes[index.saturating_sub(56)..index];
            if has_any(
                prefix,
                &[
                    b"observed",
                    b"updated",
                    b"issued",
                    b"valid",
                    b"as_of",
                    b"as",
                ],
            ) {
                return Some(scope.day as i64 * 86_400 + scope.hour as i64 * 3_600);
            }
        }
        index += 1;
    }
    None
}

fn stale_adjustment(reference: &[u8], candidate: &[u8]) -> f32 {
    match (
        observation_timestamp(reference),
        observation_timestamp(candidate),
    ) {
        (Some(reference_time), Some(candidate_time)) if reference_time - candidate_time > 3_600 => {
            -0.16
        }
        _ => 0.0,
    }
}

fn forecast_signal(bytes: &[u8]) -> bool {
    has_any(
        bytes,
        &[
            b"weather",
            b"forecast",
            b"temperature",
            b"precipitation",
            b"precip",
            b"wind",
            b"rain",
            b"drizzle",
            b"thunderstorm",
            b"celsius",
            b"fahrenheit",
        ],
    )
}

/// Return the bounded forecast-specific typed-fact adjustment.
pub fn adjustment(question: &[u8], reference: &[u8], candidate: &[u8]) -> f32 {
    if reference == candidate || !forecast_signal(reference) || !forecast_signal(candidate) {
        return 0.0;
    }
    let reference_facts = collect_facts(reference);
    let candidate_facts = collect_facts(candidate);
    let strict = same_shape(reference_facts, candidate_facts);
    let (mut total, paired_facts) = pair_facts(reference_facts, candidate_facts);
    if strict {
        total += strict_fact_penalty(reference_facts, candidate_facts);
        total += unit_consistency(candidate_facts);
        total += range_penalty(candidate_facts);
    } else {
        total += unit_consistency(candidate_facts);
    }
    total += condition_adjustment(reference, candidate, strict);
    total += date_adjustment(reference, candidate, paired_facts);
    total += duration_adjustment(reference, candidate);
    total += stale_adjustment(reference, candidate);
    // A refusal-style reference often repeats the requested city without
    // containing any forecast facts.  Do not turn that prose into a hard
    // location verdict against a live miner answer; location is high
    // confidence once the reference carries structured forecast facts.
    if reference_facts.len > 0 && candidate_facts.len > 0 {
        total += location_adjustment(question, reference, candidate);
    }
    if !total.is_finite() {
        return 0.0;
    }
    if total < -0.45 {
        -0.45
    } else if total > 0.0 {
        0.0
    } else {
        total
    }
}

#[cfg(test)]
mod tests {
    use super::adjustment;

    #[test]
    fn exact_forecast_text_has_no_adjustment() {
        let text = b"Tokyo forecast: today high 35C low 21C clear; tomorrow high 36C low 20C clear. Nearest hour 2026-08-25T00:00Z: 33.5C, 0.0mm, 4.2m/s, clear.";
        assert_eq!(adjustment(b"forecast in Tokyo", text, text), 0.0);
    }

    #[test]
    fn high_low_and_hourly_digit_mutations_are_penalized() {
        let reference = b"Tokyo forecast: today high 35C low 21C clear; tomorrow high 36C low 20C clear. Nearest hour 2026-08-25T00:00Z: 33.5C, 0.0mm, 4.2m/s, clear.";
        let candidate = b"Tokyo forecast: today high 25C low 21C clear; tomorrow high 36C low 20C clear. Nearest hour 2026-08-25T00:00Z: 33.5C, 0.0mm, 4.2m/s, clear.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < -0.05);
    }

    #[test]
    fn condition_inversion_is_penalized_only_when_shape_is_clear() {
        let reference =
            b"Tokyo forecast: today high 35C low 21C clear; tomorrow high 36C low 20C clear.";
        let candidate =
            b"Tokyo forecast: today high 35C low 21C rain; tomorrow high 36C low 20C rain.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < -0.1);
    }

    #[test]
    fn equivalent_temperature_units_are_accepted() {
        let reference = b"Tokyo forecast: high 30C low 20C clear.";
        let candidate = b"Tokyo forecast: high 86F low 68F clear.";
        assert_eq!(adjustment(b"forecast in Tokyo", reference, candidate), 0.0);
    }

    #[test]
    fn wind_unit_mismatch_is_still_hard() {
        let reference = b"Tokyo forecast: high 30C low 20C clear, wind 3.4m/s.";
        let candidate = b"Tokyo forecast: high 30C low 20C clear, wind 3.4km/h.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < 0.0);
    }

    #[test]
    fn wrong_coordinates_are_penalized() {
        let reference = b"Tokyo forecast at coordinates 35.6897, 139.6922: high 30C low 20C clear.";
        let candidate = b"Osaka forecast at coordinates 34.6937, 135.5023: high 30C low 20C clear.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < -0.2);
    }

    #[test]
    fn refusal_reference_does_not_penalize_unmatched_live_facts() {
        let reference = b"Sorry, I cannot provide the exact 7-day hourly forecast for Tokyo before 2026-09-01T06:00Z.";
        let candidate = b"Tokyo forecast: today high 30C low 24C partly cloudy.";
        assert_eq!(adjustment(b"forecast in Tokyo", reference, candidate), 0.0);
    }

    #[test]
    fn only_explicit_observation_timestamps_are_stale() {
        let reference = b"Tokyo forecast valid at 2026-08-28T12:00Z: high 30C low 24C clear.";
        let candidate = b"Tokyo forecast valid at 2026-08-28T10:00Z: high 30C low 24C clear.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < -0.1);
    }

    #[test]
    fn horizon_mutation_is_penalized() {
        let reference = b"Tokyo 7-day forecast: today high 30C low 24C clear.";
        let candidate = b"Tokyo 17-day forecast: today high 30C low 24C clear.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < -0.1);
    }

    #[test]
    fn equal_count_date_mutation_is_penalized() {
        let reference = b"Tokyo forecast for 2026-09-01: high 30C low 24C clear.";
        let candidate = b"Tokyo forecast for 2026-09-11: high 30C low 24C clear.";
        assert!(adjustment(b"forecast in Tokyo", reference, candidate) < 0.0);
    }
}
