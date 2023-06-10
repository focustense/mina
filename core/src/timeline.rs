//! Creation and consumption of [`Timeline`] instances.

use crate::easing::Easing;
use crate::time_scale::{TimeScale, TimeScalePosition};
use std::fmt::Debug;

/// An animator timeline.
pub trait Timeline {
    /// The target type that holds the set of animation properties. This is the original type from
    /// which the timeline was derived, _not_ the generated `AnimatorValues` type.
    type Target;

    /// Changes this timeline to start with a different set of values from the defaults that it was
    /// originally configured with.
    ///
    /// Does not affect delay, repeat, or other timing properties; only the keyframes at the 0%
    /// position are changed.
    ///
    /// This is typically used when blending animations; the newly-active timeline begins where the
    /// previously-active timeline ended or was interrupted.
    ///
    /// # Arguments
    ///
    /// * `values` - New values where the timeline should start, replacing previous defaults.
    fn start_with(&mut self, values: &Self::Target);

    /// Updates a set of animator values to represent the timeline at a given `time`.
    ///
    /// Properties that are not included in the timeline will not be updated.
    ///
    /// # Arguments
    ///
    /// * `target` - Target containing animatable values to update.
    /// * `time` - Time in the same unit scale as the timeline's duration, generally seconds.
    fn update(&self, values: &mut Self::Target, time: f32);
}

/// Trait for a builder that creates typed [`Timeline`] instances.
///
/// This is meant to be implemented for the specific [`TimelineConfiguration`] whose generic
/// argument is the [`Keyframe`] data on which the `T` timeline type is based. It is a generic
/// trait, rather than an associated trait, so that code in external crates can implement for the
/// `TimelineConfiguration` which is owned by this crate. Note that the
/// [`Animate`](../../mina_macros/derive.Animate.html) macro handles this generation automatically.
pub trait TimelineBuilder<T: Timeline> {
    /// Builds a timeline, consuming the builder in the process.
    fn build(self) -> T;
}

/// Common trait shared by both [`Timeline`] and [`TimelineBuilder`] types.
///
/// [`StateAnimatorBuilder`](crate::animator::StateAnimatorBuilder) uses this so that it can accept
/// either type, building new timelines as necessary; this reduces the number of type annotations
/// and chained calls necessary to complete the animator.
pub trait TimelineOrBuilder<T: Timeline> {
    /// Returns the built timeline as a [`MergedTimeline`].
    ///
    /// - If the underlying instance is already a [`MergedTimeline`], then this returns itself.
    /// - If it is a regular non-merged [`Timeline`], returns a [`MergedTimeline`] with this
    ///   [`Timeline`] as its only delegate.
    /// - If it is a [`TimelineBuilder`], builds the timeline and returns a [`MergedTimeline`] with
    ///   the built timeline as its only delegate.
    ///
    /// Note: [`MergedTimeline`] is used instead of `T` as the return type because every
    /// [`Timeline`] can be implicitly converted into a [`MergedTimeline`], and doing so allows
    /// a [`StateAnimatorBuilder`](crate::animator::StateAnimatorBuilder) to mix both simple and
    /// merged timelines in a single chain.
    fn build(self) -> MergedTimeline<T>;
}

impl<T: Timeline> TimelineOrBuilder<T> for MergedTimeline<T> {
    fn build(self) -> MergedTimeline<T> {
        self
    }
}

/// Consolidated arguments for building a specific type of [`Timeline`], derived from the
/// [`TimelineConfiguration`].
///
/// This is a sort of "bridge struct" that obtains everything a [`TimelineBuilder`] needs to
/// actually create its timeline, without having to expose the private data of a
/// [`TimelineConfiguration`] to external crates. It is used by the
/// [`Animate`](../../mina_macros/derive.Animate.html) macro.
pub struct TimelineBuilderArguments<Data: Clone + Debug> {
    /// The normalized times corresponding to the original [`Keyframe`] positions. This has the same
    /// times and order as the original keyframes but does not include any other keyframe data,
    /// since the other keyframe data gets parsed into
    /// [`SubTimeline`](crate::timeline_helpers::SubTimeline) instances by the builder.
    pub boundary_times: Vec<f32>,
    /// Default easing for the timeline. Same as the [`TimelineConfiguration::default_easing`].
    pub default_easing: Easing,
    /// Full sequence of keyframes owned by the [`TimelineConfiguration`].
    pub keyframes: Vec<Keyframe<Data>>,
    /// Timing information derived from the various [`TimelineConfiguration`] properties including
    /// [`duration_seconds`](TimelineConfiguration::duration_seconds),
    /// [`delay_seconds`](TimelineConfiguration::delay_seconds),
    /// [`repeat`](TimelineConfiguration::repeat) and [`reverse`](TimelineConfiguration::reverse).
    pub timescale: TimeScale,
}

impl<Data: Clone + Debug> From<TimelineConfiguration<Data>> for TimelineBuilderArguments<Data> {
    fn from(value: TimelineConfiguration<Data>) -> Self {
        let mut args = Self {
            timescale: value.create_timescale(),
            boundary_times: value.get_boundary_times(),
            default_easing: value.default_easing,
            keyframes: value.keyframes,
        };
        args.keyframes.sort_by(|a, b| a.normalized_time.total_cmp(&b.normalized_time));
        args
    }
}

/// Configuration and fluent builder interface for a [`Timeline`] type.
///
/// Works with [`TimelineBuilder`] to aid in the creation of timelines. `TimelineBuilder` cannot be
/// implemented ahead of time, because it depends on the specific set of animation properties;
/// to complete the API, applications (or the [`Animate`](../../mina_macros/derive.Animate.html)
/// macro) define an implementation of `TimelineBuilder` for the `TimelineConfiguration` whose
/// keyframe type corresponds to the specific timeline being created.
///
/// Refer to the `macroless_timeline` example for details on how the two are connected.
#[derive(Clone, Debug)]
pub struct TimelineConfiguration<Data: Clone + Debug> {
    default_easing: Easing,
    delay_seconds: f32,
    duration_seconds: f32,
    keyframes: Vec<Keyframe<Data>>,
    repeat: Repeat,
    reverse: bool,
}

impl<Data: Clone + Debug> Default for TimelineConfiguration<Data> {
    fn default() -> Self {
        Self {
            default_easing: Easing::default(),
            delay_seconds: 0.0,
            duration_seconds: 1.0,
            keyframes: Vec::new(),
            repeat: Repeat::None,
            reverse: false,
        }
    }
}

impl<Data: Clone + Debug> TimelineConfiguration<Data> {
    /// Configures the default easing for this timeline.
    ///
    /// The default easing is applied until a [`Keyframe`] overrides it. Once a frame specifies its
    /// own easing, that becomes the new default until another frame overrides it again, etc. If no
    /// keyframes specify their own easing, then this easing applies to every frame.
    pub fn default_easing(mut self, default_easing: Easing) -> Self {
        self.default_easing = default_easing;
        self
    }

    /// Configures the delay, in seconds, before the animation will start.
    ///
    /// Delay is applied once at the beginning of the timeline, and does not contribute to the
    /// [`duration_seconds`](Self::duration_seconds), nor does it apply to any cycles after the
    /// first if a [`repeat`](Self::repeat) setting is specified.
    pub fn delay_seconds(mut self, delay_seconds: f32) -> Self {
        self.delay_seconds = delay_seconds;
        self
    }

    /// Configures the animation duration, in seconds.
    ///
    /// If the animation [repeats](Self::repeat), this is the duration of each cycle. If the
    /// animation [reverses](Self::reverse), regardless of whether or not it repeats, then the first
    /// half of the duration is used for the forward animation and the second half is used for the
    /// reverse animation.
    pub fn duration_seconds(mut self, duration_seconds: f32) -> Self {
        self.duration_seconds = duration_seconds;
        self
    }

    /// Adds a single [`Keyframe`] to the animation, using the supplied builder to create the
    /// keyframe along with its specific typed data.
    pub fn keyframe(mut self, builder: impl KeyframeBuilder<Data = Data>) -> Self {
        self.keyframes.push(builder.build());
        self
    }

    /// Configures the number of repetitions (cycles).
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Configures whether or not the animation should automatically reverse.
    ///
    /// Reversing takes up the second half of any given cycle and uses the same keyframes, easing
    /// and other timing properties as the normal forward animation.
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    fn create_timescale(&self) -> TimeScale {
        TimeScale::new(
            self.duration_seconds,
            self.delay_seconds,
            self.repeat,
            self.reverse,
        )
    }

    fn get_boundary_times(&self) -> Vec<f32> {
        self.keyframes.iter().map(|k| k.normalized_time).collect()
    }
}

/// A single frame of an animation timeline, specifying some or all of the animation property values
/// at a given point in time.
///
/// Keyframes are actually an intermediate type used by the
/// [`Animate`](../../mina_macros/derive.Animate.html) macro when constructing [`Timeline`]
/// instances. They are not meant to be created or consumed directly. Instead, the `Animate`
/// decorated type will expose trait functions for creating keyframes as part of the timeline
/// builder.
#[derive(Clone, Debug)]
pub struct Keyframe<Data: Clone> {
    pub(super) data: Data,
    pub(super) easing: Option<Easing>,
    pub(super) normalized_time: f32,
}

impl<Data: Clone> Keyframe<Data> {
    /// Creates a new keyframe.
    ///
    /// This function is intended for use by [`KeyframeBuilder`] implementations and should normally
    /// not be needed by user code.
    ///
    /// # Arguments
    ///
    /// * `normalized_time` - Position of the keyframe on a normalized time scale from `0.0` (0%) to
    ///   `1.0` (100%).
    /// * `data` - Data for this keyframe, normally a struct with [`Option`] fields generated by the
    ///   [`Animate`](../../mina_macros/derive.Animate.html) macro.
    /// * `easing` - Easing function to use for this keyframe, and subsequent keyframes that do not
    ///   provide their own `easing`. Specifying `None` will cause the keyframe to use the easing of
    ///   the previous keyframe, or if there are no previous keyframes, then the default easing for
    ///   the timeline containing the keyframe.
    pub fn new(normalized_time: f32, data: Data, easing: Option<Easing>) -> Self {
        Self {
            normalized_time,
            data,
            easing,
        }
    }
}

/// Builder interface for creating a typed [`Keyframe`].
///
/// Implementations will normally expose additional builder-type methods to configure the animation
/// property values; this trait only encapsulates the behavior common to all keyframes.
pub trait KeyframeBuilder {
    /// Data type (animation properties) that the keyframe will hold.
    type Data: Clone + Debug;

    /// Creates a [`Keyframe`] from this builder.
    fn build(&self) -> Keyframe<Self::Data>;

    /// Configures the easing that will be used starting from the beginning of this keyframe, and
    /// applying to all subsequent keyframes until another one specifies its own `easing`.
    fn easing(self, easing: Easing) -> Self;
}

/// A [Timeline] that is composed of multiple inner timelines.
///
/// Merged timelines are useful in scenarios where a single animation behavior is difficult to
/// specify purely in terms of keyframes - for example, if different properties should animate with
/// different easing functions but share the same keyframe times, or if there will be different
/// animations that each have entirely different timescales, e.g. one loops/reverses and the other
/// does not, or the cycle durations are different.
///
/// A common example would be a spinner-like widget that fades in briefly, but also has a repeating
/// progress animation (say rotation). This relationship cannot be described by the keyframes of a
/// single timeline because [`repeat`](TimelineConfiguration::repeat) and
/// [`reverse`](TimelineConfiguration::reverse) are determined for the entire timeline. However, it
/// can be easily represented by a merged timeline whose constituent parts each have keyframes
/// referring to only one of the "parts", either rotation or alpha.
///
/// Refer to the tests and the `merged_timeline` example for details and usage.
pub struct MergedTimeline<T: Timeline> {
    timelines: Vec<T>,
}

impl<T: Timeline> MergedTimeline<T> {
    /// Creates a [`MergedTimeline`] using a sequence of component [`Timeline`]s.
    ///
    /// Timelines are queried in sequential order, meaning that if a merged timeline is created from
    /// `[t1, t2]`, and they each have a value for property `foo` at a given point in time, then
    /// only the value from `t2` is used; the values from `t1` and `t2` are **not** blended in any
    /// way. If `t2` does not have a value for the property, but `t1` does, then `t1` is used.
    ///
    /// Any number of timelines can be merged, but generally they should not overlap in the
    /// properties that they animate, otherwise the above-mentioned precedence rule above may
    /// produce unexpected outcomes.
    pub fn of(timelines: impl IntoIterator<Item = T>) -> Self {
        Self {
            timelines: timelines.into_iter().collect(),
        }
    }
}

impl<T: Timeline + Clone> Clone for MergedTimeline<T> {
    fn clone(&self) -> Self {
        MergedTimeline::of(self.timelines.iter().cloned())
    }
}

impl<T: Timeline> From<T> for MergedTimeline<T> {
    fn from(value: T) -> Self {
        MergedTimeline::of([value])
    }
}

impl<T: Timeline> Timeline for MergedTimeline<T> {
    type Target = T::Target;

    fn start_with(&mut self, values: &Self::Target) {
        for timeline in self.timelines.iter_mut() {
            timeline.start_with(values);
        }
    }

    fn update(&self, values: &mut Self::Target, time: f32) {
        for timeline in &self.timelines {
            timeline.update(values, time);
        }
    }
}

/// Describes the looping behavior of an animation timeline.
#[derive(Clone, Copy, Debug, Default)]
pub enum Repeat {
    /// Animation does not repeat; it plays once and then ends.
    #[default]
    None,
    /// Animation repeats for a given number of cycles, looping or reversing back to the beginning
    /// each time. Ends after the last cycle is completed.
    Times(u32),
    /// Animation repeats infinitely and never ends, looping or reversing back to the beginning each
    /// time it repeats.
    Infinite,
}

/// Helper function typically used by [`Timeline`] implementations at the beginning of their
/// [`update`](Timeline::update) method, which performs lookup tasks common to all timelines,
/// including converting real time to normalized time and finding the closest frame.
///
/// Encapsulates all of the generic logic that does _not_ require knowing the specific
/// [SubTimeline](crate::timeline_helpers::SubTimeline) fields and types.
pub fn prepare_frame(
    time: f32,
    boundary_times: &[f32],
    timescale: &TimeScale,
) -> Option<(f32, usize)> {
    if boundary_times.is_empty() {
        return None;
    }
    let normalized_time = match timescale.get_position(time) {
        TimeScalePosition::Active(t, _) => t,
        TimeScalePosition::NotStarted => 0.0,
        TimeScalePosition::Ended(t) => t,
    };
    let frame_index = match boundary_times.binary_search_by(|t| t.total_cmp(&normalized_time)) {
        Ok(index) => index,
        Err(next_index) => next_index.max(1) - 1,
    };
    Some((normalized_time, frame_index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ordered_float::OrderedFloat;
    use std::collections::HashMap;

    #[derive(Debug, Default, PartialEq)]
    struct TestValues {
        foo: u8,
        bar: u32,
        baz: f32,
    }

    // Setting up a timeline without proc macros requires a lot of boilerplate, so for the purposes
    // of testing merged timelines, we instead  use fake timelines here. The stub is only capable of
    // producing exact matches, i.e. does not interpolate between times.
    struct StubTimeline {
        frames: HashMap<OrderedFloat<f32>, StubFrame>,
    }

    impl StubTimeline {
        fn new() -> Self {
            Self {
                frames: HashMap::new(),
            }
        }

        fn add_frame(
            mut self,
            time: f32,
            foo: Option<u8>,
            bar: Option<u32>,
            baz: Option<f32>,
        ) -> Self {
            self.frames
                .insert(OrderedFloat(time), StubFrame { foo, bar, baz });
            self
        }
    }

    impl Timeline for StubTimeline {
        type Target = TestValues;

        fn start_with(&mut self, values: &Self::Target) {
            if let Some(first_frame) = self.frames.get_mut(&OrderedFloat(0.0)) {
                first_frame.foo = Some(values.foo);
                first_frame.bar = Some(values.bar);
                first_frame.baz = Some(values.baz);
            }
        }

        fn update(&self, values: &mut Self::Target, time: f32) {
            if let Some(frame) = self.frames.get(&OrderedFloat(time)) {
                if let Some(foo) = frame.foo {
                    values.foo = foo;
                }
                if let Some(bar) = frame.bar {
                    values.bar = bar;
                }
                if let Some(baz) = frame.baz {
                    values.baz = baz;
                }
            }
        }
    }

    struct StubFrame {
        foo: Option<u8>,
        bar: Option<u32>,
        baz: Option<f32>,
    }

    #[test]
    fn merged_timeline_delegates_to_component_timelines() {
        let timeline1 = StubTimeline::new()
            .add_frame(0.1, Some(10), Some(555), Some(0.12))
            .add_frame(0.2, Some(20), None, None)
            .add_frame(0.3, Some(30), Some(777), None);
        let timeline2 = StubTimeline::new()
            .add_frame(0.1, None, None, Some(1.5))
            .add_frame(0.2, None, None, Some(2.5))
            .add_frame(0.3, None, None, Some(6.8));
        let merged_timeline = MergedTimeline::of([timeline1, timeline2]);

        let mut values = <[TestValues; 3]>::default();
        merged_timeline.update(&mut values[0], 0.1);
        merged_timeline.update(&mut values[1], 0.2);
        merged_timeline.update(&mut values[2], 0.3);

        assert_eq!(
            values[0],
            TestValues {
                foo: 10,
                bar: 555,
                baz: 1.5
            }
        );
        assert_eq!(
            values[1],
            TestValues {
                foo: 20,
                bar: 0,
                baz: 2.5
            }
        );
        assert_eq!(
            values[2],
            TestValues {
                foo: 30,
                bar: 777,
                baz: 6.8
            }
        );
    }
}
