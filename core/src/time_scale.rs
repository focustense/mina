//! Internal helper module for relations between real time units and normalized timelines.

use crate::timeline::Repeat;

/// Describes the time scale of a [Timeline](crate::timeline::Timeline).
///
/// Time scales handle the conversion between elapsed (since animation started) times and the
/// normalized timestamps used in [SubTimeline](crate::timeline_helpers::SubTimeline) instances.
///
/// This is an internal helper class that is used by generated code and not intended to be created
/// or consumed directly.
#[derive(Clone, Debug)]
pub struct TimeScale {
    delay: f32,
    duration: f32,
    repeat: Repeat,
    reverse: bool,
}

impl Default for TimeScale {
    fn default() -> Self {
        Self {
            delay: 0.0,
            duration: 1.0,
            repeat: Repeat::None,
            reverse: false,
        }
    }
}

impl TimeScale {
    /// Creates a new [TimeScale].
    ///
    /// # Arguments
    ///
    /// * `duration` - Duration of an animation cycle, including the reversal time if `reverse` is
    ///   `true`, but *not* including the `delay`. If `repeat` is [`Repeat::None`], then this is the
    ///   total animation duration.
    /// * `delay` - Time to wait, in the same units as `duration`, before starting the animation.
    ///   This is a flat delay and only applies once to the entire timeline - i.e. it is _not_
    ///   repeated on every cycle.
    /// * `repeat` - Whether and how many times the animation should repeat.
    /// * `reverse` - Whether the animation loops instantly from the 100% position back to the 0%
    ///   position, assuming it repeats, or animates backward to 0% during the second half of each
    ///   cycle using the same easing function as the forward half.
    pub fn new(duration: f32, delay: f32, repeat: Repeat, reverse: bool) -> Self {
        Self {
            duration,
            delay,
            repeat,
            reverse,
        }
    }

    /// Gets the duration of a single cycle, irrespective of [Repeat] setting.
    pub fn get_cycle_duration(&self) -> f32 {
        self.duration
    }

    /// Gets the delay before animation starts.
    pub fn get_delay(&self) -> f32 {
        self.delay
    }

    /// Gets the duration of the entire animation.
    ///
    /// # Returns
    ///
    /// The sum of the initial delay and all cycle repetitions. If the animation repeats infinitely,
    /// returns `[f32::INFINITY]`.
    pub fn get_duration(&self) -> f32 {
        if self.repeat == Repeat::Infinite {
            f32::INFINITY
        } else {
            self.delay + self.duration * (self.repeat.as_ordinal() + 1) as f32
        }
    }

    /// Gets the repetitions configured for this timescale.
    pub fn get_repeat(&self) -> Repeat {
        self.repeat
    }

    /// Computes the timescale-relative position (e.g. normalized time) for some real time.
    ///
    /// # Arguments
    ///
    /// * `time` - Elapsed time in the same units as the timescale's duration.
    ///
    /// # Returns
    ///
    /// If the timeline is active at the specified `time`, then a [`TimeScalePosition::Active`]
    /// value holding the normalized time between `0.0` and `1.0`. Normalized time is relative to
    /// keyframe times, which are also between `0.0` (0%) and `1.0` (100%).
    ///
    /// * For example, if the animator is configured to reverse, then the last keyframe is reached
    /// (result = `1.0`) when `time` is at 50% of the configured duration, and declines back to
    /// `0.0` until 100% of the duration is reached.
    /// * If not reversing, then the normalized time increases monotonically from `0.0` to `1.0`
    /// until either the animation fully ends (remains at `1.0`) or the next loop begins (resets to
    /// `0.0`).
    ///
    /// If the `time` is nowhere on the timeline, returns one of the other [`TimeScalePosition`]
    /// values indicating which extreme was reached.
    pub fn get_position(&self, time: f32) -> TimeScalePosition {
        let time = time - self.delay;
        if time < 0.0 {
            return TimeScalePosition::NotStarted;
        }
        let (cycle_time, is_repeating) = match self.repeat {
            Repeat::None if time > self.duration => return self.position_ended(),
            Repeat::None => (time, false),
            Repeat::Times(times) if time > self.duration * (times + 1) as f32 => {
                return self.position_ended();
            }
            Repeat::Times(_) | Repeat::Infinite => {
                // Doing the "simple" modulo arithmetic can produce some unintuitive results, since
                // the normalized remainder can never be equal to 1.0 at the end of a cycle, it will
                // always reset to 0.0. In a looping animation, this means we literally never hit
                // the terminal value, which could be very noticeable for a reversing animation and
                // especially one with a steep "ease-in" function.
                //
                // Instead, we hold the value at `duration` (normalized 1.0) as long as at least one
                // full cycle has completed; this results in interpolating up to 1.0, then resetting
                // or reversing back down to some very small but non-zero value.
                //
                // This might just have the opposite problem - never reaching the exact zero value,
                // which could be noticeable with a steep ease-OUT function - but since animations
                // are usually going to be blended with a state-dependent start value anyway, it
                // makes somewhat more sense to focus on getting the end value correct.
                let (quot, rem) = (time / self.duration, time % self.duration);
                if rem == 0.0 && quot >= 1.0 {
                    (self.duration, quot > 1.0)
                } else {
                    (rem, quot >= 1.0)
                }
            }
        };
        let cycle_ratio = cycle_time / self.duration;
        let (normalized_time, is_reversing) = match self.reverse {
            true if cycle_ratio > 0.5 => ((1.0 - cycle_ratio) * 2.0, true),
            true => (cycle_ratio * 2.0, false),
            false => (cycle_ratio, false),
        };
        TimeScalePosition::Active(
            normalized_time,
            TimeScaleLoopState::new(is_repeating, is_reversing),
        )
    }

    fn position_ended(&self) -> TimeScalePosition {
        let normalized_time = if self.reverse { 0.0 } else { 1.0 };
        TimeScalePosition::Ended(normalized_time)
    }
}

/// Result of a [`TimeScale::get_position`] query, describing either the normalized position of a
/// time on the timeline or a boundary that is exceeded.
#[derive(Debug, PartialEq)]
pub enum TimeScalePosition {
    /// The timeline has not started at the specified time, either because the time was negative or
    /// because it is within the configured delay period. When determining animator values, this can
    /// be considered equivalent to a normalized time of `0.0`.
    NotStarted,
    /// The timeline is in progress at the specified time, corresponding to the normalized position
    /// (from `0.0` to `1.0`), and with the given loop info.
    Active(f32, TimeScaleLoopState),
    /// The timeline has already ended at the specified time, i.e. it does not loop infinitely and
    /// the specified time is after the last loop ends. Holds a value indicating the normalized time
    /// reached at the end, which is either `0.0` if the timeline reverses or `1.0` if it does not.
    Ended(f32),
}

/// Provides additional information about the relationship between a real time and a normalized
/// position, taking into account repeat and/or reverse behavior.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct TimeScaleLoopState {
    /// Whether or not the position tagged with this state is considered a repetition, i.e. the
    /// timeline has completed at least one entire cycle before reaching it.
    pub is_repeating: bool,

    /// Whether or not the position tagged with this state is on the reverse pass of any cycle,
    /// including the first cycle.
    pub is_reversing: bool,
}

impl TimeScaleLoopState {
    fn new(is_repeating: bool, is_reversing: bool) -> TimeScaleLoopState {
        Self {
            is_repeating,
            is_reversing,
        }
    }

    #[cfg(test)]
    fn repeating() -> Self {
        Self {
            is_repeating: true,
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn repeating_and_reversing() -> Self {
        Self {
            is_repeating: true,
            is_reversing: true,
        }
    }

    #[cfg(test)]
    fn reversing() -> Self {
        Self {
            is_reversing: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_before_delay_then_not_started() {
        let timescale = TimeScale::new(10.0, 2.0, Repeat::None, false);

        assert_eq!(timescale.get_position(0.0), TimeScalePosition::NotStarted);
        assert_eq!(timescale.get_position(1.0), TimeScalePosition::NotStarted);
        assert_eq!(timescale.get_position(1.99), TimeScalePosition::NotStarted);
    }

    #[test]
    fn when_after_delay_then_subtracts_delay() {
        let timescale = TimeScale::new(10.0, 2.0, Repeat::None, false);

        assert_eq!(
            timescale.get_position(2.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(7.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(11.5),
            TimeScalePosition::Active(0.95, TimeScaleLoopState::default())
        );
    }

    #[test]
    fn when_no_repeat_or_reverse_then_normalized_by_duration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::None, false);

        assert_eq!(
            timescale.get_position(0.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(2.5),
            TimeScalePosition::Active(0.125, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(10.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(19.0),
            TimeScalePosition::Active(0.95, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(20.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::default())
        );
        assert_eq!(timescale.get_position(21.0), TimeScalePosition::Ended(1.0));
    }

    #[test]
    fn when_repeat_times_then_normalized_by_iteration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Times(2), false);

        assert_eq!(
            timescale.get_position(0.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(5.0),
            TimeScalePosition::Active(0.25, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(19.0),
            TimeScalePosition::Active(0.95, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(20.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(21.0),
            TimeScalePosition::Active(0.05, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(25.0),
            TimeScalePosition::Active(0.25, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(35.0),
            TimeScalePosition::Active(0.75, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(40.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(55.0),
            TimeScalePosition::Active(0.75, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(60.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::repeating())
        );
        assert_eq!(timescale.get_position(61.0), TimeScalePosition::Ended(1.0));
    }

    #[test]
    fn when_repeat_infinite_then_normalized_by_iteration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Infinite, false);

        assert_eq!(
            timescale.get_position(0.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(2.0),
            TimeScalePosition::Active(0.1, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(18.0),
            TimeScalePosition::Active(0.9, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(20.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(22.0),
            TimeScalePosition::Active(0.1, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(38.0),
            TimeScalePosition::Active(0.9, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(40.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(1998.0),
            TimeScalePosition::Active(0.9, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(2002.0),
            TimeScalePosition::Active(0.1, TimeScaleLoopState::repeating())
        );
    }

    #[test]
    fn when_reverse_then_peaks_at_mid_duration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Infinite, true);

        assert_eq!(
            timescale.get_position(0.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(5.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(10.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::default())
        );
        assert_eq!(
            timescale.get_position(15.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::reversing())
        );
        assert_eq!(
            timescale.get_position(20.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::reversing())
        );
        assert_eq!(
            timescale.get_position(22.5),
            TimeScalePosition::Active(0.25, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(25.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(27.5),
            TimeScalePosition::Active(0.75, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(30.0),
            TimeScalePosition::Active(1.0, TimeScaleLoopState::repeating())
        );
        assert_eq!(
            timescale.get_position(32.5),
            TimeScalePosition::Active(0.75, TimeScaleLoopState::repeating_and_reversing())
        );
        assert_eq!(
            timescale.get_position(35.0),
            TimeScalePosition::Active(0.5, TimeScaleLoopState::repeating_and_reversing())
        );
        assert_eq!(
            timescale.get_position(37.5),
            TimeScalePosition::Active(0.25, TimeScaleLoopState::repeating_and_reversing())
        );
        assert_eq!(
            timescale.get_position(40.0),
            TimeScalePosition::Active(0.0, TimeScaleLoopState::repeating_and_reversing())
        );
    }

    #[test]
    fn when_reverse_then_ends_at_zero() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::None, true);

        assert_eq!(timescale.get_position(25.0), TimeScalePosition::Ended(0.0));
    }

    #[test]
    fn get_cycle_duration_ignores_delay_and_repetitions() {
        let single_timescale = TimeScale::new(20.0, 3.0, Repeat::None, false);
        let repeating_timescale = TimeScale::new(20.0, 3.0, Repeat::Infinite, false);

        assert_eq!(single_timescale.get_cycle_duration(), 20.0);
        assert_eq!(repeating_timescale.get_cycle_duration(), 20.0);
    }

    #[test]
    fn get_delay_returns_delay() {
        let single_timescale = TimeScale::new(20.0, 3.0, Repeat::None, false);
        let repeating_timescale = TimeScale::new(20.0, 3.0, Repeat::Infinite, false);

        assert_eq!(single_timescale.get_delay(), 3.0);
        assert_eq!(repeating_timescale.get_delay(), 3.0);
    }

    #[test]
    fn get_duration_includes_delay_and_repetitions() {
        let single_timescale = TimeScale::new(20.0, 3.0, Repeat::None, false);
        let repeating_timescale = TimeScale::new(20.0, 3.0, Repeat::Times(5), true);
        let infinite_timescale = TimeScale::new(20.0, 3.0, Repeat::Infinite, false);

        assert_eq!(single_timescale.get_duration(), 23.0);
        assert_eq!(repeating_timescale.get_duration(), 123.0);
        assert_eq!(infinite_timescale.get_duration(), f32::INFINITY);
    }

    #[test]
    fn get_repeat_returns_delay() {
        let single_timescale = TimeScale::new(20.0, 3.0, Repeat::None, true);
        let repeating_timescale = TimeScale::new(20.0, 3.0, Repeat::Times(5), true);
        let infinite_timescale = TimeScale::new(20.0, 3.0, Repeat::Infinite, true);

        assert_eq!(single_timescale.get_repeat(), Repeat::None);
        assert_eq!(repeating_timescale.get_repeat(), Repeat::Times(5));
        assert_eq!(infinite_timescale.get_repeat(), Repeat::Infinite);
    }
}
