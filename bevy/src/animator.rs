//! Contains the primary [Animator] component and supporting types and systems.

use crate::traits::SafeTimeline;
use bevy::prelude::*;
use std::time::Duration;

/// The state of an [Animator].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Reflect)]
pub enum AnimationState {
    /// No animation. Either the animator does not have a [Timeline](mina::Timeline), or the
    /// timeline was just added on this frame and the animation system has not run yet.
    #[default]
    None,
    /// Animation has not started yet because of a configured [delay](mina::Timeline::delay), and
    /// will start after the delay elapses.
    Waiting,
    /// Animation is in progress.
    Playing,
    /// Animation progressed to the end of the [Timeline](mina::Timeline) and there are no more
    /// frames to animate. This state is only possible for timelines whose
    /// [repeat](mina::Timeline::repeat) is not [Infinite](mina::Repeat::Infinite).
    Ended,
}

/// An event that is sent whenever an animator's [Animator::state] changes.
#[derive(Event, Reflect)]
pub struct AnimationStateChanged {
    /// The entity to which the affected [Animator] is attached.
    pub entity: Entity,
    /// State of the affected [Animator] when the event was created.
    pub state: AnimationState,
}

impl AnimationStateChanged {
    /// Creates a new [AnimationStateChanged] event.
    pub fn new(entity: Entity, state: AnimationState) -> Self {
        Self { entity, state }
    }
}

/// Controls animation of the properties of another [Component] attached to the same entity.
///
/// In most cases, the component type `T` should also be decorated with
/// [Animate](mina::prelude::Animate), which will generate the corresponding
/// [Timeline](mina::Timeline) type that can be assigned in [Self::set_timeline].
#[derive(Component, Reflect)]
pub struct Animator<T: Component> {
    /// Whether or not the animator is currently enabled. If disabled, animations will not progress.
    /// Setting this property does not change the [Self::state].
    pub enabled: bool,
    /// The current position of the associated [Timeline](mina::Timeline), i.e. the duration of time
    /// that the current timeline has been active on this component.
    ///
    /// Be careful when setting this property directly, as it will not change the animator's
    /// [Self::state]. If the state is already [AnimationState::Ended], then setting the position
    /// before the end time will not restart it. To manually restart animation using the existing
    /// timeline, call [Self::reset].
    pub timeline_position: Duration,
    #[reflect(ignore)]
    pub(super) timeline: Option<Box<dyn SafeTimeline<Target = T>>>,
    pub(super) state: AnimationState,
}

impl<T: Component> Default for Animator<T> {
    fn default() -> Self {
        Self {
            enabled: true,
            timeline_position: Duration::ZERO,
            timeline: None,
            state: AnimationState::default(),
        }
    }
}

impl<T: Component> Animator<T> {
    /// Creates a new [Animator] without a [Timeline].
    pub fn new() -> Self {
        Self {
            enabled: true,
            timeline: None,
            timeline_position: Duration::ZERO,
            state: AnimationState::None,
        }
    }

    /// Creates a new [Animator] initialized with the specified [Timeline].
    pub fn with_timeline(timeline: impl SafeTimeline<Target = T>) -> Self {
        Self {
            enabled: true,
            timeline: Some(Box::new(timeline)),
            timeline_position: Duration::ZERO,
            state: AnimationState::None,
        }
    }

    /// Disables and returns the animator. Used when creating new instances to prevent the
    /// animations from running immediately.
    pub fn as_disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Resets this animator so that it starts its configured animation from the first keyframe.
    ///
    /// If the configured [Timeline](mina::Timeline) has a [delay](mina::Timeline::delay), then this
    /// will **reintroduce** the delay and real animation will not start until the delay elapses.
    ///
    /// Resetting can be combined with [Self::timeline_position] for fine-grained control of
    /// animation frames.
    pub fn reset(&mut self) {
        self.timeline_position = Duration::ZERO;
        self.state = AnimationState::None;
    }

    /// Configures the [Timeline](mina::Timeline) that this animator will use.
    ///
    /// If no timeline was previously configured, then animation will start on this frame or the
    /// next frame (depending on when the caller runs). If a previous timeline was in use, then
    /// changing the timeline will **not** reset the animation state. This is intentional, in order
    /// to allow "hot-swapping" of timelines with equal or similar durations and continuing from the
    /// current [Self::timeline_position].
    ///
    /// To ensure that the new animation starts from the beginning, explicitly [Self::reset] after
    /// changing the timeline.
    pub fn set_timeline(&mut self, timeline: impl SafeTimeline<Target = T>) {
        self.timeline = Some(Box::new(timeline));
    }

    /// Gets the current animation state.
    pub fn state(&self) -> AnimationState {
        self.state
    }
}

pub(super) fn animate<T: Component>(
    time: Res<Time>,
    mut animators: Query<(Entity, &mut Animator<T>)>,
    mut targets: Query<&mut T>,
    mut events: EventWriter<AnimationStateChanged>,
) {
    for (entity, mut animator) in animators.iter_mut() {
        if !animator.enabled {
            continue;
        }
        if animator.timeline.is_none() {
            if animator.state != AnimationState::None {
                animator.state = AnimationState::None;
                events.send(AnimationStateChanged::new(entity, AnimationState::None));
            }
            continue;
        }
        let position_secs = animator.timeline_position.as_secs_f32();
        let timeline = animator.timeline.as_ref().unwrap();
        // Early assignments are needed to make Rust's borrow checker happy; it won't let us read
        // from the `timeline` struct anymore after the `update`.
        let timeline_delay = timeline.delay();
        let timeline_duration = timeline.duration();
        if animator.state == AnimationState::Playing {
            if let Ok(mut target) = targets.get_mut(entity) {
                timeline.update(&mut target, position_secs);
            }
        }
        let mut state_changed = false;
        if animator.state == AnimationState::None {
            animator.state = AnimationState::Waiting;
            state_changed = true;
        }
        if animator.state == AnimationState::Waiting && position_secs >= timeline_delay {
            animator.state = AnimationState::Playing;
            state_changed = true;
        }
        if position_secs >= timeline_duration && animator.state != AnimationState::Ended {
            animator.state = AnimationState::Ended;
            state_changed = true;
        }
        if animator.state != AnimationState::Ended {
            animator.timeline_position += time.delta();
        }
        if state_changed {
            events.send(AnimationStateChanged::new(entity, animator.state));
        }
    }
}
