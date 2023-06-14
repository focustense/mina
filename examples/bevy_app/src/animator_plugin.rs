use crate::registry::Registry;
use bevy::prelude::*;
use bevy_mod_picking::events::{Down, Out, Over, Up};
use bevy_mod_picking::prelude::OnPointer;
use enum_map::EnumArray;
use mina::prelude::*;

/// Plugin for integrating the [`StateAnimator`] and related types with Bevy.
///
/// For each timeline type registered via [`AnimatorPlugin::add_timeline`], when an
/// [`AnimatorBundle`] is spawned with some animator, it will automatically set up Bevy's picking
/// mod to update the animator state based on mouse interactions, as well as advancing the animation
/// on every frame. It may still be up to the app to decide what to do with the animated values.
///
/// Note: Several types work on the specific [`EnumStateAnimator`] struct, but supposing we were
/// willing to sacrifice a tiny bit of performance for ergonomics, this could easily be made to work
/// on any boxed implementation of [`StateAnimator`].
pub struct AnimatorPlugin {
    registry: Registry,
}

impl AnimatorPlugin {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
        }
    }

    /// Registers a timeline type for animation.
    ///
    /// This method only needs to know the [`Timeline`] type in order to resolve the correct
    /// animator and system types.
    pub fn add_timeline<T>(mut self) -> Self
    where
        T: Timeline + Send + Sync + 'static,
        T::Target: Clone + Send + Sync,
    {
        self.registry.add(|app| {
            app.add_system(animate_all::<T>);
        });
        self
    }
}

impl Plugin for AnimatorPlugin {
    fn build(&self, app: &mut App) {
        self.registry.apply(app);
    }
}

/// Simple animator state representing common mouse interactions.
#[derive(Clone, Default, Eq, PartialEq, State)]
pub enum Interaction {
    /// No mouse interaction.
    #[default] None,
    /// Mouse cursor is over the target, but button is not pressed.
    Over,
    /// Mouse cursor is over the target, _and_ button is pressed.
    Down,
}

/// Newtype for an animator in order to make it a bevy [`Component`].
#[derive(Component)]
pub struct Animator<State, T>(pub EnumStateAnimator<State, T>)
where
    State: Clone + EnumArray<Option<MergedTimeline<T>>> + PartialEq,
    T: Timeline,
    T::Target: Clone;

/// Type alias for the type of animator we generally care about, using [`Interaction`] for state.
pub type InteractionAnimator<T> = Animator<Interaction, T>;

impl<T> InteractionAnimator<T>
where
    T: Timeline,
    T::Target: Clone,
{
    /// Gets the current animator values.
    pub fn current_values(&self) -> &T::Target {
        self.0.current_values()
    }

    fn set_down(&mut self, is_down: bool) {
        let was_down = self.0.current_state() == &Interaction::Down;
        if is_down != was_down {
            // In bevy_mod_picking, click/up events can only happen when the cursor is still over
            // the target; so if we receive this at all, we know the next interaction is always
            // `Over` and not `None`.
            self.0.set_state(if is_down {
                &Interaction::Down
            } else {
                &Interaction::Over
            });
        }
    }

    fn set_over(&mut self, is_over: bool) {
        if !is_over {
            self.0.set_state(&Interaction::None);
        } else if self.0.current_state() != &Interaction::Down {
            self.0.set_state(&Interaction::Over);
        }
    }
}

/// Utility bundle for a component that animates according to pointer events.
///
/// Requires the Bevy Picking mod to be active, and updates an [`Interaction`]-based animator
/// according to mouse over/out/down/up events.
#[derive(Bundle)]
pub struct AnimatorBundle<T>
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    animator: InteractionAnimator<T>,
    pointer_over: OnPointer<Over>,
    pointer_out: OnPointer<Out>,
    pointer_down: OnPointer<Down>,
    pointer_up: OnPointer<Up>,
}

impl<T> AnimatorBundle<T>
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    pub fn new(animator: EnumStateAnimator<Interaction, T>) -> Self {
        Self {
            animator: Animator(animator),
            pointer_over: OnPointer::<Over>::target_component_mut::<InteractionAnimator<T>>(
                |_, animator| {
                    animator.set_over(true);
                },
            ),
            pointer_out: OnPointer::<Out>::target_component_mut::<InteractionAnimator<T>>(
                |_, animator| {
                    animator.set_over(false);
                },
            ),
            pointer_down: OnPointer::<Down>::target_component_mut::<InteractionAnimator<T>>(
                |_, animator| {
                    animator.set_down(true);
                },
            ),
            pointer_up: OnPointer::<Up>::target_component_mut::<InteractionAnimator<T>>(
                |_, animator| {
                    animator.set_down(false);
                },
            ),
        }
    }
}

fn animate_all<T>(time: Res<Time>, mut animators: Query<&mut InteractionAnimator<T>>)
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    let elapsed_seconds = time.delta_seconds();
    for mut animator in animators.iter_mut() {
        animator.0.advance(elapsed_seconds);
    }
}
