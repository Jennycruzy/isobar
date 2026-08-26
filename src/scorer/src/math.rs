//! Deterministic scalar math shared by the scorer and native harness.

const EPSILON: f32 = 1.0e-12;

#[inline]
pub fn finite_or_zero(value: f32) -> f32 {
    if value.is_nan() || value.is_infinite() {
        0.0
    } else {
        value
    }
}

#[inline]
#[allow(clippy::manual_clamp)]
pub fn clamp01(value: f32) -> f32 {
    let value = finite_or_zero(value);
    if value <= 0.0 {
        0.0
    } else if value >= 1.0 {
        1.0
    } else {
        value
    }
}

#[inline]
pub fn safe_div(numerator: f32, denominator: f32, fallback: f32) -> f32 {
    let denominator = finite_or_zero(denominator);
    if denominator.abs() <= EPSILON {
        clamp01(fallback)
    } else {
        finite_or_zero(numerator / denominator)
    }
}

#[inline]
pub fn sigmoid(value: f32) -> f32 {
    // Keep the published baseline's direct form while using libm explicitly.
    // A host-provided exp can vary across runtime implementations.
    let value = finite_or_zero(value);
    safe_div(1.0, 1.0 + libm::expf(-value), 0.5)
}

/// Endpoint-pinned, strictly increasing logistic contrast transform.
pub fn contrast_norm(value: f32, steepness: f32, centre: f32) -> f32 {
    let x = clamp01(value);
    let k = if steepness.is_finite() && steepness > 0.0 {
        steepness
    } else {
        8.0
    };
    let c = if centre.is_finite() { centre } else { 0.5 };
    let low = sigmoid(-k * c);
    let high = sigmoid(k * (1.0 - c));
    let transformed = sigmoid(k * (x - c));
    clamp01(safe_div(transformed - low, high - low, x))
}

/// Round only at the public-score boundary. Six decimal places preserve more
/// rank information than coarse display-oriented rounding.
#[inline]
pub fn quantize6(value: f32) -> f32 {
    let value = clamp01(value);
    let scaled = libm::floorf(value * 1_000_000.0 + 0.5);
    clamp01(scaled * 0.000001)
}
