// Utilities to help with setup and registration of generic systems.

use bevy::prelude::*;

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
