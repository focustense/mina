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
    #[cfg(test)]
    fn new(duration: f32, delay: f32, repeat: Repeat, reverse: bool) -> Self {
        Self {
            duration,
            delay,
            repeat,
            reverse,
        }
    }

    /// Computes the normalized time or "timeline position" for some real time.
    ///
    /// # Arguments
    ///
    /// * `time` - Elapsed time in the same units as the timescale's duration.
    ///
    /// # Returns
    ///
    /// The normalized time between `0.0` and `1.0`. This is the position on the timeline described
    /// in terms of keyframe times, which are also between `0.0` (0%) and `1.0` (100%).
    ///
    /// * For example, if the animator is configured to reverse, then the last keyframe is reached
    /// (result = `1.0`) when `time` is at 50% of the configured duration, and declines back to
    /// `0.0` until 100% of the duration is reached.
    /// * If not reversing, then the normalized time increases monotonically from `0.0` to `1.0`
    /// until either the animation fully ends (remains at `1.0`) or the next loop begins (resets to
    /// `0.0`).
    ///
    /// If the `time` is nowhere on the timeline, returns an [`TimeScaleOutOfBounds`] error.
    pub fn get_normalized_time(&self, time: f32) -> Result<f32, TimeScaleOutOfBounds> {
        let time = time - self.delay;
        if time < 0.0 {
            return Err(TimeScaleOutOfBounds::NotStarted);
        }
        let cycle_time = match self.repeat {
            Repeat::None if time > self.duration => Err(TimeScaleOutOfBounds::Ended),
            Repeat::None => Ok(time),
            Repeat::Times(times) if time > self.duration * (times + 1) as f32 => {
                Err(TimeScaleOutOfBounds::Ended)
            }
            Repeat::Times(_) | Repeat::Infinite => Ok({
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
                if rem == 0.0 && quot >= 1.0 { self.duration } else { rem }
            }),
        }? / self.duration;
        let normalized_time = match self.reverse {
            true if cycle_time > 0.5 => (1.0 - cycle_time) * 2.0,
            true => cycle_time * 2.0,
            false => cycle_time,
        };
        Ok(normalized_time)
    }
}

/// Error produced by [`TimeScale::get_normalized_time`], specifying which boundary is exceeded by
/// a given time.
#[derive(Debug, Eq, PartialEq)]
pub enum TimeScaleOutOfBounds {
    /// The timeline has not started at the specified time, either because the time was negative or
    /// because it is within the configured delay period. When determining animator values, this can
    /// be considered equivalent to a normalized time of `0.0`.
    NotStarted,
    /// The timeline has already ended at the specified time, i.e. it does not loop infinitely and
    /// the specified time is after the last loop ends. When determining animator values, this can
    /// be considered equivalent to a normalized time of `1.0` if the timescale is forward-only, or
    /// `0.0` if the timeline reverses.
    Ended,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_before_delay_then_not_started() {
        let timescale = TimeScale::new(10.0, 2.0, Repeat::None, false);

        assert_eq!(
            timescale.get_normalized_time(0.0),
            Err(TimeScaleOutOfBounds::NotStarted)
        );
        assert_eq!(
            timescale.get_normalized_time(1.0),
            Err(TimeScaleOutOfBounds::NotStarted)
        );
        assert_eq!(
            timescale.get_normalized_time(1.99),
            Err(TimeScaleOutOfBounds::NotStarted)
        );
    }

    #[test]
    fn when_after_delay_then_subtracts_delay() {
        let timescale = TimeScale::new(10.0, 2.0, Repeat::None, false);

        assert_eq!(timescale.get_normalized_time(2.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(7.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(11.5), Ok(0.95));
    }

    #[test]
    fn when_no_repeat_or_reverse_then_normalized_by_duration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::None, false);

        assert_eq!(timescale.get_normalized_time(0.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(2.5), Ok(0.125));
        assert_eq!(timescale.get_normalized_time(10.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(19.0), Ok(0.95));
        assert_eq!(timescale.get_normalized_time(20.0), Ok(1.0));
        assert_eq!(
            timescale.get_normalized_time(21.0),
            Err(TimeScaleOutOfBounds::Ended)
        );
    }

    #[test]
    fn when_repeat_times_then_normalized_by_iteration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Times(2), false);

        assert_eq!(timescale.get_normalized_time(0.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(5.0), Ok(0.25));
        assert_eq!(timescale.get_normalized_time(19.0), Ok(0.95));
        assert_eq!(timescale.get_normalized_time(20.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(21.0), Ok(0.05));
        assert_eq!(timescale.get_normalized_time(25.0), Ok(0.25));
        assert_eq!(timescale.get_normalized_time(35.0), Ok(0.75));
        assert_eq!(timescale.get_normalized_time(40.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(55.0), Ok(0.75));
        assert_eq!(timescale.get_normalized_time(60.0), Ok(1.0));
        assert_eq!(
            timescale.get_normalized_time(61.0),
            Err(TimeScaleOutOfBounds::Ended)
        );
    }

    #[test]
    fn when_repeat_infinite_then_normalized_by_iteration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Infinite, false);

        assert_eq!(timescale.get_normalized_time(0.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(2.0), Ok(0.1));
        assert_eq!(timescale.get_normalized_time(18.0), Ok(0.9));
        assert_eq!(timescale.get_normalized_time(20.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(22.0), Ok(0.1));
        assert_eq!(timescale.get_normalized_time(38.0), Ok(0.9));
        assert_eq!(timescale.get_normalized_time(40.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(1998.0), Ok(0.9));
        assert_eq!(timescale.get_normalized_time(2002.0), Ok(0.1));
    }

    #[test]
    fn when_reverse_then_peaks_at_mid_duration() {
        let timescale = TimeScale::new(20.0, 0.0, Repeat::Infinite, true);

        assert_eq!(timescale.get_normalized_time(0.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(5.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(10.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(15.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(20.0), Ok(0.0));
        assert_eq!(timescale.get_normalized_time(22.5), Ok(0.25));
        assert_eq!(timescale.get_normalized_time(25.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(27.5), Ok(0.75));
        assert_eq!(timescale.get_normalized_time(30.0), Ok(1.0));
        assert_eq!(timescale.get_normalized_time(32.5), Ok(0.75));
        assert_eq!(timescale.get_normalized_time(35.0), Ok(0.5));
        assert_eq!(timescale.get_normalized_time(37.5), Ok(0.25));
        assert_eq!(timescale.get_normalized_time(40.0), Ok(0.0));
    }
}
