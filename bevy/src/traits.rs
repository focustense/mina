/// Convenience traits for using animation types in Bevy ECS.

use std::hash::Hash;
use dyn_clone::{clone_trait_object, DynClone};
use mina::prelude::*;

/// Represents a [Timeline] that is safe to store (boxed) in a
/// [Component](bevy::ecs::component::Component).
///
/// Every timeline built with a [TimelineBuilder] or macro equivalent, including [MergedTimeline],
/// implicitly implements this trait.
pub trait SafeTimeline: Timeline + DynClone + Send + Sync + 'static {}

clone_trait_object!(<T> SafeTimeline<Target = T>);

impl<T> SafeTimeline for T where T : Timeline + DynClone + Send + Sync + 'static {}

/// Trait for a type that can be used as a key in an
/// [AnimationSelector](crate::selection::AnimationSelector).
///
/// Explicit implementations are usually not necessary. Primitives and strings implicitly implement
/// this, and thread-safe enums only need to implement [`Clone`], [`Default`], [`Eq`] and [`Hash`]
/// in order to receive the same implicit implementation.
pub trait AnimationKey: Clone + Default + Eq + Hash + Send + Sync + 'static {}

impl<T> AnimationKey for T where T: Clone + Default + Eq + Hash + Send + Sync + 'static {}
