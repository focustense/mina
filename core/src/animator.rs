//! Stateful animations that change according to external conditions such as user interaction.

use crate::timeline::{MergedTimeline, Timeline, TimelineOrBuilder};
pub use enum_map::Enum as State;
use enum_map::{EnumArray, EnumMap};
use std::marker::PhantomData;
use std::time::Duration;

/// Provides read-only methods that are similar to those of a [`HashMap`](std::collections::HashMap)
/// but can be implemented by other concrete types.
///
/// The main purpose of this is to support the use of [`EnumMap`] in animators.
pub trait MapLike<K, V> {
    /// Gets a reference to the value with specified `key`, or [`None`] if no such key is present in
    /// the map.
    fn get(&self, key: &K) -> Option<&V>;

    /// Gets a mutable reference to the value with specified `key`, or [`None`] if no such key is
    /// present in the map.
    fn get_mut(&mut self, key: &K) -> Option<&mut V>;
}

impl<K: Clone + EnumArray<Option<V>>, V> MapLike<K, V> for EnumMap<K, Option<V>> {
    fn get(&self, key: &K) -> Option<&V> {
        self[key.clone()].as_ref()
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self[key.clone()].as_mut()
    }
}

/// Animates a collection of values over time, automatically selecting the correct animation based
/// on current state and blending with any previous animation still in progress.
///
/// To create a `StateAnimator`, use the [`StateAnimatorBuilder`] helper.
///
/// # Animator vs. Timeline
///
/// [`Timeline`]s are stateless types that represent a single animation and are able to query the
/// animated values at any given point in time. They are generally only useful for non-interactive
/// scenarios, coordinating many related animations, etc. Typically, a timeline might be defined as
/// a shared resource and used many times throughout an app.
///
/// `StateAnimator`s encapsulate an entire animated "component", including both the animation
/// definitions (timelines) and the current animator values, e.g. the "style". If there are many
/// animatable widgets in a UI, each one of those widgets will have its own animator. Animators are
/// more opinionated than timelines, but also model _transitions between timelines_ and the states
/// that trigger them.
pub trait StateAnimator {
    /// State enum that determines which animation to play. Most often this describes the state of
    /// user interaction and includes values such as `MouseOver`, `MouseDown`, etc.
    type State;

    /// Target values animated by this type, i.e. the "Style" of the component being animated.
    /// Typical fields might include a transform (`x`, `y`, `scale`, etc.), colors or `alpha`
    /// values, or anything else that describes the visual appearance.
    ///
    /// Any type supported by a [`Timeline`] may be used, but in practice this should almost always
    /// be a struct decorated with the [`Animate`](../../mina_macros/derive.Animate.html) macro.
    type Values;

    /// Advances whichever animation is currently playing by `elapsed_seconds` (time since the most
    /// recent update).
    ///
    /// If the current animation has ended and does not repeat, this has no effect.
    fn advance(&mut self, elapsed_seconds: f32);

    /// Gets a reference to the current values, corresponding to the current `State` and current
    /// position on the timeline for that state.
    fn current_values(&self) -> &Self::Values;

    /// Transitions to a new state.
    ///
    /// This has no immediate effect on the [`current_values`](Self::current_values), but will start
    /// to take effect on the next [`advance`](Self::advance). As time elapses, the values will
    /// gradually animate from where they were before `set_state` to where they should be at the
    /// first keyframe (excluding any keyframe at 0%) of the [`Timeline`] configured for the new
    /// `state`. Afterward, they will follow the normal timeline for that state.
    ///
    /// If the current state is already equal to `state`, this is ignored. If the state has changed,
    /// but the new `state` does not have any associated timeline, then the previous animation will
    /// be stopped but the values will not be changed.
    fn set_state(&mut self, state: &Self::State);
}

/// Default implementation of a [`StateAnimator`] using an [`EnumMap`].
///
/// Cannot be created directly; to create an instance, use the [`StateAnimatorBuilder`].
pub struct MappedTimelineAnimator<State, Timeline, TimelineMap>
where
    State: Clone + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone,
    TimelineMap: MapLike<State, MergedTimeline<Timeline>>,
{
    timelines: TimelineMap,
    current_state: State,
    current_values: Timeline::Target,
    state_duration: Duration,
    _timeline_phantom: PhantomData<Timeline>,
}

impl<State, Timeline, TimelineMap> MappedTimelineAnimator<State, Timeline, TimelineMap>
where
    State: Clone + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone,
    TimelineMap: MapLike<State, MergedTimeline<Timeline>>,
{
    fn new(timelines: TimelineMap, initial_state: State, initial_values: Timeline::Target) -> Self {
        let mut animator = MappedTimelineAnimator {
            timelines,
            current_state: initial_state.clone(),
            current_values: initial_values,
            state_duration: Duration::ZERO,
            _timeline_phantom: PhantomData,
        };
        animator.blend_next_timeline(&initial_state);
        animator
    }

    fn blend_next_timeline(&mut self, state: &State) {
        if let Some(next_timeline) = self.timelines.get_mut(state) {
            next_timeline.start_with(&self.current_values);
        }
    }

    fn update_current_values(&mut self) {
        if let Some(timeline) = self.timelines.get(&self.current_state) {
            timeline.update(&mut self.current_values, self.state_duration.as_secs_f32());
        }
    }
}

impl<State, Timeline, TimelineMap> StateAnimator
    for MappedTimelineAnimator<State, Timeline, TimelineMap>
where
    State: Clone + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone,
    TimelineMap: MapLike<State, MergedTimeline<Timeline>>,
{
    type State = State;
    type Values = Timeline::Target;

    fn advance(&mut self, elapsed_seconds: f32) {
        self.state_duration += Duration::from_secs_f32(elapsed_seconds);
        self.update_current_values();
    }

    fn current_values(&self) -> &Self::Values {
        &self.current_values
    }

    fn set_state(&mut self, state: &State) {
        if state == &self.current_state {
            return;
        }
        self.blend_next_timeline(state);
        self.current_state = state.clone();
        self.state_duration = Duration::ZERO;
        self.update_current_values();
    }
}

// Examples not provided due to https://github.com/rust-lang/rust/issues/82544.
//
// There doesn't seem to be a way to use the Animate macro, which depends on the core library, in
// the tests for the core library, without getting into dependency hell. The `state_animator_test`
// integration test provides some usage examples.

/// Builder for a [`StateAnimator`].
///
/// Provides a fluent interface for configuring the [`Timeline`] associated with each state.
pub struct StateAnimatorBuilder<State, Timeline>
where
    State: Clone + Default + EnumArray<Option<MergedTimeline<Timeline>>> + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone + Default,
{
    initial_state: State,
    initial_values: Timeline::Target,
    timelines: EnumMap<State, Option<MergedTimeline<Timeline>>>,
}

// There appears to be something wrong with `#[derive(Default)]`, or possibly a strange quirk caused
// by EnumMap. Using default-derive, rustc will complain about `Default` not being implemented for
// the Timeline type, which isn't actually required (as opposed to the Timeline::Target type, which
// is required). It will complain about this only when we actually try to _call_ the `default()`
// method. So we must use a manual implementation.
impl<State, Timeline> Default for StateAnimatorBuilder<State, Timeline>
where
    State: Clone + Default + EnumArray<Option<MergedTimeline<Timeline>>> + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone + Default,
{
    fn default() -> Self {
        Self {
            initial_state: Default::default(),
            initial_values: Timeline::Target::default(),
            timelines: EnumMap::default(),
        }
    }
}

impl<State, Timeline> StateAnimatorBuilder<State, Timeline>
where
    State: Clone + Default + EnumArray<Option<MergedTimeline<Timeline>>> + PartialEq,
    Timeline: crate::timeline::Timeline,
    Timeline::Target: Clone + Default,
{
    /// Creates a new [`StateAnimatorBuilder`] with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the [`StateAnimator`], consuming self.
    pub fn build(
        self,
    ) -> MappedTimelineAnimator<State, Timeline, EnumMap<State, Option<MergedTimeline<Timeline>>>>
    {
        MappedTimelineAnimator::new(self.timelines, self.initial_state, self.initial_values)
    }

    /// Specifies the default `State` in which the animator starts, typically a "None" or "Idle"
    /// state depending on the actual type of `State`.
    pub fn from_state(mut self, state: State) -> Self {
        self.initial_state = state;
        self
    }

    /// Specifies the default `Values` that the resulting animator will provide in its
    /// [`current_values`](StateAnimator::current_values) before any time advances. Not required
    /// unless the values should be different from the [`Default`] for the target type.
    ///
    /// Commonly, these are the same values that the timeline for the default `State` (or whichever
    /// state is specified in [`from_state`](Self::from_state) has at its 100% keyframe; using these
    /// values means that no visible animation will occur until the state changes. Specifying any
    /// other values implies that the animator should be immediately playing an animation.
    pub fn from_values(mut self, values: Timeline::Target) -> Self {
        self.initial_values = values;
        self
    }

    /// Configures the [`Timeline`] for a given `State` value.
    ///
    /// The `timeline` can be the actual timeline for the given `Values` type that was generated by
    /// [`Animate`](../../mina_macros/derive.Animate.html), or a builder for that type, or a
    /// [`MergedTimeline`].
    ///
    /// Multiple calls to [`on`](Self::on) with the same `state` argument will result in the most
    /// recent timelines being used, and previous timelines being dropped. To apply multiple
    /// independent animations to a single state, use a [`MergedTimeline`].
    pub fn on(mut self, state: State, timeline: impl TimelineOrBuilder<Timeline>) -> Self {
        self.timelines[state] = Some(timeline.build());
        self
    }
}
