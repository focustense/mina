pub mod prelude;

pub use mina_core::{
    easing::{Easing, EasingFunction},
    interpolation::Lerp,
    timeline::{
        KeyframeBuilder, MergedTimeline, Repeat, Timeline, TimelineBuilder, TimelineConfiguration,
    },
};
pub use mina_macros::Animate;
