//! Contains the [`Easing`] enum which defines many standard easing types available for animations,
//! as well as an [`EasingFunction`] trait for defining custom easings.

use dyn_clone::{clone_trait_object, DynClone};
use lazy_static::lazy_static;
use lyon_geom::{CubicBezierSegment, Point};
use std::fmt::Debug;

/// Provides an easing function, AKA animation timing function, for non-linear interpolation of
/// values, typically along some curve.
///
/// Easing functions and [`Lerp`](crate::interpolation::Lerp) are complementary. `Lerp` is always
/// responsible for determining the value of a given animation property at a given time `t` (or `x`
/// in lerp terminology), but `EasingFunction` can modify which `x` value the lerp will use in its
/// evaluation. This has the same effect as using the easing function directly, because linear
/// interpolation constitutes an identity function over normalized `y`.
pub trait EasingFunction: Debug + DynClone {
    /// Computes the `y` value along the curve for a given `x` position.
    ///
    /// Expects `x` to be normalized (from 0 to 1) and returns a normalized y-value which is
    /// typically between 0 and 1, but may be outside that range (e.g. [Easing::OutBack]).
    fn calc(&self, x: f32) -> f32;
}

clone_trait_object!(EasingFunction);

/// Specifies a standard or custom [`EasingFunction`].
///
/// Available easings include:
/// - CSS standard: `Ease`, `In`, `Out`, `InOut` corresponding to `ease`, `ease-in`, `ease-out` and
///   `ease-in-out`
/// - Common easings that can be implemented with a cubic bezier function, i.e. the majority of
///   functions listed on <https://easings.net> except for the "elastic" and "bounce" types.
/// - User-defined functions via [`Custom`](Easing::Custom).
#[derive(Clone, Debug, Default)]
pub enum Easing {
    /// Linear easing, i.e. no easing or curve, only straight-line interpolation.
    #[default]
    Linear,
    /// Curve equivalent to CSS
    /// [`ease`](https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function#ease).
    Ease,
    /// Curve equivalent to CSS
    /// [`ease-in`](https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function#ease-in).
    In,
    /// Curve equivalent to CSS
    /// [`ease-out`](https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function#ease-out).
    Out,
    /// Curve equivalent to CSS
    /// [`ease-in-out`](https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function#ease-in-out).
    InOut,
    /// Sinusoidal easing that starts slowly and ends quickly. Subtle, almost linear curve.
    ///
    /// See: <https://easings.net/#easeInSine>
    InSine,
    /// Sinusoidal easing that starts quickly and ends slowly. Subtle, almost linear curve.
    ///
    /// See: <https://easings.net/#easeOutSine>
    OutSine,
    /// Sinusoidal easing that starts slowly, speeds up, and then ends slowly. Subtle, almost linear
    /// curve.
    ///
    /// See: <https://easings.net/#easeInOutSine>
    InOutSine,
    /// Quadratic (`^2`) easing that starts slowly and ends quickly. Slightly steeper curve than
    /// [`InSine`](Self::InSine).
    ///
    /// See: <https://easings.net/#easeInQuad>
    InQuad,
    /// Quadratic (`^2`) easing that starts quickly and ends slowly. Slightly steeper curve than
    /// [`OutSine`](Self::OutSine).
    ///
    /// See: <https://easings.net/#easeOutQuad>
    OutQuad,
    /// Quadratic (`^2`) easing that starts slowly, speeds up, and then ends slowly. Slightly
    /// steeper curve than [`InOutSine`](Self::InOutSine).
    ///
    /// See: <https://easings.net/#easeInOutQuad>
    InOutQuad,
    /// Cubic (`^3`) easing that starts slowly and ends quickly. Slightly steeper curve than
    /// [`InQuad`](Self::InQuad).
    ///
    /// See: <https://easings.net/#easeInCubic>
    InCubic,
    /// Cubic (`^3`) easing that starts quickly and ends slowly. Slightly steeper curve than
    /// [`OutQuad`](Self::OutQuad).
    ///
    /// See: <https://easings.net/#easeOutCubic>
    OutCubic,
    /// Cubic (`^3`) easing that starts slowly, speeds up, and then ends slowly. Slightly steeper
    /// curve than [`InOutQuad`](Self::InOutQuad).
    ///
    /// See: <https://easings.net/#easeInOutCubic>
    InOutCubic,
    /// Quartic (`^4`) easing that starts slowly and ends quickly. Slightly steeper curve than
    /// [`InCubic`](Self::InCubic).
    ///
    /// See: <https://easings.net/#easeInQuart>
    InQuart,
    /// Quartic (`^4`) easing that starts quickly and ends slowly. Slightly steeper curve than
    /// [`OutCubic`](Self::OutCubic).
    ///
    /// See: <https://easings.net/#easeOutQuart>
    OutQuart,
    /// Quartic (`^4`) easing that starts slowly, speeds up, and then ends slowly. Slightly steeper
    /// curve than [`InOutCubic`](Self::InOutCubic).
    ///
    /// See: <https://easings.net/#easeInOutQuart>
    InOutQuart,
    /// Quintic (`^5`) easing that starts slowly and ends quickly. Slightly steeper curve than
    /// [`InQuart`](Self::InQuart).
    ///
    /// See: <https://easings.net/#easeInQuint>
    InQuint,
    /// Quintic (`^5`) easing that starts quickly and ends slowly. Slightly steeper curve than
    /// [`OutQuart`](Self::OutQuart).
    ///
    /// See: <https://easings.net/#easeOutQuint>
    OutQuint,
    /// Quintic (`^5`) easing that starts slowly, speeds up, and then ends slowly. Slightly steeper
    /// curve than [`InOutQuart`](Self::InOutQuart).
    ///
    /// See: <https://easings.net/#easeInOutQuint>
    InOutQuint,
    /// Exponential easing that starts slowly and ends quickly. Steeper curve than
    /// [`InQuint`](Self::InQuint) and generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeInExpo>
    InExpo,
    /// Exponential easing that starts quickly and ends slowly. Steeper curve than
    /// [`OutQuint`](Self::OutQuint) and generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeOutExpo>
    OutExpo,
    /// Exponential easing that starts slowly, speeds up, and then ends slowly. Steeper curve than
    /// [`InOutQuint`](Self::InOutQuint) and generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeInOutExpo>
    InOutExpo,
    /// A curve that looks like the lower-right quarter of a circle. Starts slowly, and speeds up
    /// dramatically. Generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeInCirc>
    InCirc,
    /// A curve that looks like the upper-left quarter of a circle. Starts very quickly, and
    /// decelerates dramatically. Generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeOutCirc>
    OutCirc,
    /// A curve that looks like the lower-right quarter of a circle connected to the upper-left
    /// quarter of a circle. Starts very slowly, accelerates dramatically, and ends very slowly.
    /// dramatically. Generally only suitable for long animations/frames.
    ///
    /// See: <https://easings.net/#easeInOutCirc>
    InOutCirc,
    /// A curve that has similar timing to [InExpo](Easing::InExpo) but moves slightly backward
    /// (negative) before accelerating forward.
    ///
    /// See: <https://easings.net/#easeInBack>
    InBack,
    /// A curve that has similar timing to [OutExpo](Easing::OutExpo) but overshoots the terminal
    /// value (i.e. goes above 1.0) before decelerating backward and settling at the final value.
    ///
    /// See: <https://easings.net/#easeOutBack>
    OutBack,
    /// A curve that has similar timing to [InOutExpo](Easing::InOutExpo) but moves slightly
    /// backward (negative) before accelerating forward and also overshoots the terminal value (i.e.
    /// goes above 1.0) before decelerating backward and settling at the final value.
    ///
    /// See: <https://easings.net/#easeInOutBack>
    InOutBack,
    /// User-defined easing, such as an ad-hoc [CubicBezierEasing].
    Custom(Box<dyn EasingFunction>),
}

impl EasingFunction for Easing {
    fn calc(&self, x: f32) -> f32 {
        match self {
            Self::Linear => EASE_LINEAR.calc(x),
            Self::Ease => EASE_WEB.calc(x),
            Self::In => EASE_IN.calc(x),
            Self::Out => EASE_OUT.calc(x),
            Self::InOut => EASE_IN_OUT.calc(x),
            Self::InSine => EASE_IN_SINE.calc(x),
            Self::OutSine => EASE_OUT_SINE.calc(x),
            Self::InOutSine => EASE_IN_OUT_SINE.calc(x),
            Self::InQuad => EASE_IN_QUAD.calc(x),
            Self::OutQuad => EASE_OUT_QUAD.calc(x),
            Self::InOutQuad => EASE_IN_OUT_QUAD.calc(x),
            Self::InCubic => EASE_IN_CUBIC.calc(x),
            Self::OutCubic => EASE_OUT_CUBIC.calc(x),
            Self::InOutCubic => EASE_IN_OUT_CUBIC.calc(x),
            Self::InQuart => EASE_IN_QUART.calc(x),
            Self::OutQuart => EASE_OUT_QUART.calc(x),
            Self::InOutQuart => EASE_IN_OUT_QUART.calc(x),
            Self::InQuint => EASE_IN_QUINT.calc(x),
            Self::OutQuint => EASE_OUT_QUINT.calc(x),
            Self::InOutQuint => EASE_IN_OUT_QUINT.calc(x),
            Self::InExpo => EASE_IN_EXPO.calc(x),
            Self::OutExpo => EASE_OUT_EXPO.calc(x),
            Self::InOutExpo => EASE_IN_OUT_EXPO.calc(x),
            Self::InCirc => EASE_IN_CIRC.calc(x),
            Self::OutCirc => EASE_OUT_CIRC.calc(x),
            Self::InOutCirc => EASE_IN_OUT_CIRC.calc(x),
            Self::InBack => EASE_IN_BACK.calc(x),
            Self::OutBack => EASE_OUT_BACK.calc(x),
            Self::InOutBack => EASE_IN_OUT_BACK.calc(x),
            Self::Custom(custom) => custom.calc(x),
        }
    }
}

lazy_static! {
    static ref EASE_LINEAR: LinearEasing = LinearEasing;
    static ref EASE_WEB: CubicBezierEasing = cubic_bezier(0.25, 0.1, 0.25, 1.0);
    static ref EASE_IN: CubicBezierEasing = cubic_bezier(0.42, 0.0, 1.0, 1.0);
    static ref EASE_OUT: CubicBezierEasing = cubic_bezier(0.0, 0.0, 0.58, 1.0);
    static ref EASE_IN_OUT: CubicBezierEasing = cubic_bezier(0.42, 0.0, 0.58, 1.0);
    static ref EASE_IN_SINE: CubicBezierEasing = cubic_bezier(0.12, 0.0, 0.39, 0.0);
    static ref EASE_OUT_SINE: CubicBezierEasing = cubic_bezier(0.61, 1.0, 0.88, 1.0);
    static ref EASE_IN_OUT_SINE: CubicBezierEasing = cubic_bezier(0.37, 0.0, 0.63, 1.0);
    static ref EASE_IN_QUAD: CubicBezierEasing = cubic_bezier(0.11, 0.0, 0.5, 0.0);
    static ref EASE_OUT_QUAD: CubicBezierEasing = cubic_bezier(0.5, 1.0, 0.89, 1.0);
    static ref EASE_IN_OUT_QUAD: CubicBezierEasing = cubic_bezier(0.45, 0.0, 0.55, 1.0);
    static ref EASE_IN_CUBIC: CubicBezierEasing = cubic_bezier(0.32, 0.0, 0.67, 0.0);
    static ref EASE_OUT_CUBIC: CubicBezierEasing = cubic_bezier(0.33, 1.0, 0.68, 1.0);
    static ref EASE_IN_OUT_CUBIC: CubicBezierEasing = cubic_bezier(0.65, 0.0, 0.35, 1.0);
    static ref EASE_IN_QUART: CubicBezierEasing = cubic_bezier(0.5, 0.0, 0.75, 0.0);
    static ref EASE_OUT_QUART: CubicBezierEasing = cubic_bezier(0.25, 1.0, 0.5, 1.0);
    static ref EASE_IN_OUT_QUART: CubicBezierEasing = cubic_bezier(0.76, 0.0, 0.24, 1.0);
    static ref EASE_IN_QUINT: CubicBezierEasing = cubic_bezier(0.64, 0.0, 0.78, 0.0);
    static ref EASE_OUT_QUINT: CubicBezierEasing = cubic_bezier(0.22, 1.0, 0.36, 1.0);
    static ref EASE_IN_OUT_QUINT: CubicBezierEasing = cubic_bezier(0.83, 0.0, 0.17, 1.0);
    static ref EASE_IN_EXPO: CubicBezierEasing = cubic_bezier(0.7, 0.0, 0.84, 0.0);
    static ref EASE_OUT_EXPO: CubicBezierEasing = cubic_bezier(0.16, 1.0, 0.3, 1.0);
    static ref EASE_IN_OUT_EXPO: CubicBezierEasing = cubic_bezier(0.87, 0.0, 0.13, 1.0);
    static ref EASE_IN_CIRC: CubicBezierEasing = cubic_bezier(0.55, 0.0, 1.0, 0.45);
    static ref EASE_OUT_CIRC: CubicBezierEasing = cubic_bezier(0.0, 0.55, 0.45, 1.0);
    static ref EASE_IN_OUT_CIRC: CubicBezierEasing = cubic_bezier(0.85, 0.0, 0.15, 1.0);
    static ref EASE_IN_BACK: CubicBezierEasing = cubic_bezier(0.36, 0.0, 0.66, -0.56);
    static ref EASE_OUT_BACK: CubicBezierEasing = cubic_bezier(0.34, 1.56, 0.64, 1.0);
    static ref EASE_IN_OUT_BACK: CubicBezierEasing = cubic_bezier(0.68, -0.6, 0.32, 1.6);
}

/// Linear easing which returns the `x` value as the `y` result. Has the same behavior as
/// [Easing::Linear] or [Easing::default].
#[derive(Clone, Debug)]
pub struct LinearEasing;

impl EasingFunction for LinearEasing {
    fn calc(&self, x: f32) -> f32 {
        x
    }
}

/// Easing function defined by a cubic bezier curve with the start and end points fixed at `(0, 0)`
/// and `(1, 1)`, i.e. only the control points are specified.
///
/// Most standard easing functions use `CubicBezierEasing`. Instances of this may be created and
/// used in [Easing::Custom] in cases where the standard easings do not suffice.
#[derive(Clone, Debug)]
pub struct CubicBezierEasing {
    segment: CubicBezierSegment<f32>,
}

impl CubicBezierEasing {
    /// Creates a new [CubicBezierEasing] with control points `(x1, y1)` and `(x2, y2)`.
    ///
    /// To experiment with different curves, see: <https://cubic-bezier.com/>
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            segment: CubicBezierSegment {
                from: Point::new(0.0, 0.0),
                to: Point::new(1.0, 1.0),
                ctrl1: Point::new(x1, y1),
                ctrl2: Point::new(x2, y2),
            },
        }
    }
}

impl EasingFunction for CubicBezierEasing {
    fn calc(&self, x: f32) -> f32 {
        self.segment.y(x)
    }
}

fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> CubicBezierEasing {
    CubicBezierEasing::new(x1, y1, x2, y2)
}
