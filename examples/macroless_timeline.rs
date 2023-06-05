use mina::{Easing, Repeat, Timeline, TimelineBuilder, TimelineConfiguration};
use mina_core::{
    time_scale::TimeScale,
    timeline::{
        prepare_frame, Keyframe, KeyframeBuilder, TimelineBuilderArguments,
    },
    timeline_helpers::SubTimeline,
};

// Demonstrates how to write an entire set of timeline and keyframe classes explicitly, without
// using any of the proc macros.
//
// The purpose of this example is primarily to be a living template for the proc macros, so that
// their output is easy to compare with a known-good example when debugging. It may also help some
// users to understand the API a little better, though most users should ignore it.
//
// Most of what follows is the boilerplate that is, or should be, generated by the macro. Skip all
// the way to the `main()` function at the end for how to actually build and use the timeline.

pub struct Style {
    // Normally our Style would have all of these fields in use, and public; then, on animation
    // frames, the animator would assign the values onto this from `StyleAnimatorValues`. Since this
    // example focuses on only the timeline itself, the fields are not used, and are only shown
    // (with underscores) as a reference to what the animator values are generated from.
    _x: u32,
    _y: u32,
    _scale: u32,
}

impl Style {
    pub fn keyframe(normalized_time: f32) -> StyleKeyframeBuilder {
        StyleKeyframeBuilder::new(normalized_time)
    }

    pub fn timeline() -> TimelineConfiguration<StyleKeyframeData> {
        TimelineConfiguration::default()
    }
}

#[derive(Debug, Default)]
struct StyleAnimatorValues {
    x: u32,
    y: u32,
    scale: f32,
}

struct StyleTimeline {
    boundary_times: Vec<f32>,
    timescale: TimeScale,
    t_x: SubTimeline<u32>,
    t_y: SubTimeline<u32>,
    t_scale: SubTimeline<f32>,
}

impl Timeline for StyleTimeline {
    type Values = StyleAnimatorValues;

    fn values_at(&self, time: f32) -> Self::Values {
        let mut values = Self::Values::default();
        let Some((normalized_time, frame_index)) = prepare_frame(
            time, self.boundary_times.as_slice(), &self.timescale
        ) else {
            return values;
        };
        if let Some(x) = self.t_x.value_at(normalized_time, frame_index) {
            values.x = x;
        }
        if let Some(y) = self.t_y.value_at(normalized_time, frame_index) {
            values.y = y;
        }
        if let Some(scale) = self.t_scale.value_at(normalized_time, frame_index) {
            values.scale = scale;
        }
        values
    }
}

impl TimelineBuilder<StyleTimeline> for TimelineConfiguration<StyleKeyframeData> {
    fn build(self) -> StyleTimeline {
        let args = TimelineBuilderArguments::from(self);
        let defaults = StyleAnimatorValues::default();
        StyleTimeline {
            timescale: args.timescale,
            t_x: SubTimeline::from_keyframes(
                &args.keyframes,
                defaults.x,
                |k| k.x,
                args.default_easing.clone(),
            ),
            t_y: SubTimeline::from_keyframes(
                &args.keyframes,
                defaults.y,
                |k| k.y,
                args.default_easing.clone(),
            ),
            t_scale: SubTimeline::from_keyframes(
                &args.keyframes,
                defaults.scale,
                |k| k.scale,
                args.default_easing.clone(),
            ),
            boundary_times: args.boundary_times,
        }
    }
}

#[derive(Clone, Default)]
pub struct StyleKeyframeData {
    x: Option<u32>,
    y: Option<u32>,
    scale: Option<f32>,
}

pub struct StyleKeyframeBuilder {
    data: StyleKeyframeData,
    easing: Option<Easing>,
    normalized_time: f32,
}

impl StyleKeyframeBuilder {
    fn new(normalized_time: f32) -> Self {
        Self {
            normalized_time,
            data: Default::default(),
            easing: None,
        }
    }

    pub fn x(mut self, x: u32) -> Self {
        self.data.x = Some(x);
        self
    }

    pub fn y(mut self, y: u32) -> Self {
        self.data.y = Some(y);
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.data.scale = Some(scale);
        self
    }
}

impl KeyframeBuilder for StyleKeyframeBuilder {
    type Data = StyleKeyframeData;

    fn build(&self) -> Keyframe<StyleKeyframeData> {
        Keyframe::new(self.normalized_time, self.data.clone(), self.easing.clone())
    }

    fn easing(mut self, easing: Easing) -> Self {
        self.easing = Some(easing);
        self
    }
}

fn main() {
    let timeline: StyleTimeline = Style::timeline()
        .duration_seconds(10.0)
        .delay_seconds(5.0)
        .default_easing(Easing::Ease)
        .repeat(Repeat::Times(2))
        .keyframe(Style::keyframe(0.0).scale(1.0))
        .keyframe(Style::keyframe(0.25).x(200))
        .keyframe(Style::keyframe(0.5).x(200).y(50))
        .keyframe(Style::keyframe(0.75).x(0).y(50))
        .keyframe(Style::keyframe(1.0).y(0).scale(2.0))
        .build();

    for i in 0..=100 {
        let time = i as f32 * 0.5;
        let values = timeline.values_at(time);
        println!("Values at t = {time}: {:?}", values);
    }
}
