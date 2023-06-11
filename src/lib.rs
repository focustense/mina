pub mod prelude;

pub use mina_core::{
    animator::{EnumStateAnimator, State, StateAnimator, StateAnimatorBuilder},
    easing::{Easing, EasingFunction},
    interpolation::Lerp,
    time_scale::TimeScale,
    timeline::{
        prepare_frame, Keyframe, KeyframeBuilder, MergedTimeline, Repeat, Timeline,
        TimelineBuilder, TimelineBuilderArguments, TimelineConfiguration, TimelineOrBuilder,
    },
    timeline_helpers::SubTimeline,
};

/// Configures and creates a [`StateAnimator`] for an [`Animate`](macro@Animate) type.
///
/// Animators hold one or more [`Timeline`] instances mapped to particular state values and will
/// automatically switch and blend timelines when the state is changed. The `animator` macro uses a
/// CSS-like syntax to define the states and timelines.
///
/// Use of this macro requires both an [`Animate`] type for the values/timeline and a
/// [`State`] derived type for the state (note: `State` is a re-export of the `Enum` derive macro
/// from the [`enum-map`](https://crates.io/crates/enum-map) crate; you will need to add this  crate
/// to the downstream project in order for `State` to compile. In addition, the state type must
/// implement [`Clone`](std::clone::Clone), [`Default`](std::default::Default) and
/// [`PartialEq`](std::cmp::PartialEq).
///
/// # Example
///
/// ```
/// use mina::prelude::*;
///
/// #[derive(Clone, Default, PartialEq, State)]
/// enum State {
///     #[default] Idle,
///     Active,
/// }
///
/// #[derive(Animate, Clone, Debug, Default, PartialEq)]
/// struct Style {
///     alpha: f32,
///     size: u16,
/// }
///
/// fn main() {
///     let mut animator = animator!(Style {
///         default(State::Idle, { alpha: 0.5, size: 60 }),
///         State::Idle => 2s Easing::OutQuad to default,
///         State::Active => 1s Easing::Linear to { alpha: 1.0, size: 80 }
///     });
///
///     animator.advance(12.0);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
///     animator.set_state(&State::Active);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
///     animator.advance(0.5);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.75, size: 70 });
///     animator.set_state(&State::Idle);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.75, size: 70 });
///     animator.advance(0.8);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.554, size: 62 });
///     animator.advance(1.2);
///     assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
/// }
/// ```
pub use mina_macros::animator;

/// Sets up a type for animation.
///
/// Animatable types gain two functions:
/// - `timeline()` creates a [`TimelineBuilder`] that can be used to build a single animation
///   [`Timeline`]. Timelines provide the interpolates values of the animatable type at any
///   arbitrary point in time.
/// - `keyframe()` creates a [`KeyframeBuilder`] which is used to provide [`Keyframe`] instances to
///   the timeline builder. Keyframes specify the exact values at a specific point in the timeline.
///
/// In addition, a specific timeline type is generated with a `Timeline` suffix; for example, if the
/// name of the type is `Style`, then the generated timeline type will be `StyleTimeline`. This type
/// will have the same visibility as the animatable type, and can be used directly to store the
/// timeline, or an animator based on the timeline, without boxing.
///
/// Making a type animatable also allows it to be used with the [`animator`](macro@animator) macro.
///
/// The following requirements apply to any type decorated with `#[derive(Animate}]`:
///
/// 1. Must be a `struct`. Tuple and `enum` types are not supported.
/// 2. Must implement the [`Clone`](std::clone::Clone) and [`Default`](std::default::Default)
///    traits.
/// 3. All _animated_ fields must implement [`Lerp`].
///    - A blanket implementation is provided for all primitive numeric types.
///    - Other types may need explicit implementations and/or a newtype for unowned types.
///    - **To exclude fields** from animation, either because it is not `Lerp`able or simply because
///      it is intended to be constant, add the `#[animate]` helper attribute to all fields which
///      _should_ be animated; any remaining fields not decorated will be ignored.
/// 4. Nested structures, `Option` fields, etc. are allowed, but will be treated as black-box, which
///    means the actual type of the field (e.g. the entire `struct`) must meet the `Lerp`
///    requirement above. This can be the desired behavior for a limited number of complex types
///    such as vectors or colors, but usually flat structs are more appropriate.
/// 5. Generic types are not supported (for now) at the `struct` level, although the individual
///    fields can be generic.
///
/// # Example
///
/// ```
/// use mina::prelude::*;
///
/// #[derive(Animate, Clone, Debug, Default, PartialEq)]
/// struct Style {
///     alpha: f32,
///     size: u16,
/// }
///
/// let timeline = Style::timeline()
///     .duration_seconds(5.0)
///     .delay_seconds(1.0)
///     .keyframe(Style::keyframe(1.0).alpha(1.0).size(25))
///     .build();
///
/// let mut values = Style::default();
/// timeline.update(&mut values, 3.0);
/// assert_eq!(values, Style { alpha: 0.4, size: 10 });
/// ```
pub use mina_macros::Animate;
