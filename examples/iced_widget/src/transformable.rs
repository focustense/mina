use crate::widget_helpers::WidgetAnimator;
use iced::{mouse::Interaction, window, Element, Length, Point, Rectangle, Vector};
use iced_native::{
    event::Status,
    layout::{Limits, Node},
    renderer,
    renderer::Style,
    widget::{tree, tree::Tag, Id, Operation, Tree},
    window::RedrawRequest,
    Clipboard, Event, Layout, Shell, Widget,
};
use mina::prelude::*;
use std::any::Any;

/// A widget that animates between two transforms.
///
/// Does not trigger its own state changes; instead it is designed for remote-control, via the
/// [`TransformOperation`]. For the purposes of this example, only two states ("off" and "on") are
/// supported, although this could be extended to support any number of possible states.
///
/// Ideally a widget like this would also support specifying the animation duration and easing type.
/// There are some limitations to the [`Animate`] macro which prevent doing this right now, although
/// it is possible when using the builder syntax instead.
///
/// Uses Iced's translation primitive to avoid requiring new layout on each frame.
pub struct Transformable<'a, Message, Renderer> {
    content: Element<'a, Message, Renderer>,
    id: Option<Id>,
    off_transform: Transform,
    on_transform: Transform,
}

#[derive(Animate, Clone, Debug, Default)]
pub struct Transform {
    translate_x: f32,
    translate_y: f32,
}

impl Transform {
    pub fn new(translate_x: f32, translate_y: f32) -> Self {
        Self {
            translate_x,
            translate_y,
        }
    }
}

#[derive(Clone, Default, Eq, PartialEq, State)]
pub enum TransformStatus {
    #[default]
    Off,
    On,
}

impl<'a, Message, Renderer> Transformable<'a, Message, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Renderer>>) -> Self {
        Self {
            content: content.into(),
            id: None,
            off_transform: Default::default(),
            on_transform: Default::default(),
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn off_transform(mut self, transform: Transform) -> Self {
        self.off_transform = transform;
        self
    }

    pub fn on_transform(mut self, transform: Transform) -> Self {
        self.on_transform = transform;
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Transformable<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.content.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let content = self.content.as_widget().layout(renderer, limits);
        Node::with_children(content.size(), vec![content])
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
        let content_layout = layout.children().next().unwrap();
        let animator = tree
            .state
            .downcast_ref::<WidgetAnimator<TransformTimeline, TransformStatus>>();
        let transform = animator.current_values();
        renderer.with_translation(
            Vector::new(transform.translate_x, transform.translate_y),
            |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    content_layout,
                    cursor_position,
                    viewport,
                )
            },
        )
    }

    fn tag(&self) -> Tag {
        Tag::of::<WidgetAnimator<TransformTimeline, TransformStatus>>()
    }

    fn state(&self) -> iced_native::widget::tree::State {
        let animator = animator!(Transform {
            default(TransformStatus::Off, self.off_transform.clone()),
            TransformStatus::Off => 0.5s Easing::OutCubic from default to default,
            TransformStatus::On => 0.5s Easing::OutCubic to {
                translate_x: self.on_transform.translate_x,
                translate_y: self.on_transform.translate_y,
            }
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
        let animator = tree
            .state
            .downcast_mut::<WidgetAnimator<TransformTimeline, TransformStatus>>();
        operation.custom(animator, self.id.as_ref());
        operation.container(self.id.as_ref(), &mut |operation| {
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> Status {
        let animator = tree
            .state
            .downcast_mut::<WidgetAnimator<TransformTimeline, TransformStatus>>();
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            animator.sync(now);
        }
        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        );
        shell.request_redraw(RedrawRequest::NextFrame);
        Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor_position,
            viewport,
            renderer,
        )
    }
}

impl<'a, Message, Renderer> From<Transformable<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(value: Transformable<'a, Message, Renderer>) -> Self {
        Self::new(value)
    }
}

/// Operation for changing the state of a ['Transformable'] widget's transform.
///
/// Publish this with a [`Command::widget`](iced::Command::widget) to switch the transform position,
/// e.g. to "show" or "hide" (move off-screen, without affecting layout) some part of the UI.
pub struct TransformOperation {
    target: Id,
    status: TransformStatus,
}

impl TransformOperation {
    pub fn new(target: Id, status: TransformStatus) -> Self {
        Self { target, status }
    }
}

impl<T> Operation<T> for TransformOperation {
    fn container(
        &mut self,
        _id: Option<&Id>,
        operate_on_children: &mut dyn FnMut(&mut dyn Operation<T>),
    ) {
        operate_on_children(self)
    }

    fn custom(&mut self, state: &mut dyn Any, id: Option<&Id>) {
        if Some(&self.target) != id {
            return;
        }
        if let Some(animator) =
            state.downcast_mut::<WidgetAnimator<TransformTimeline, TransformStatus>>()
        {
            animator.set_interaction(&self.status);
        }
    }
}
