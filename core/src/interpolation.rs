//! Traits and implementations related to interpolation of animatable values.

use num_traits::FromPrimitive;

/// Trait for a type that supports the standard `lerp` (**l**inear int**erp**olation) operation.
///
/// Linear interpolation refers mathematically to computing the `y` value of the straight line
/// connecting two points `(x0, y0)` and `(x1, y1)` at a given `x` position. This requires solving
/// for the equality: `(y - y0) / (x - x0) = (y1 - y0) / (x1 - x0)`. `Lerp` assumes a normalized `x`
/// value, such that _x0_ = 0 and _x1_ = 1, which reduces the equation to:
///
/// `lerp(y0, y1, x) = y0 + x(y1 - y0)`
///
/// All primitive numeric types are implicitly `lerp`able, although the computation is performed
/// using 32-bit floating-point arithmetic, so there may be some precision loss when interpolating
/// with a type that is narrower (e.g. `u32`) **or** wider (`f64`). For any other type that is
/// composed entirely of numeric values, the trait can be implemented by `lerp`ing all of the
/// individual values.
pub trait Lerp {
    /// Computes the linear interpolation between this value (`y0`) and a second (`y1`) value of the
    /// same type, at normalized (from 0 to 1) position `x`.
    ///
    /// # Panics
    ///
    /// The default implementation for primitives will panic if `self` or `y1` are too large to fit
    /// in an `f32`, or if the resulting interpolated value is out of bounds for the `y` type.
    ///
    /// # Example
    ///
    /// ```
    /// use mina_core::interpolation::Lerp;
    ///
    /// let y0: f32 = 5.0;
    /// let y1: f32 = 15.0;
    ///
    /// assert_eq!(y0.lerp(&y1, 0.0), 5.0);
    /// assert_eq!(y0.lerp(&y1, 0.25), 7.5);
    /// assert_eq!(y0.lerp(&y1, 0.5), 10.0);
    /// assert_eq!(y0.lerp(&y1, 1.0), 15.0);
    /// ```
    fn lerp(&self, y1: &Self, x: f32) -> Self;
}

// There are (roughly) two ways to represent the "lerp equation":
//
// 1. a + t(b - a)
// 2. tb + (1 - t)a
//
// These are mathematically equivalent in theory, but the first variant does its addition and
// subtraction in the `Value` space, which is prone to overflow, e.g. if interpolating over an i8
// from -128..127. The second variant does these calculations in floating-point arithmetic, which
// may lose precision but will not fail as long as the final result fits in the original type.

macro_rules! impl_lerp_for_integer_types {
    ($($t:ty),*) => {
        $( impl Lerp for $t {
            fn lerp(&self, y1: &Self, x: f32) -> Self {
                let result_f32 = (*self as f32).lerp(&(*y1 as f32), x);
                Self::from_f32(result_f32.round())
                    .expect("Converted value was outside the valid range for this type.")
            }
        }) *
    }
}

impl_lerp_for_integer_types! { i8, i16, i32, i64, u8, u16, u32, u64, usize }

impl Lerp for f32 {
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        self * (1.0 - x) + y1 * x
    }
}

impl Lerp for f64 {
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        // Converting `x` to `f64` and doing the entire computation as f64 should be a lot more
        // accurate, yet somehow consistently produces worse results in the `lerp_wider_type` test.
        // TODO: Investigate this.
        (*self as f32 * (1.0 - x) + *y1 as f32 * x) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::fmt::Debug;

    #[test]
    fn lerp_narrow_type_full_range() {
        test_lerp(0, 255, 0.0, 0u8);
        test_lerp(0, 255, 0.25, 64u8);
        test_lerp(0, 255, 0.5, 128u8);
        test_lerp(0, 255, 1.0, 255u8);
    }

    #[test]
    fn lerp_narrow_type_partial_range() {
        test_lerp(-64, 64, 0.0, -64i8);
        test_lerp(-64, 64, 0.25, -32i8);
        test_lerp(-64, 64, 0.5, 0i8);
        test_lerp(-64, 64, 1.0, 64i8);
    }

    #[test]
    fn lerp_same_type() {
        test_lerp(0.0, 1.0, 0.0, 0.0f32);
        test_lerp(0.0, 1.0, 0.314, 0.314f32);
        test_lerp(0.0, 1.0, 1.0, 1.0f32);
        test_lerp(1.25e5, 6.77e5, 0.4, 3.458e5f32)
    }

    #[test]
    fn lerp_wider_type() {
        // Precision loss will result in small differences in the interpolated values. Compare using
        // approximate equality instead.
        assert_eq!(0.0.lerp(&1.0, 0.0), 0.0f64);
        assert_relative_eq!(0.0.lerp(&1.0, 0.314), 0.314f64, epsilon = 0.00001);
        assert_eq!(0.0.lerp(&1.0, 1.0), 1.0f64);
        assert_relative_eq!(1.25e5.lerp(&6.77e5, 0.4), 3.458e5f64, epsilon = 0.00001);
        assert_relative_eq!(1e-10.lerp(&2.5e-10, 0.123), 1.1845e-10f64, epsilon = 0.00001);
    }

    #[test]
    fn lerp_empty_range() {
        test_lerp(0.5, 0.5, 0.0, 0.5);
        test_lerp(0.5, 0.5, 0.123, 0.5);
        test_lerp(0.5, 0.5, 1.0, 0.5);
    }

    fn test_lerp<V: Debug + Lerp + PartialEq>(from: V, to: V, t: f32, expected: V) {
        assert_eq!(from.lerp(&to, t), expected);
    }
}
