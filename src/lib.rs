//! Mina is a framework-independent animation library focused on ergonomics, aiming to bring the
//! simplicity and versatility of CSS transitions and animations to Rust.
//!
//! # Features
//!
//! - Turn any ordinary `struct` into an animator or animated type.
//! - Create state-driven animators that automatically blend between states.
//! - Provides (almost) all standard [easings](https://easings.net/) out of the box, with the option
//!   to add custom curves or functions.
//! - Define animations using a concise, CSS-like syntax, and state transitions using a style
//!   similar to CSS pseudo-classes.
//! - Animate any property type that supports [linear interpolation](crate::Lerp).
//! - Easily specify delayed, repeating or reversing animations.
//! - Merge heterogeneous animations/transitions into a single timeline; e.g. define a _single_
//!   animation that pulses in and out infinitely but also scales or slides in only once.
//! - Use with any GUI or creative coding environment -
//!   [integration examples](https://github.com/focustense/mina/tree/main/examples) are provided for
//!   [nannou](https://nannou.cc/), [bevy](https://bevyengine.org/) and
//!   [iced](https://github.com/iced-rs/iced).
//!
//! # Timeline Example
//!
//! The [`Timeline`] is the most basic abstraction and defines a single, state-independent animation
//! over a time axis.
//!
//! Suppose we want to animate both the size and position of some entity over time. It will be small
//! at each edge and largest in the middle:
//!
//! ```
//! use mina::prelude::*;
//!
//! #[derive(Animate, Clone, Debug, Default, PartialEq)]
//! struct Style {
//!   x: i32,
//!   size: u32,
//! }
//!
//! let timeline = timeline!(Style 10s
//!     from { x: -200, size: 10 }
//!     50% { x: 0, size: 20 }
//!     to { x: 200, size: 10});
//!
//! let mut style = Style::default();
//! timeline.update(&mut style, 2.5);
//! assert_eq!(style, Style { x: -100, size: 15 });
//! ```
//!
//! Note: in the above code, [`Clone`] and [`Default`] are required traits for any timeline-able
//! type, but [`Debug`] and [`PartialEq`] are only needed for the assertions and are not required
//! for regular usage.
//!
//! `from` and `to` are aliases for `0%` and `100%` respectively. Either may be used, but the former
//! are more idiomatic and tend to improve readability.
//!
//! # Animator Example
//!
//! [`StateAnimator`] types own many timelines, as well as the style or other structure being
//! animated, and are meant to be driven directly by an event loop. Instead of requesting the
//! properties at a particular time, as in the [`Timeline`] example above, you interact with
//! animators by notifying them of elapsed time and state changes.
//!
//! Suppose we are designing an animated button; when hovered, it receives an elevation, and when
//! pressed, it slightly increases in size.
//!
//! ```
//! use mina::prelude::*;
//!
//! #[derive(Animate, Clone, Debug, Default, PartialEq)]
//! struct Style {
//!     elevation: f32,
//!     scale: f32,
//! }
//!
//! #[derive(Clone, Default, PartialEq, State)]
//! enum State {
//!     #[default] Idle,
//!     Hovered,
//!     Pressed,
//! }
//!
//! let mut animator = animator!(Style {
//!     default(State::Idle, { elevation: 0.0, scale: 1.0 }),
//!     State::Idle => 0.25s to default,
//!     State::Hovered => 0.5s to { elevation: 5.0, scale: 1.0 },
//!     State::Pressed => 0.1s to { scale: 1.1 }
//! });
//!
//! assert_eq!(animator.current_values(), &Style { elevation: 0.0, scale: 1.0 }); // Default
//! animator.advance(1.0); // No change in state
//! assert_eq!(animator.current_values(), &Style { elevation: 0.0, scale: 1.0 });
//! animator.set_state(&State::Hovered);
//! assert_eq!(animator.current_values(), &Style { elevation: 0.0, scale: 1.0 }); // No time elapsed
//! animator.advance(0.25);
//! assert_eq!(animator.current_values(), &Style { elevation: 2.5, scale: 1.0 });
//! animator.set_state(&State::Pressed); // Change state before animation is finished
//! assert_eq!(animator.current_values(), &Style { elevation: 2.5, scale: 1.0 }); // No time elapsed
//! animator.advance(0.05);
//! assert_eq!(animator.current_values(), &Style { elevation: 2.5, scale: 1.05 });
//! ```
//!
//! The [`Clone`], [`Default`] and [`PartialEq`] traits **are all** required for any type to be used
//! as an animator state. [`State`] is an alias for
//! [Enum](https://docs.rs/enum-map/latest/enum_map/trait.Enum.html), and currently required for the
//! [`EnumStateAnimator`] and [`animator`] macro.
//!
//! The `default(state, props)` line is not required, but supported for specific and relatively
//! common cases where the default resting values _for that animation_ do not match the [`Default`]
//! implementation for the corresponding `struct`, or in the (less common) case that the default
//! _state_ for the animation should be different from the default enum member. In the above
//! example, the derived `Default` implementation would give a `scale` of `0.0`, but the normal
//! scale of the button should be `1.0`, so we override the default.
//!
//! The same `default` term _within_ an animation state has a different meaning, and is interpreted
//! as "use the default (for this animator) values for this keyframe". This helps avoid repetitive,
//! copy-paste code for  default/idle states that should simply return the widget to its base state.
//!
//! Note how the `Hovered` state specifies a `scale` that is the same as the default. The reason for
//! this is to tell the animator that when transitioning from `Pressed` back to `Hovered`, it should
//! revert the scale transform used for `Pressed`. Mina does **not** assume that undefined values in
//! a keyframe should use defaults, and this is a very important property for merged timelines and
//! more advanced animations in general. If a keyframe is missing one or more properties, those
//! properties are _ignored_:
//! - If there are earlier or later keyframes that do specify them, then it will animate smoothly
//!   between those keyframes. This applies to both [`Timeline`] and [`StateAnimator`].
//! - If any given state does not specify any keyframes at all with some properties, then the
//!   properties will _not animate_ when in that state; they will remain at whichever values the
//!   previous state left them in.
//!
//! The actual implementation of `elevation` and `scale` are up to the underlying GUI. Mina doesn't
//! care about the meaning of these properties, it just animates their values; the plumbing will
//! vary with the specific GUI in use. Future updates may include standard integrations with those
//! GUIs, but for now, the [examples](https://github.com/focustense/mina/tree/main/examples)
//! directory serves as the unofficial integration how-to guide, as well as the repository for more
//! complex and interesting uses of the API.
//!
//! # Event Loop
//!
//! Mina doesn't use its own event loop, so that it can instead be integrated into the event loop of
//! whichever GUI is actually in use. This also allows global customizations - for example, stopping
//! all animations when a game is paused, or playing them in slow motion during some key event.
//!
//! In most cases, establishing the event loop is a one- or two-line function. Refer to the
//! [examples](https://github.com/focustense/mina/tree/main/examples) for framework-specific
//! patterns.

pub mod prelude;

pub use mina_core::{
    animator::{EnumStateAnimator, State, StateAnimator, StateAnimatorBuilder},
    easing::{Easing, EasingFunction},
    interpolation::Lerp,
    timeline::{
        Animate, Keyframe, KeyframeBuilder, MergedTimeline, Repeat, Timeline,
        TimelineBuilder, TimelineConfiguration,
    },
};

#[doc(hidden)]
pub use mina_core::{
    time_scale::TimeScale,
    timeline::{prepare_frame, TimelineBuilderArguments, TimelineOrBuilder},
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
/// let mut animator = animator!(Style {
///     default(State::Idle, { alpha: 0.5, size: 60 }),
///     State::Idle => 2s Easing::OutQuad to default,
///     State::Active => 1s Easing::Linear to { alpha: 1.0, size: 80 }
/// });
///
/// animator.advance(12.0);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
/// animator.set_state(&State::Active);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
/// animator.advance(0.5);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.75, size: 70 });
/// animator.set_state(&State::Idle);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.75, size: 70 });
/// animator.advance(0.8);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.554, size: 62 });
/// animator.advance(1.2);
/// assert_eq!(animator.current_values(), &Style { alpha: 0.5, size: 60 });
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
///
/// # Remote
///
/// Since it is not possible to run a derive macro on an external type, a `remote` attribute exists
/// to support the same auto-implemented [Timeline] and [TimelineBuilder] types for external types.
///
/// One example, which also appears in some of the
/// [examples](https://github.com/focustense/mina/tree/main/examples/bevy_app), is
/// [Bevy's](bevyengine.org) `Transform` type. It is very common to want to animate this component,
/// and all its properties are [Lerp]able, but because it is a foreign type, it cannot be used
/// directly with `#[derive(Animate)]`.
///
/// Note that this is not primarily a problem with Rust's
/// [orphan rule](https://doc.rust-lang.org/book/traits.html#rules-for-implementing-traits), since
/// `Animate` does not need to add any traits to the original struct; it is simply that any derive
/// macro needs to run on the declaration.
///
/// To work around this, we can define a proxy type with the same members as the type we wish to
/// animate, and specify that it is remote:
///
/// ```
/// // In real-world usage, this would be in another crate.
/// mod external {
///     pub struct Style {
///         pub alpha: f32,
///         pub size: u16,
///     }
/// }
///
/// use external::Style;
/// use mina::prelude::*;
///
/// #[derive(Animate)]
/// #[animate(remote = "Style")]
/// struct StyleProxy {
///     alpha: f32,
///     size: u16,
/// }
///
/// let fade_in = timeline!(StyleProxy 5s from { alpha: 0.1 } to { alpha: 1.0 });
/// let mut style = Style { alpha: 0.5, size: 12 };
/// fade_in.update(&mut style, 1.0);
///
/// assert_eq!(style.alpha, 0.28);
/// assert_eq!(style.size, 12);
/// ```
///
/// Note that although we must specify `StyleProxy` as the timeline target in macro usage (or when
/// calling builder methods manually), the resulting [Timeline] instance has a target of `Style` and
/// operates directly on `Style` structs. There is no need to pass around newtypes or other
/// wrappers, or provide any conversion methods.
pub use mina_macros::Animate;

/// Configures and creates a [`Timeline`] for an [`Animate`](macro@Animate) type.
///
/// Provides a more ergonomic, CSS-like alternative to the builder syntax using
/// [`TimelineConfiguration`] and [`TimelineBuilder`], producing the same result. Requires an
/// [`Animate`] type for the values.
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
/// let timeline = timeline!(Style 2s reverse Easing::Out
///     from { alpha: 0.5, size: 50 }
///     to { alpha: 1.0, size: 100 });
///
/// let mut values = Style::default();
/// timeline.update(&mut values, 0.25);
/// assert_eq!(values, Style { alpha: 0.578125, size: 58 });
/// timeline.update(&mut values, 0.5);
/// assert_eq!(values, Style { alpha: 0.75, size: 75 });
/// timeline.update(&mut values, 1.0);
/// assert_eq!(values, Style { alpha: 1.0, size: 100 });
/// timeline.update(&mut values, 1.25);
/// assert_eq!(values, Style { alpha: 0.921875, size: 92 });
/// timeline.update(&mut values, 1.5);
/// assert_eq!(values, Style { alpha: 0.75, size: 75 });
/// timeline.update(&mut values, 2.0);
/// assert_eq!(values, Style { alpha: 0.5, size: 50 });
/// ```
pub use mina_macros::timeline;
