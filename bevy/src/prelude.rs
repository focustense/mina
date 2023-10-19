//! Common types used for Mina animations in Bevy apps.

pub use crate::{
    animator::{AnimationState, AnimationStateChanged, Animator},
    selection::{
        AnimationChain, AnimationChainBuilder, AnimationSelector, AnimationSelectorBuilder,
    },
    traits::*,
    AnimationPlugin,
};
