use mina::prelude::*;
use nannou::prelude::*;

/// Demonstrates different ways of handling delays, and their practical uses.
///
/// A [`Timeline`] itself can be configured with a delay via the
/// [delay_seconds](mina::TimelineConfiguration::delay_seconds) configuration method; this is
/// normally going to be the preferred option when animations are automatically triggered by state
/// changes, especially if timing is centralized, e.g. in a Bevy generic system that queries all
/// animatable components and updates them in one place.
///
/// A second way to handle delays, using only a single timeline, is to intercept and change the
/// `time` passed to the [`Timeline::update`] function. By varying the time slightly, multiple
/// objects can take the properties from different locations on the same timeline. This approach can
/// sometimes be more useful when writing complex animations, possibly involving hundreds or
/// thousands of objects that all follow the same or similar path, like a pseudo particle effect.
/// Since only a single shared timeline is used, this is more memory efficient for such scenarios.
///
/// This example also illustrates the use of partial keyframes to produce different paths per
/// property, and in particular how interpolation for any given property always occurs between the
/// keyframes that actually specify the property, which preserves easing. Thus if (for example) an
/// alpha transition starts midway through the timeline with its own easing, it does not affect the
/// curves of any transitions that had already started, such as scale or position.
///
/// Note also that the "progress" animation does not specify an `x` value for any keyframes, and as
/// a result, the shape _preserves_ the `x` value it was created with; it does not reset to some
/// default value.

fn main() {
    nannou::app(model).update(update).run();
}

#[derive(Animate, Clone, Copy)]
struct Shape {
    #[animate] x: f32,
    y: f32,
    #[animate] size: f32,
    color: Srgba<u8>,
    #[animate] alpha: f32,
}

impl Shape {
    pub fn new(x: f32, y: f32, size: f32, color: impl Into<Srgba<u8>>) -> Self {
        Shape { x, y, size, color: color.into(), alpha: 1.0 }
    }
}

struct Model {
    _window: window::Id,
    bounce_shapes: [Shape; 10],
    bounce_timeline: ShapeTimeline,
    progress_shapes: [Shape; 3],
    progress_timelines: [ShapeTimeline; 3],
}

fn model(app: &App) -> Model {
    let _window = app.new_window().size(800, 450).view(view).build().unwrap();

    let progress_shapes = [
        Shape::new(-50.0, 100.0, 20.0, CORNFLOWERBLUE),
        Shape::new(0.0, 100.0, 20.0, CORNFLOWERBLUE),
        Shape::new(50.0, 100.0, 20.0, CORNFLOWERBLUE)
    ];
    let progress_timeline_builder = Shape::timeline()
        .duration_seconds(3.0)
        .repeat(Repeat::Infinite)
        .default_easing(Easing::OutExpo)
        .keyframe(Shape::keyframe(0.0).size(0.0).alpha(0.0))
        .keyframe(Shape::keyframe(0.25).alpha(1.0))
        .keyframe(Shape::keyframe(0.35).alpha(1.0).easing(Easing::OutSine))
        .keyframe(Shape::keyframe(0.5).size(15.0))
        .keyframe(Shape::keyframe(0.75).alpha(0.0));
    let progress_timelines = [
        progress_timeline_builder.clone().build(),
        progress_timeline_builder.clone().delay_seconds(0.25).build(),
        progress_timeline_builder.delay_seconds(0.5).build()
    ];

    let mut bounce_color = Srgba::from(ORANGE);
    bounce_color.alpha = 120;
    let bounce_shapes = [Shape::new(0.0, -100.0, 25.0, bounce_color); 10];
    let bounce_timeline = Shape::timeline()
        .duration_seconds(5.0)
        .repeat(Repeat::Infinite)
        .reverse(true)
        .default_easing(Easing::InOutQuint)
        .keyframe(Shape::keyframe(0.0).x(-300.0).size(15.0))
        .keyframe(Shape::keyframe(0.5).size(5.0))
        .keyframe(Shape::keyframe(1.0).x(300.0).size(15.0))
        .build();

    Model {
        _window,
        bounce_shapes,
        bounce_timeline,
        progress_shapes,
        progress_timelines,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let time = update.since_start.as_secs_f32();
    for (shape, timeline) in model.progress_shapes
        .iter_mut()
        .zip(model.progress_timelines.iter_mut())
    {
        timeline.update(shape, time);
    }
    for (i, shape) in model.bounce_shapes.iter_mut().enumerate() {
        model.bounce_timeline.update(shape, time - 0.15 * i as f32);
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(Srgb::new(0.1, 0.1, 0.1));
    for shape in &model.progress_shapes {
        let mut color = shape.color;
        color.alpha = (shape.alpha * 255.0) as u8;
        draw.ellipse()
            .x(shape.x)
            .y(shape.y)
            .radius(shape.size)
            .color(color);
    }
    for shape in &model.bounce_shapes {
        draw.ellipse()
            .x(shape.x)
            .y(shape.y)
            .radius(shape.size)
            .color(shape.color);
    }
    draw.to_frame(app, &frame).unwrap();
}
