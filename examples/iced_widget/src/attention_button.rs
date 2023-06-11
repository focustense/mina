use crate::widget_helpers::{Interaction, WidgetAnimator};
use iced::{mouse, window, Alignment, Background, Color, Element, Length, Point, Rectangle, Size};
use iced_native::window::RedrawRequest;
use iced_native::{
    event::Status,
    layout::{Limits, Node},
    renderer,
    renderer::Style,
    widget::{tree, tree::Tag, Operation, Tree},
    Clipboard, Event, Layout, Shell, Widget,
};
use mina::prelude::*;

const EFFECT_SCALE: f32 = 2.0;

/// A button that really wants your attention and never stops trying to get your attention. Don't
/// use this in a serious app.
///
/// Produces a very basic "ghosting" effect which we call an emission here - simply a circle that
/// pulses outward at a regular interval. A prettier and more complex version might use multiple
/// concentric circles, Bloom effects, etc. An even more advanced version would duplicate the entire
/// content and not just the circle/background. Here we're keeping it basic.
///
/// Hovering will collapse and stop the pulsing. Holding the mouse button down will produce a
/// subtler, single pulse that does not fade out.
///
/// The integration with Iced is mostly standard. We don't use an actual
/// [`Button`](iced::widget::Button) because it doesn't provide the exact hooks or state-tracking
/// behavior we need, so it has to be re-implemented in our `update` function. The animation and
/// drawing code is only about 50 lines, the rest is boilerplate.
pub struct AttentionButton<'a, Message, Renderer> {
    background_color: Color,
    content: Element<'a, Message, Renderer>,
    on_press: Option<Message>,
    radius: f32,
}

#[derive(Animate, Clone, Debug, Default)]
struct Effects {
    background_alpha: f32,
    emission_alpha: f32,
    emission_scale: f32,
}

impl<'a, Message, Renderer> AttentionButton<'a, Message, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        Self {
            background_color: Color::from_rgb(1.0, 0.0, 0.0),
            content: content.into(),
            on_press: None,
            radius: 10.0,
        }
    }

    pub fn background_color(mut self, background_color: Color) -> Self {
        self.background_color = background_color;
        self
    }

    pub fn on_press(mut self, msg: Message) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for AttentionButton<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn width(&self) -> Length {
        Length::Fixed(self.radius * EFFECT_SCALE * 2.0)
    }

    fn height(&self) -> Length {
        Length::Fixed(self.radius * EFFECT_SCALE * 2.0)
    }

    fn layout(&self, renderer: &Renderer, _limits: &Limits) -> Node {
        let inner_size = Size::new(self.radius * 2.0, self.radius * 2.0);
        let content_limits = Limits::NONE
            .width(Length::Shrink)
            .height(Length::Shrink)
            .max_width(inner_size.width)
            .max_height(inner_size.height);
        let mut content = self.content.as_widget().layout(renderer, &content_limits);
        let outer_size = Size::new(
            inner_size.width * EFFECT_SCALE,
            inner_size.height * EFFECT_SCALE,
        );
        content.align(Alignment::Center, Alignment::Center, outer_size);
        Node::with_children(outer_size, vec![content])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        let outer_bounds = layout.bounds();
        let center = outer_bounds.center();
        let animator = tree.state.downcast_ref::<WidgetAnimator<EffectsTimeline>>();
        let effects = animator.current_values();
        draw_circle_centered(
            renderer,
            center,
            effects.emission_scale * self.radius,
            Color::from_rgba(1.0, 1.0, 1.0, effects.emission_alpha),
        );
        renderer.with_layer(*viewport, |renderer| {
            // Draw an opaque clearing shape so that we don't see the emission effect underneath a
            // semi-transparent button.
            draw_circle_centered(renderer, center, self.radius, Color::BLACK);
            let Color { r, g, b, a } = self.background_color;
            let background_color = Color::from_rgba(r, g, b, a * effects.background_alpha);
            draw_circle_centered(renderer, center, self.radius, background_color);
            let content_layout = layout.children().next().unwrap();
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                &Style::default(),
                content_layout,
                cursor_position,
                &bounds_from_center(center, self.radius),
            );
        });
    }

    fn tag(&self) -> Tag {
        Tag::of::<WidgetAnimator<EffectsTimeline>>()
    }

    fn state(&self) -> tree::State {
        let animator = animator!(Effects {
            default(Interaction::None, {
                background_alpha: 0.5,
                emission_alpha: 0.25,
                emission_scale: 0.85
            }),
            Interaction::None => [
                0.5s Easing::OutCubic to { background_alpha: 0.5 },
                2s Easing::OutQuint infinite
                    from { emission_alpha: 0.0, emission_scale: 0.0 }
                    2% { emission_alpha: 0.15, emission_scale: 0.0 }
                    5% { emission_scale: 0.85 } Easing::InOutCirc
                    75% { emission_alpha: 0.0 }
                    100% { emission_scale: EFFECT_SCALE },
            ],
            Interaction::Over => 0.5s Easing::OutCubic to {
                background_alpha: 0.8,
                emission_alpha: 0.0,
                emission_scale: 0.85,
            },
            Interaction::Down => [
                0.5s Easing::OutCubic to {
                    background_alpha: 1.0,
                    emission_alpha: 0.0,
                    emission_scale: 0.85,
                },
                3s Easing::OutExpo
                    1% { emission_scale: 1.05 }
                    to { emission_alpha: 0.1, emission_scale: 1.5 }
            ]
        });
        tree::State::new(WidgetAnimator::new(animator))
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content])
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        operation.container(None, &mut |operation| {
            self.content.as_widget().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> Status {
        let animator = tree.state.downcast_mut::<WidgetAnimator<EffectsTimeline>>();
        let inner_bounds = bounds_from_center(layout.bounds().center(), self.radius);
        let is_over = inner_bounds.contains(cursor_position);
        let default_interaction = if is_over {
            Interaction::Over
        } else {
            Interaction::None
        };
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if is_over {
                    animator.set_interaction(&Interaction::Down);
                    return Status::Captured;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if animator.current_interaction() == &Interaction::Down {
                    animator.set_interaction(&default_interaction);
                    if is_over {
                        if let Some(ref on_press) = self.on_press {
                            shell.publish(on_press.clone());
                        }
                    }
                    return Status::Captured;
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                animator.sync(now);
            }
            _ => {}
        }
        if animator.current_interaction() != &Interaction::Down {
            animator.set_interaction(&default_interaction);
        }
        shell.request_redraw(RedrawRequest::NextFrame);
        Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        let inner_bounds = bounds_from_center(layout.bounds().center(), self.radius);
        if inner_bounds.contains(cursor_position) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message, Renderer> From<AttentionButton<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(value: AttentionButton<'a, Message, Renderer>) -> Self {
        Self::new(value)
    }
}

fn bounds_from_center(center: Point, radius: f32) -> Rectangle {
    Rectangle::new(
        Point::new(center.x - radius, center.y - radius),
        Size::new(radius * 2.0, radius * 2.0),
    )
}

fn draw_circle_centered<Renderer>(renderer: &mut Renderer, center: Point, radius: f32, color: Color)
where
    Renderer: renderer::Renderer,
{
    let bounds = bounds_from_center(center, radius);
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border_radius: radius.into(),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        },
        Background::Color(color),
    );
}
