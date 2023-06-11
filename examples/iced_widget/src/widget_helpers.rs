use enum_map::EnumArray;
use mina::prelude::*;
use std::time::Instant;

/// Animator state based on a typical pattern of mouse interaction.
///
/// For simplicity, does not track a "down but not over" state, as it would complicate the timeline
/// setup. It is possible to do by adding another state, or by adding a `bool` to the `Down` state.
#[derive(Clone, Debug, Default, Eq, PartialEq, State)]
pub enum Interaction {
    #[default]
    None,
    Over,
    Down,
}

/// Wrapper for an animator encapsulated in widget state.
///
/// Widgets in Iced have to follow a particular pattern for running animations, in which they track
/// the last instant the animator updated and use the next instant to compute the delta. Mina's
/// animations want just the delta. This makes it more convenient to bridge the two, and provides
/// some pass-through helper functions so that callers don't need to pull the internal animator.
pub struct WidgetAnimator<Timeline, WidgetState = Interaction>
where
    Timeline: mina::Timeline,
    Timeline::Target: Clone,
    WidgetState: Clone + Default + EnumArray<Option<MergedTimeline<Timeline>>> + PartialEq,
{
    animator: EnumStateAnimator<WidgetState, Timeline>,
    last_tick: Instant,
}

impl<Timeline, WidgetState> WidgetAnimator<Timeline, WidgetState>
where
    Timeline: mina::Timeline,
    Timeline::Target: Clone,
    WidgetState: Clone + Default + EnumArray<Option<MergedTimeline<Timeline>>> + PartialEq,
{
    pub fn new(effects: EnumStateAnimator<WidgetState, Timeline>) -> Self {
        Self {
            animator: effects,
            last_tick: Instant::now(),
        }
    }

    pub fn current_interaction(&self) -> &WidgetState {
        self.animator.current_state()
    }

    pub fn current_values(&self) -> &Timeline::Target {
        self.animator.current_values()
    }

    pub fn set_interaction(&mut self, state: &WidgetState) {
        self.animator.set_state(state);
    }

    pub fn sync(&mut self, now: Instant) {
        let elapsed_seconds = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        self.animator.advance(elapsed_seconds);
    }
}
