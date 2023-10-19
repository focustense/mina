use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_mina::prelude::*;

/// Simple animator state representing common mouse interactions.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum Interaction {
    /// No mouse interaction.
    #[default] None,
    /// Mouse cursor is over the target, but button is not pressed.
    Over,
    /// Mouse cursor is over the target, _and_ button is pressed.
    Down,
}

// Type alias for readability.
type InteractionSelector<T> = AnimationSelector<Interaction, T>;

// Internal helper trait for InteractionSelector to handle nuances of down <--> over transitions.
trait PointerInteractions {
    fn set_down(&mut self, is_down: bool);
    fn set_over(&mut self, is_over: bool);
}

impl<T: Component> PointerInteractions for AnimationSelector<Interaction, T> {
    fn set_down(&mut self, is_down: bool) {
        let was_down = self.timeline_key == Interaction::Down;
        if is_down != was_down {
            // In bevy_mod_picking, click/up events can only happen when the cursor is still over
            // the target; so if we receive this at all, we know the next interaction is always
            // `Over` and not `None`.
            self.timeline_key = if is_down {
                Interaction::Down
            } else {
                Interaction::Over
            };
        }
    }

    fn set_over(&mut self, is_over: bool) {
        if !is_over {
            self.timeline_key = Interaction::None;
        } else if self.timeline_key != Interaction::Down {
            self.timeline_key = Interaction::Over;
        }
    }
}

/// Utility bundle for a component that animates according to pointer events.
///
/// Requires the Bevy Picking mod to be active, and updates an [`Interaction`]-based
/// [AnimationSelector] according to mouse over/out/down/up events.
#[derive(Bundle)]
pub struct PointerInteractionBundle {
    pointer_over: On<Pointer<Over>>,
    pointer_out: On<Pointer<Out>>,
    pointer_down: On<Pointer<Down>>,
    pointer_up: On<Pointer<Up>>,
}

impl PointerInteractionBundle {
    pub fn new<T: Component>() -> Self {
        Self {
            pointer_over: On::<Pointer<Over>>::target_component_mut::<InteractionSelector<T>>(
                |_, animator| {
                    animator.set_over(true);
                },
            ),
            pointer_out: On::<Pointer<Out>>::target_component_mut::<InteractionSelector<T>>(
                |_, animator| {
                    animator.set_over(false);
                },
            ),
            pointer_down: On::<Pointer<Down>>::target_component_mut::<InteractionSelector<T>>(
                |_, animator| {
                    animator.set_down(true);
                },
            ),
            pointer_up: On::<Pointer<Up>>::target_component_mut::<InteractionSelector<T>>(
                |_, animator| {
                    animator.set_down(false);
                },
            ),
        }
    }
}
