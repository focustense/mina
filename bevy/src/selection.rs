//! Contains components for state-based animations with behavior similar to
//! [StateAnimator](mina::StateAnimator).

use crate::traits::*;
use crate::{AnimationState, AnimationStateChanged, Animator};
use bevy::prelude::*;
use bevy::utils::HashMap;
use dyn_clone::clone_box;

/// [Component] for automatically selecting the [Timeline](mina::Timeline) of an [Animator] based on
/// some arbitrary state.
///
/// This is the ECS version of a [StateAnimator](mina::StateAnimator). To avoid ambiguity with
/// Bevy's own [State](bevy::ecs::schedule::State), the "state" is referred to as simply a "key".
///
/// Like the `StateAnimator`, this blends animations. When the current key is changed, and a new
/// timeline is chosen, the timeline will animate from the current properties instead of the values
/// configured for the first keyframe.
#[derive(Component, Reflect)]
pub struct AnimationSelector<K: AnimationKey, T: Component> {
    /// Map of state keys to the corresponding animations (timelines).
    #[reflect(ignore)]
    pub timelines: HashMap<K, Box<dyn SafeTimeline<Target = T>>>,
    /// Key controlling the current animation to play. The key must be present in [Self::timelines],
    /// otherwise no animation will play.
    pub timeline_key: K,
    previous_key: Option<K>,
}

impl<K: AnimationKey, T: Component> AnimationSelector<K, T> {
    /// Creates a new [AnimationSelector].
    ///
    /// For better readability, prefer to use the [AnimationSelectorBuilder] when possible.
    pub fn new(timelines: HashMap<K, Box<dyn SafeTimeline<Target = T>>>, initial_key: K) -> Self {
        Self {
            timelines,
            timeline_key: initial_key,
            previous_key: None,
        }
    }
}

/// Builder for an [AnimationSelector].
#[derive(Default)]
pub struct AnimationSelectorBuilder<K: AnimationKey, T: Component> {
    initial_key: K,
    timelines: HashMap<K, Box<dyn SafeTimeline<Target = T>>>,
}

impl<K: AnimationKey, T: Component> AnimationSelectorBuilder<K, T> {
    /// Creates a new [AnimationSelectorBuilder] with no timelines and using the default value of
    /// `K` as the starting key.
    pub fn new() -> Self {
        Self {
            initial_key: K::default(),
            timelines: HashMap::new(),
        }
    }

    /// Registers a new key-timeline pair. When [AnimationSelector::timeline_key] is set to the
    /// specified `key`, the corresponding [Animator] will switch to the specified `timeline`.
    pub fn add(mut self, key: K, timeline: impl SafeTimeline<Target = T>) -> Self {
        self.timelines.insert(key, Box::new(timeline));
        self
    }

    /// Configures the starting value of the [AnimationSelector::timeline_key], if it should be any
    /// value other than the default for `K`.
    pub fn initial_key(mut self, key: K) -> Self {
        self.initial_key = key;
        self
    }

    /// Builds the [AnimationSelector].
    pub fn build(self) -> AnimationSelector<K, T> {
        AnimationSelector::new(self.timelines, self.initial_key)
    }
}

/// Add-on [Component] for an [Animator] + [AnimationSelector] pair that supports automatic
/// transitioning between different states (keys).
///
/// The typical problem this solves is to enable state changes that are partially or fully dependent
/// on the duration of the animation. For example, if each weapon has its own "attack" animation of
/// varying length, this can be used to automatically return to an "idle" or "moving" state once
/// that animation completes, without having to coordinate timing or store additional metadata.
///
/// Another example would be showing a temporary effect, such as highlighting a health bar at low
/// health and then returning to the normal state. Aside from the most typical "animate and reset"
/// scenarios, multiple discrete animations can also be chained together, such as attacks in a
/// combo.
#[derive(Component, Default, Reflect)]
pub struct AnimationChain<K: AnimationKey> {
    /// Map of ended keys (states) to next keys.
    ///
    /// When an [Animator] finishes its animation (transitions to [AnimationState::Ended]), if the
    /// [AnimationSelector::timeline_key] matches a key in this map, then it will automatically be
    /// assigned the value for that key, and the animator will be reset.
    pub next_keys: HashMap<K, K>,
}

impl<K: AnimationKey> AnimationChain<K> {
    /// Creates a new [AnimationChain] with an empty map.
    pub fn new() -> Self {
        Self {
            next_keys: HashMap::new(),
        }
    }

    /// Creates a new [AnimationChain] with a single entry that reverts to the default key after
    /// some other key completes its animation.
    ///
    /// For very simple [AnimationKey] types, especially those with a simple "on/off" state like a
    /// `bool` or 2-state `enum`, this can be more convenient than using [AnimationChainBuilder] and
    /// also slightly more memory-efficient since it allocates a map with initial capacity of `1`.
    pub fn reset_after(ended_key: K) -> Self {
        let next_keys = HashMap::from([(ended_key, K::default())]);
        Self { next_keys }
    }
}

/// Builder for an [AnimationChain].
#[derive(Default)]
pub struct AnimationChainBuilder<K: AnimationKey> {
    next_keys: HashMap<K, K>,
}

impl<K: AnimationKey> AnimationChainBuilder<K> {
    /// Creates a new [AnimationChainBuilder].
    pub fn new() -> Self {
        Self {
            next_keys: HashMap::new(),
        }
    }

    /// Adds a new transition, specifying that when the animation in `ended_key` ends ([Animator]
    /// transitions to [AnimationState::Ended]), the [AnimationSelector] should automatically switch
    /// its [AnimationSelector::timeline_key] to `next_key`.
    pub fn add(mut self, ended_key: K, next_key: K) -> Self {
        self.next_keys.insert(ended_key, next_key);
        self
    }

    /// Builds the [AnimationChain].
    pub fn build(self) -> AnimationChain<K> {
        AnimationChain {
            next_keys: self.next_keys,
        }
    }
}

pub(super) fn chain_animations<K: AnimationKey, T: Component>(
    mut events: EventReader<AnimationStateChanged>,
    mut selector_query: Query<(&mut AnimationSelector<K, T>, &AnimationChain<K>)>,
) {
    for ev in events.iter() {
        let AnimationStateChanged { entity, state } = ev;
        if state != &AnimationState::Ended {
            continue;
        }
        let Ok((mut selector, chain)) = selector_query.get_mut(*entity) else {
            continue;
        };
        if let Some(next_key) = chain.next_keys.get(&selector.timeline_key) {
            selector.timeline_key = next_key.clone();
        }
    }
}

pub(super) fn select_animation<K: AnimationKey, T: Component>(
    mut selector_query: Query<
        (Entity, &T, &mut AnimationSelector<K, T>),
        Changed<AnimationSelector<K, T>>,
    >,
    mut animator_query: Query<&mut Animator<T>>,
) {
    for (entity, current_values, mut selector) in selector_query.iter_mut() {
        if selector
            .previous_key
            .as_ref()
            .is_some_and(|k| k == &selector.timeline_key)
        {
            continue;
        }
        selector.previous_key = Some(selector.timeline_key.clone());
        if let Ok(mut animator) = animator_query.get_mut(entity) {
            animator.timeline = selector.timelines.get(&selector.timeline_key).map(|t| {
                let mut next_timeline = *clone_box(t);
                next_timeline.start_with(current_values);
                next_timeline
            });
            animator.reset();
        }
    }
}
