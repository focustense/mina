//! Traits and implementations related to interpolation of animatable values.

use num_traits::{FromPrimitive, ToPrimitive};

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
// These are mathematically equivalent but not type/trait-equivalent. The first variant does its
// addition and subtraction in the `Value` space, which is prone to overflow, e.g. if interpolating
// over an i8 from -128..127. The second has two variants of its own:
//
// 2a. Value(t(f32(b))) + Value((1 - t)(f32(a)))
// 2b. Value(t(f32(b)) + (1 - t)(f32(a)))
//
// Variant 2a requires us to be able to multiply a Value by an f32 and return another Value. This
// does NOT work for ordinary primitives, but is good for some common types like vectors and
// matrices. Variant 2b only requires conversion of Value to and from f32, which DOES work for all
// standard primitive types.
//
// Both of these implementations are valid for different scenarios, but they overlap, for example on
// f32 itself. Until trait specialization lands, we can only implement one of them as a catch-all.
// The other has to be done manually and/or with a derive macro.
//
// Since primitives are likely to be a lot more common in style props than complex types that happen
// to support scalar multiplication, the primitive version is chosen here.

impl<Convertible> Lerp for Convertible
where
    Convertible: FromPrimitive + ToPrimitive,
{
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        let a = self.to_f32().expect("Start value does not fit in an f32");
        let b = y1.to_f32().expect("End value does not fit in an f32");
        Convertible::from_f32(a * (1.0 - x) + b * x)
            .expect("Converted value was outside the valid range for this type.")
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
        test_lerp(0, 255, 0.25, 63u8);
        test_lerp(0, 255, 0.5, 127u8);
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
