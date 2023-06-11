use crate::widget_helpers::{Interaction, WidgetAnimator};
use iced::{window, Element, Length, Padding, Point, Rectangle};
use iced_native::{
    event::Status,
    layout::{Limits, Node},
    renderer,
    renderer::Style,
    widget::{tree, tree::Tag, Operation, Tree},
    Clipboard, Event, Layout, Shell, Widget,
};
use mina::prelude::*;

/// A widget that clips content along the X-axis when collapsed, and expands (with animation) to
/// show the full content on hover.
///
/// Unlike most animated widgets, this one affects layout and requires layout invalidation on each
/// frame while it is animating, because it is common for an expander to be _inside_ something like
/// a button or toggle. Best used somewhere inside a fixed-width container, e.g. as a child of a
/// [`Button`](iced::widget::Button) that is itself inside a fixed-width
/// [`Column`](iced::widget::Column).
pub struct Expander<'a, Message, Renderer> {
    collapsed_width: f32,
    // Ideally, we would not store a transient state field in the widget itself; however, in this
    // example, it's important for the width to actually affect layout, since we're going to be
    // growing a button to reveal the inner text. Unfortunately, Iced doesn't give us access to the
    // state tree in the layout method, so we have to use this roundabout method, where we apply
    // expanded ratio to true content width in the update method, assign it here, and invalidate the
    // layout. Subsequently, on the next layout pass, we'll have the correct dimension.
    current_width: f32,
    content: Element<'a, Message, Renderer>,
    padding: Padding,
}

#[derive(Animate, Clone, Debug, Default)]
struct Effects {
    expanded_ratio: f32,
}

impl<'a, Message, Renderer> Expander<'a, Message, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        Self {
            collapsed_width: 0.0,
            content: content.into(),
            current_width: 0.0,
            padding: Padding::ZERO,
        }
    }

    pub fn collapsed_width(mut self, collapsed_width: f32) -> Self {
        self.collapsed_width = collapsed_width;
        self.current_width = collapsed_width;
        self
    }

    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Expander<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        // Layout in this widget is a little odd. We need the content to be as big as it wants to
        // be (so we know the correct expanded width), but our own layout may be collapsed or only
        // partially expanded.
        let mut content = self.content.as_widget().layout(renderer, limits);
        let padding = self.padding.fit(content.size(), limits.max());
        content.move_to(Point::new(padding.left, padding.top));
        let mut size = limits.pad(padding).resolve(content.size());
        size.width = self.current_width.max(self.collapsed_width);
        Node::with_children(size.pad(padding), vec![content])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        renderer.with_layer(*viewport, |renderer| {
            let content_layout = layout.children().next().unwrap();
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content_layout,
                cursor_position,
                viewport,
            )
        });
    }

    fn tag(&self) -> Tag {
        Tag::of::<WidgetAnimator<EffectsTimeline>>()
    }

    fn state(&self) -> tree::State {
        let animator = animator!(Effects {
            Interaction::None => 0.4s Easing::OutCubic from default to default,
            Interaction::Over => 0.4s Easing::OutCubic to { expanded_ratio: 1.0 }
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
        if layout.bounds().contains(cursor_position) {
            animator.set_interaction(&Interaction::Over)
        } else {
            animator.set_interaction(&Interaction::None)
        }
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            animator.sync(now);
            let effects = animator.current_values();
            let content_layout = layout.children().next().unwrap();
            let expand_width = content_layout.bounds().width - self.collapsed_width;
            let next_width = self.collapsed_width + expand_width * effects.expanded_ratio;
            if self.current_width != next_width {
                self.current_width = next_width;
                shell.invalidate_layout();
            }
        }
        Status::Ignored
    }
}

impl<'a, Message, Renderer> From<Expander<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(value: Expander<'a, Message, Renderer>) -> Self {
        Self::new(value)
    }
}
