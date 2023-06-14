use bevy::prelude::*;

/// Tracks a set of registration closures that will be executed when setting up the Bevy app.
///
/// This small utility exists to slightly improve the ergonomics of plugins that are meant to deal
/// with generic types and register generic systems. The animator and carousel plugins both use
/// generic systems and therefore need to know (by having the caller _register_) the particular
/// types that will be used. There's a non-trivial amount of boilerplate to go along with this
/// registration and it's always exactly the same except for the 1-line function that acts on the
/// Bevy [`App`].
pub struct Registry {
    registrations: Vec<Box<dyn Registration>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    pub fn add<R: Fn(&mut App) + Send + Sync + 'static>(&mut self, register: R) {
        self.registrations
            .push(Box::new(RegistrationImpl::new(register)));
    }

    pub fn apply(&self, app: &mut App) {
        for registration in &self.registrations {
            registration.apply(app);
        }
    }
}

trait Registration: Send + Sync {
    fn apply(&self, app: &mut App);
}

struct RegistrationImpl<R: Fn(&mut App)> {
    register: R,
}

impl<R: Fn(&mut App)> RegistrationImpl<R> {
    fn new(register: R) -> Self {
        Self { register }
    }
}

impl<R: Fn(&mut App) + Send + Sync> Registration for RegistrationImpl<R> {
    fn apply(&self, app: &mut App) {
        (self.register)(app);
    }
}
