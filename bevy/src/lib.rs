//! An ECS-friendly [Mina](https://crates.io/crates/mina) plugin for
//! [bevy](https://github.com/bevyengine/bevy), which enables animations and transitions to be set
//! up as components.
//!
//! # Getting Started
//!
//! The simplest use of `bevy_mina` is to add a single [AnimationPlugin] and [Animator] component.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_mina::prelude::*;
//! use mina::prelude::*;
//!
//! #[derive(Animate)]
//! #[animate(remote = "Transform")]
//! struct TransformProxy {
//!     translation: Vec3,
//!     rotation: Quat,
//!     scale: Vec3,
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, AnimationPlugin::<Transform>::new()))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(Camera2dBundle::default());
//!
//!     commands.spawn((
//!         SpriteBundle {
//!             texture: asset_server.load("images/example.png"),
//!             ..default()
//!         },
//!         Animator::<Transform>::with_timeline(timeline! {
//!             TransformProxy 1s reverse infinite
//!                 from { translation: Vec3::new(-50., 0., 0.) }
//!                 to { translation: Vec3::new(50., 0., 0.) }
//!         }),
//!     ));
//! }
//! ```
//!
//! # State-based Animation
//!
//! In "classic" Mina, we can use the [animator](mina::animator) macro to create a
//! [StateAnimator](mina::StateAnimator) that accepts some state (key) and automatically transitions
//! to the timeline registered for that state. However, this is not very ECS-friendly. It will not
//! work with the [Animator] component, cannot be implemented as a [Component] itself, and still
//! requires something external to drive the timing.
//!
//! Enter [AnimationSelector](crate::selection::AnimationSelector), the ECS replacement for
//! [StateAnimator](mina::StateAnimator). The selector is an add-on `Component` that can be attached
//! to the same entity as the `Animator`, and is set up with a map of keys to timelines:
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_mina::prelude::*;
//! use mina::prelude::*;
//!
//! // Note: We could also set this up on a `Transform`, like the previous example.
//! #[derive(Animate, Component, Default)]
//! struct PauseOverlay {
//!     alpha: f32,
//! }
//!
//! #[derive(Clone, Default, Eq, Hash, PartialEq)]
//! enum GameState {
//!     #[default] Playing,
//!     Paused,
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((
//!             DefaultPlugins,
//!             AnimationPlugin::<Transform>::new()
//!                 .add_selection_key::<GameState>()
//!         ))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(Camera2dBundle::default());
//!
//!     commands.spawn((
//!         SpriteBundle {
//!             texture: asset_server.load("images/paused.png"),
//!             ..default()
//!         },
//!         PauseOverlay::default(),
//!         Animator::<Transform>::new(),
//!         AnimationSelectorBuilder::new()
//!             .add(
//!                 GameState::Paused,
//!                 timeline!(PauseOverlay 0.5s from { alpha: 0.0 } to { alpha: 1.0 })
//!             )
//!             .add(
//!                 GameState::Playing,
//!                 timeline!(PauseOverlay 0.1s from {alpha: 1.0 } to { alpha: 0.0 })
//!             )
//!             .build()
//!     ));
//! }
//! ```
//!
//! This will perform the same blending as a `StateAnimator`, so in the above example, if the game
//! is unpaused while the overlay is still semi-transparent, then it will animate back smoothly from
//! semi-transparent to fully-transparent.
//!
//! It is also possible, though typically not advisable, to control an animation by using multiple
//! states, i.e. by adding multiple `AnimationSelector` components (which is allowed, because the
//! generic arguments are different) and chaining multiple calls to `add_selection_key`. If this is
//! done, then the timeline will be swapped when _any_ of the controlling states change.
//!
//! States can also be configured to auto-transition when animation ends; for more information,
//! refer to the [AnimationChain](crate::selection::AnimationChain) documentation.

use crate::animator::{animate, AnimationState, AnimationStateChanged, Animator};
use crate::selection::{chain_animations, select_animation};
use crate::traits::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub mod prelude;

mod animator;
mod selection;
mod traits;

/// Enables animation of a specific [Component] type.
///
/// When the animated component, `T`, and an [`Animator<T>`] are both added to an entity, the
/// properties of the `T` component will follow the animator's [Timeline](mina::Timeline).
///
/// A separate instance of the plugin must be added for each component type being animated.
#[derive(Default)]
pub struct AnimationPlugin<T: Component> {
    phantom: PhantomData<T>,
    registrations: Vec<Box<dyn Registration>>,
}

impl<T: Component> AnimationPlugin<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
            registrations: Vec::new(),
        }
    }
}

impl<T: Component> AnimationPlugin<T> {
    /// Enables state-driven animation of the [Component] type `T` using an animation key `K`.
    ///
    /// Once a key is registered, an [`AnimationSelector<K, T>`] can be added to the same entity as
    /// the [`Animator<T>`], and updating the selector's [AnimationSelector::timeline_key] will
    /// transition (blend) into the new timeline.
    pub fn add_selection_key<K: AnimationKey>(mut self) -> Self {
        self.registrations
            .push(Box::new(RegistrationImpl::new(|app| {
                app.add_systems(
                    Update,
                    (chain_animations::<K, T>, select_animation::<K, T>).before(animate::<T>),
                );
            })));
        self
    }
}

impl<T: Component> Plugin for AnimationPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<AnimationStateChanged>()
            .add_systems(Update, animate::<T>);
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
