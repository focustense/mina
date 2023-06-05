use mina::{Animate, Easing, Repeat, Timeline, TimelineBuilder};
use nannou::prelude::*;

fn main() {
    nannou::app(model).update(update).run();
}

#[derive(Animate)]
struct Shape {
    size: f32,
    #[animate] x: f32,
    #[animate] y: f32,
}

impl Shape {
    pub fn new(size: f32) -> Self {
        Shape { size, x: 0.0, y: 0.0 }
    }
}

struct Model {
    _window: window::Id,
    timeline: ShapeTimeline,
    shape: Shape,
}

fn model(app: &App) -> Model {
    let _window = app.new_window().size(800, 450).view(view).build().unwrap();
    let shape = Shape::new(28.0);
    let timeline = Shape::timeline()
        .duration_seconds(5.0)
        .repeat(Repeat::Infinite)
        .default_easing(Easing::OutCubic)
        .keyframe(Shape::keyframe(0.0).x(-300.0).y(120.0))
        .keyframe(Shape::keyframe(0.25).x(300.0).y(120.0))
        .keyframe(Shape::keyframe(0.5).x(300.0).y(-120.0))
        .keyframe(Shape::keyframe(0.75).x(-300.0).y(-120.0))
        .keyframe(Shape::keyframe(1.0).x(-300.0).y(120.0))
        .build();
    Model { _window, timeline, shape }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let time = update.since_start.as_secs_f32();
    let values = model.timeline.values_at(time);
    values.update(&mut model.shape);
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(Srgb::new(0.1, 0.1, 0.1));
    draw.ellipse()
        .x(model.shape.x)
        .y(model.shape.y)
        .radius(model.shape.size)
        .color(STEELBLUE);
    draw.to_frame(app, &frame).unwrap();
}
