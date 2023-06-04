//! Internal helper types used in the composition of [`Timeline`](crate::timeline::Timeline)
//! implementations.
//!
//! These types are public because they need to be accessible to the timeline structs generated by
//! the [`Animate`](../../mina_macros/derive.Animate.html) macro.

use crate::{
    easing::{Easing, EasingFunction},
    interpolation::Lerp,
    timeline::Keyframe,
};

/// Partial timeline representing the animation path of a single value belonging to a collection of
/// animation properties.
///
/// In a regular, CSS-style timeline, keyframes are not required to specify all (or any) properties.
/// The expected behavior is that each property interpolates between the most recent keyframe that
/// _did_ include the property, and the earliest subsequent keyframe that includes it. The
/// implementation of this can be finicky and confusing, particularly when taking into account edge
/// cases such as not having any keyframe at position 0% or 100%, which is perfectly within spec.
///
/// Sub-timelines solve two problems:
///
/// - First, they present a timeline view that is precomputed and optimized for evaluation on every
///   frame, i.e. one that can determine in O(1) time which frames apply to any given timeline
///   position, including 0% and 100% regardless of whether or not there are real keyframes defined
///   at those positions.
///
/// - Second, since they operate on only a single value, they can be implemented as normal generic
///   structs without the use of a proc macro, which improves testability and reduces the complexity
///   of the generated timeline structs.
///
/// User code should normally not need to create or access a sub-timeline; it is an implementation
/// detail of the [`Animate`](../../mina_macros/derive.Animate.html) macro output.
#[derive(Debug)]
pub struct SubTimeline<Value> {
    frames: Vec<SplitKeyframe<Value>>,
    frame_index_map: Vec<usize>,
}

impl<Value: Clone + Lerp> SubTimeline<Value> {
    /// Extract a single-valued sub-timeline from a sequence of multi-valued keyframes.
    ///
    /// # Arguments
    ///
    /// * `keyframes` - Sequence of original [Keyframe] values, generally whose `Data` argument is
    ///   a struct generated by the [`Animate`](../../mina_macros/derive.Animate.html) macro, e.g.
    ///   `FooKeyframe` for an animator defined on type `Foo`, which contains one [`Option`] for
    ///   each animatable field.
    ///
    /// * `default_value` - Value of the timeline at the 0% (`0.0`) position, **if and only if**
    ///   the `keyframes` do not start at 0%. Otherwise, this argument is ignored.
    ///
    /// * `default_easing` - Type of easing that will be used from the start of the timeline until
    ///   a frame overrides it with its own [`Easing`]. Once a frame specifies its own easing, that
    ///   becomes the new default until another frame overrides it again, etc. If no keyframes
    ///   specify their own easing, then this easing applies to every frame.
    pub fn from_keyframes<'a, Data: 'a, ValueFn>(
        keyframes: impl IntoIterator<Item = &'a Keyframe<Data>>,
        default_value: Value,
        get_value: ValueFn,
        default_easing: Easing,
    ) -> Self
    where
        ValueFn: Fn(&Data) -> Option<Value>,
    {
        let mut converted_frames = Vec::new();
        let mut frame_index_map = Vec::new();
        let mut current_easing = default_easing;
        for keyframe in keyframes.into_iter() {
            // There must always be a frame at t = 0. If the original timeline does not specify one,
            // add one with the default value.
            if converted_frames.is_empty() && keyframe.normalized_time > 0.0 {
                converted_frames.push(SplitKeyframe::new(
                    0.0,
                    default_value.clone(),
                    current_easing.clone(),
                ));
            }
            if let Some(data) = get_value(&keyframe.data) {
                if let Some(easing) = &keyframe.easing {
                    current_easing = easing.clone();
                }
                converted_frames.push(SplitKeyframe::new(
                    keyframe.normalized_time,
                    data,
                    current_easing.clone(),
                ));
            }
            frame_index_map.push(converted_frames.len() - 1);
        }
        let trailing_frame = match converted_frames.last() {
            Some(frame) if frame.normalized_time < 1.0 =>
            // There must always be a frame at t = 1. If the original timeline does not specify
            // one, add one with the same value as the previous frame.
            {
                Some(frame.with_time(1.0))
            }
            _ => None,
        };
        if let Some(trailing_frame) = trailing_frame {
            converted_frames.push(trailing_frame);
        }
        Self {
            frames: converted_frames,
            frame_index_map,
        }
    }

    /// Gets the value for this sub-timeline's property at a given position.
    ///
    /// Does not perform a full search of keyframes based on the time; instead this expects the
    /// caller to first determine the keyframe index in the _master_ timeline (not this
    /// sub-timeline) and provide it as the `index_hint`.
    ///
    /// # Arguments
    ///
    /// * `normalized_time` - Timeline position from 0% (`0.0`) to 100% (`1.0`). Values outside this
    ///   range are clamped to the range.
    /// * `index_hint` - Index of the keyframe containing the `normalized_time` in the original
    ///   timeline that was provided to [`from_keyframes`](SubTimeline::from_keyframes) on creation.
    pub fn value_at(&self, normalized_time: f32, index_hint: usize) -> Option<Value> {
        let normalized_time = normalized_time.clamp(0.0, 1.0);
        let bounding_frames = self.get_bounding_frames(normalized_time, index_hint)?;
        Some(interpolate_value(&bounding_frames, normalized_time))
    }

    fn get_bounding_frames(
        &self,
        normalized_time: f32,
        index_hint: usize,
    ) -> Option<[&SplitKeyframe<Value>; 2]> {
        let index_at = *self.frame_index_map.get(index_hint)?;
        let frame_at = self.frames.get(index_at)?;
        if normalized_time < frame_at.normalized_time {
            if index_at > 0 {
                Some([&self.frames[index_at - 1], frame_at])
            } else {
                None
            }
        } else if index_at == self.frames.len() - 1 {
            Some([&self.frames[index_at], &self.frames[index_at]])
        } else {
            self.frames
                .get(index_at + 1)
                .map(|next_frame| [frame_at, next_frame])
        }
    }
}

/// Internal keyframe type used in a [SubTimeline].
///
/// This is referred to as a "split" keyframe because the original keyframes are _split_ into
/// sub-timelines per animation property. The differences between a [Keyframe] and [SplitKeyframe]
/// are:
///
/// * `Keyframe`s include the entire set of animatable properties as `Option`s. [SplitKeyframe]
///   holds the value of only one property, and it is non-optional.
///
/// * `Keyframe`s specify an optional [Easing] that overrides whichever previous easing was used,
///   and applies until a subsequent frame overrides it again; this means zero or some very small
///   number of keyframes may have the field populated. `SplitKeyframe` always specifies an easing
///   function, as determined by the aforementioned rules on `Keyframe`, so that the interpolation
///   for any given timeline position does not require additional searching.
#[derive(Debug)]
struct SplitKeyframe<Value> {
    easing: Easing,
    normalized_time: f32,
    value: Value,
}

impl<Value> SplitKeyframe<Value> {
    fn new(normalized_time: f32, value: Value, easing: Easing) -> Self {
        Self {
            normalized_time,
            value,
            easing,
        }
    }
}

impl<Value: Clone> SplitKeyframe<Value> {
    fn with_time(&self, normalized_time: f32) -> Self {
        SplitKeyframe::new(normalized_time, self.value.clone(), self.easing.clone())
    }
}

fn interpolate_value<Value: Clone + Lerp>(
    bounding_frames: &[&SplitKeyframe<Value>; 2],
    time: f32,
) -> Value {
    let [start_frame, end_frame] = bounding_frames;
    let duration = end_frame.normalized_time - start_frame.normalized_time;
    if duration == 0.0 {
        return start_frame.value.clone();
    }
    // For parity with CSS spec, easing (timing function) is always taken from the "start" frame.
    // Any easing defined on a keyframe at t = 1.0 is ignored.
    // https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function#description
    let easing = &start_frame.easing;
    let x = (time - start_frame.normalized_time) / duration;
    let y = easing.calc(x);
    start_frame.value.lerp(&end_frame.value, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::Timeline;

    #[derive(Debug, Default, PartialEq)]
    struct TestValues {
        foo: u8,
        bar: f32,
    }

    impl TestValues {
        pub fn new(foo: u8, bar: f32) -> Self {
            Self { foo, bar }
        }

        // A normal values wouldn't have this, but it helps with testing given the lack of floating
        // point precision.
        pub fn round(&self) -> Self {
            Self {
                foo: self.foo,
                bar: self.bar.round(),
            }
        }
    }

    struct TestKeyframeData {
        foo: Option<u8>,
        bar: Option<f32>,
    }

    impl TestKeyframeData {
        fn new(foo: Option<u8>, bar: Option<f32>) -> Self {
            Self { foo, bar }
        }

        fn full(foo: u8, bar: f32) -> Self {
            Self {
                foo: Some(foo),
                bar: Some(bar),
            }
        }
    }

    #[derive(Debug)]
    struct TestTimeline {
        boundary_times: Vec<f32>,
        foo: SubTimeline<u8>,
        bar: SubTimeline<f32>,
    }

    impl TestTimeline {
        fn new(keyframes: Vec<Keyframe<TestKeyframeData>>, default_easing: Easing) -> Self {
            let defaults = TestValues::default();
            Self {
                foo: SubTimeline::from_keyframes(
                    &keyframes,
                    defaults.foo,
                    |k| k.foo,
                    default_easing.clone(),
                ),
                bar: SubTimeline::from_keyframes(
                    &keyframes,
                    defaults.bar,
                    |k| k.bar,
                    default_easing.clone(),
                ),
                boundary_times: keyframes.iter().map(|k| k.normalized_time).collect(),
            }
        }
    }

    impl Timeline for TestTimeline {
        type Values = TestValues;

        // A real timeline would have its own time scale with delay, duration, etc., which is
        // different from the normalized time scale of the `SubTimeline`. These differences aren't
        // important for the purpose of unit-testing the sub.
        fn values_at(&self, time: f32) -> Self::Values {
            let mut values = Self::Values::default();
            if self.boundary_times.is_empty() {
                return values;
            }
            let frame_index = match self.boundary_times.binary_search_by(|t| t.total_cmp(&time)) {
                Ok(index) => index,
                Err(next_index) => next_index.max(1) - 1,
            };
            if let Some(foo) = self.foo.value_at(time, frame_index) {
                values.foo = foo;
            }
            if let Some(bar) = self.bar.value_at(time, frame_index) {
                values.bar = bar;
            }
            values
        }
    }

    #[test]
    fn when_empty_then_always_provides_defaults() {
        let timeline = TestTimeline::new(vec![], Easing::default());

        assert_eq!(timeline.values_at(0.0), TestValues::default());
        assert_eq!(timeline.values_at(0.5), TestValues::default());
        assert_eq!(timeline.values_at(1.0), TestValues::default());
    }

    #[test]
    fn when_no_keyframe_at_zero_then_interpolates_from_defaults() {
        let keyframes = vec![
            Keyframe::new(0.25, TestKeyframeData::new(None, Some(50.0)), None),
            Keyframe::new(0.5, TestKeyframeData::new(Some(80), Some(200.0)), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::default());

        assert_eq!(timeline.values_at(0.0), TestValues::default());
        assert_eq!(timeline.values_at(0.1), TestValues::new(16, 20.0));
        assert_eq!(timeline.values_at(0.25), TestValues::new(40, 50.0));
    }

    #[test]
    fn when_keyframe_at_zero_then_interpolates_from_first_keyframe() {
        let keyframes = vec![
            Keyframe::new(0.0, TestKeyframeData::full(10, 20.0), None),
            Keyframe::new(0.4, TestKeyframeData::full(50, 200.0), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::default());

        assert_eq!(timeline.values_at(0.0), TestValues::new(10, 20.0));
        assert_eq!(timeline.values_at(0.2), TestValues::new(30, 110.0));
        assert_eq!(timeline.values_at(0.4), TestValues::new(50, 200.0));
    }

    #[test]
    fn when_no_keyframe_at_end_then_stays_at_last_keyframe() {
        let keyframes = vec![
            Keyframe::new(0.5, TestKeyframeData::new(Some(30), None), None),
            Keyframe::new(0.75, TestKeyframeData::new(Some(50), Some(1000.0)), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::default());

        assert_eq!(timeline.values_at(0.75), TestValues::new(50, 1000.0));
        assert_eq!(timeline.values_at(0.85), TestValues::new(50, 1000.0));
        assert_eq!(timeline.values_at(1.0), TestValues::new(50, 1000.0));
    }

    #[test]
    fn when_keyframe_at_end_then_interpolates_to_last_keyframe() {
        let keyframes = vec![
            Keyframe::new(0.25, TestKeyframeData::full(40, 250.0), None),
            Keyframe::new(0.5, TestKeyframeData::full(20, 0.0), None),
            Keyframe::new(1.0, TestKeyframeData::full(60, 1000.0), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::default());

        assert_eq!(timeline.values_at(0.5), TestValues::new(20, 0.0));
        assert_eq!(timeline.values_at(0.75), TestValues::new(40, 500.0));
        assert_eq!(timeline.values_at(1.0), TestValues::new(60, 1000.0));
    }

    #[test]
    fn when_easing_not_overridden_then_interpolates_with_default_easing() {
        let keyframes = vec![
            Keyframe::new(0.0, TestKeyframeData::full(0, 0.0), None),
            Keyframe::new(1.0, TestKeyframeData::full(40, 100.0), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::OutQuad);

        assert_eq!(timeline.values_at(0.0).round(), TestValues::new(0, 0.0));
        assert_eq!(timeline.values_at(0.2).round(), TestValues::new(19, 49.0));
        assert_eq!(timeline.values_at(0.4).round(), TestValues::new(31, 78.0));
        assert_eq!(timeline.values_at(0.6).round(), TestValues::new(37, 94.0));
        assert_eq!(timeline.values_at(0.8).round(), TestValues::new(39, 99.0));
        assert_eq!(timeline.values_at(1.0).round(), TestValues::new(40, 100.0));
    }

    #[test]
    fn when_easing_overridden_then_uses_new_easing() {
        let keyframes = vec![
            Keyframe::new(0.0, TestKeyframeData::full(0, 0.0), None),
            Keyframe::new(0.2, TestKeyframeData::full(50, 100.0), None),
            Keyframe::new(
                0.4,
                TestKeyframeData::full(100, 400.0),
                Some(Easing::OutCirc),
            ),
            Keyframe::new(0.6, TestKeyframeData::full(150, 1000.0), None),
            Keyframe::new(
                0.8,
                TestKeyframeData::full(200, 5000.0),
                Some(Easing::InSine),
            ),
            Keyframe::new(1.0, TestKeyframeData::full(250, 10000.0), None),
        ];
        let timeline = TestTimeline::new(keyframes, Easing::default());

        assert_eq!(timeline.values_at(0.0).round(), TestValues::new(0, 0.0));
        assert_eq!(timeline.values_at(0.1).round(), TestValues::new(25, 50.0));
        assert_eq!(timeline.values_at(0.2).round(), TestValues::new(50, 100.0));
        assert_eq!(timeline.values_at(0.3).round(), TestValues::new(75, 250.0));
        assert_eq!(timeline.values_at(0.4).round(), TestValues::new(100, 400.0));
        assert_eq!(timeline.values_at(0.5).round(), TestValues::new(135, 824.0));
        assert_eq!(
            timeline.values_at(0.6).round(),
            TestValues::new(150, 1000.0)
        );
        assert_eq!(
            timeline.values_at(0.7).round(),
            TestValues::new(185, 3825.0)
        );
        assert_eq!(
            timeline.values_at(0.8).round(),
            TestValues::new(200, 5000.0)
        );
        assert_eq!(
            timeline.values_at(0.9).round(),
            TestValues::new(206, 5625.0)
        );
        assert_eq!(
            timeline.values_at(1.0).round(),
            TestValues::new(250, 10000.0)
        );
    }
}
