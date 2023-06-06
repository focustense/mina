use mina::prelude::*;
use nannou::color::Lch;
use nannou::prelude::*;

/// Example of a [`MergedTimeline`]. Specifies separate timelines for horizontal motion and vertical
/// motion that have different durations and easings; then, Because We Can, adds a third animation
/// for color that doesn't reverse.
///
/// These timelines are all merged to create a single timeline to run the animation. Note how all
/// the inner timelines operate on the same style struct (`[Slider]`), but for different properties,
/// and do not interfere with each other.

fn main() {
    nannou::app(model).update(update).run();
}

#[derive(Animate, Default)]
struct Slider {
    x: f32,
    y: f32,
    rotation: f32,
    hue: f32,
}

struct Model {
    _window: window::Id,
    timeline: MergedTimeline<SliderTimeline>,
    slider: Slider,
}

fn model(app: &App) -> Model {
    let _window = app.new_window().size(800, 450).view(view).build().unwrap();
    let slider = Slider::default();
    let vertical_timeline = Slider::timeline()
        .duration_seconds(20.0)
        .repeat(Repeat::Infinite)
        .reverse(true)
        .default_easing(Easing::Linear)
        .keyframe(Slider::keyframe(0.0).y(150.0))
        .keyframe(Slider::keyframe(1.0).y(-150.0))
        .build();
    let horizontal_timeline = Slider::timeline()
        .duration_seconds(5.0)
        .repeat(Repeat::Infinite)
        .reverse(true)
        .default_easing(Easing::InOutCirc)
        .keyframe(Slider::keyframe(0.0).x(-350.0).rotation(0.0))
        .keyframe(Slider::keyframe(1.0).x(350.0).rotation(PI * 16.0))
        .build();
    let color_timeline = Slider::timeline()
        .duration_seconds(30.0)
        .repeat(Repeat::Infinite)
        .keyframe(Slider::keyframe(0.0).hue(140.0))
        .keyframe(Slider::keyframe(1.0).hue(500.0))
        .build();
    let merged_timeline =
        MergedTimeline::of([vertical_timeline, horizontal_timeline, color_timeline]);
    Model {
        _window,
        slider,
        timeline: merged_timeline,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let time = update.since_start.as_secs_f32();
    model.timeline.update(&mut model.slider, time);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(Srgb::new(0.1, 0.1, 0.1));
    draw.line()
        .start(Vec2::new(-350.0, model.slider.y - 2.0))
        .end(Vec2::new(350.0, model.slider.y - 2.0))
        .color(Srgb::new(0.2, 0.2, 0.2));
    draw.line()
        .start(Vec2::new(-350.0, model.slider.y))
        .end(Vec2::new(350.0, model.slider.y))
        .color(DIMGRAY);
    draw.line()
        .start(Vec2::new(-350.0, model.slider.y + 2.0))
        .end(Vec2::new(350.0, model.slider.y + 2.0))
        .color(Srgb::new(0.2, 0.2, 0.2));
    draw.rect()
        .x(model.slider.x)
        .y(model.slider.y)
        .width(15.0)
        .height(60.0)
        .rotate(-model.slider.rotation)
        .color(Lch::new(55.0, 120.0, model.slider.hue % 360.0))
        .stroke_color(DARKGREEN)
        .stroke_weight(2.0);
    draw.to_frame(app, &frame).unwrap();
}
