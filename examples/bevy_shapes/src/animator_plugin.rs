use bevy::prelude::*;
use bevy_mod_picking::events::{Down, Out, Over, Up};
use bevy_mod_picking::prelude::OnPointer;
use mina::prelude::*;
use std::marker::PhantomData;

pub struct AnimatorPlugin {
    registrations: Vec<Box<dyn AnimatorRegistration>>,
}

impl AnimatorPlugin {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    pub fn add_timeline<T>(mut self) -> Self
    where
        T: Timeline + Send + Sync + 'static,
        T::Target: Clone + Send + Sync,
    {
        self.registrations
            .push(Box::new(AnimatorRegistrationImpl::<T>::new()));
        self
    }
}

impl Plugin for AnimatorPlugin {
    fn build(&self, app: &mut App) {
        for registration in &self.registrations {
            registration.setup(app);
        }
    }
}

trait AnimatorRegistration: Send + Sync {
    fn setup(&self, app: &mut App);
}

struct AnimatorRegistrationImpl<T: Timeline> {
    phantom: PhantomData<T>,
}

impl<T: Timeline> AnimatorRegistrationImpl<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> AnimatorRegistration for AnimatorRegistrationImpl<T>
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    fn setup(&self, app: &mut App) {
        app.add_system(animate_all::<T>);
    }
}

#[derive(Clone, Default, Eq, PartialEq, State)]
pub enum Interaction {
    #[default]
    None,
    Over,
    Down,
}

#[derive(Component)]
pub struct Animator<T>(EnumStateAnimator<Interaction, T>)
where
    T: Timeline,
    T::Target: Clone;

impl<T> Animator<T>
where
    T: Timeline,
    T::Target: Clone,
{
    pub fn current_values(&self) -> &T::Target {
        self.0.current_values()
    }

    pub fn set_down(&mut self, is_down: bool) {
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

    pub fn set_over(&mut self, is_over: bool) {
        if !is_over {
            self.0.set_state(&Interaction::None);
        } else if self.0.current_state() != &Interaction::Down {
            self.0.set_state(&Interaction::Over);
        }
    }
}

#[derive(Bundle)]
pub struct AnimatorBundle<T>
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    pub animator: Animator<T>,
    pub pointer_over: OnPointer<Over>,
    pub pointer_out: OnPointer<Out>,
    pub pointer_down: OnPointer<Down>,
    pub pointer_up: OnPointer<Up>,
}

impl<T> AnimatorBundle<T>
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    pub fn new(animator: EnumStateAnimator<Interaction, T>) -> Self {
        Self {
            animator: Animator(animator),
            pointer_over: OnPointer::<Over>::target_component_mut::<Animator<T>>(|_, animator| {
                animator.set_over(true);
            }),
            pointer_out: OnPointer::<Out>::target_component_mut::<Animator<T>>(|_, animator| {
                animator.set_over(false);
            }),
            pointer_down: OnPointer::<Down>::target_component_mut::<Animator<T>>(|_, animator| {
                animator.set_down(true);
            }),
            pointer_up: OnPointer::<Up>::target_component_mut::<Animator<T>>(|_, animator| {
                animator.set_down(false);
            }),
        }
    }
}

fn animate_all<T>(time: Res<Time>, mut animators: Query<&mut Animator<T>>)
where
    T: Timeline + Send + Sync + 'static,
    T::Target: Clone + Send + Sync,
{
    let elapsed_seconds = time.delta_seconds();
    for mut animator in animators.iter_mut() {
        animator.0.advance(elapsed_seconds);
    }
}
