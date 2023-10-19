use crate::interaction::{Interaction, PointerInteractionBundle};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mina::prelude::*;
use bevy_mod_picking::{
    backend::{HitData, PointerHits},
    picking_core::PickSet,
    prelude::*,
};
use bevy_vector_shapes::prelude::*;
use mina::prelude::*;
use std::f32::consts::PI;

/// Components and systems for the animated arrow button.
///
/// Fades in a translucent background on hover and displays a pulsing, rotating focus ring. On
/// click, performs an additional "ratchet" animation with the border and pulses the color.
///
/// This collection represents the most conventional usage in a Bevy app. It relies on regular 2D
/// components and the Picking mod instead of Bevy UI because it's hard to do anything very
/// intricate in Bevy UI without the use of custom shaders, nine-patches, etc. Using the Bevy Shapes
/// crate means we can represent everything in code. This does require a custom picking backend
/// since there isn't anything like a sprite or mesh to check bounds or raycast.
///
/// In terms of the animations, it is using the [`animate`] macro and [`StateAnimator`] trait "as
/// intended", simply allowing the animator to control the style as states change, and reporting
/// state changes to the animator through the picking components.
pub struct ArrowButtonPlugin;

impl Plugin for ArrowButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AnimationPlugin::<ArrowButton>::new().add_selection_key::<Interaction>())
            .add_systems(
                Update,
                (arrow_button_picking.in_set(PickSet::Backend), draw_arrows),
            );
    }
}

#[derive(Animate, Clone, Component)]
pub struct ArrowButton {
    #[animate]
    background_alpha: f32,
    #[animate]
    background_lightness: f32,
    direction: ArrowDirection,
    #[animate]
    focus_ring_alpha: f32,
    #[animate]
    focus_ring_rotation: f32,
    size: f32,
}

// Required for animator. Can we do anything to eliminate the requirement?
impl Default for ArrowButton {
    fn default() -> Self {
        Self {
            direction: ArrowDirection::Right,
            background_alpha: 0.0,
            background_lightness: 0.0,
            focus_ring_alpha: 0.0,
            focus_ring_rotation: 0.0,
            size: 0.0,
        }
    }
}

impl ArrowButton {
    pub fn new(direction: ArrowDirection, size: f32) -> Self {
        Self {
            direction,
            size,
            ..default()
        }
    }

    fn selection_radius(&self) -> f32 {
        self.size * 2.0
    }
}

#[derive(Clone)]
pub enum ArrowDirection {
    Left,
    Right,
}

#[derive(Bundle)]
pub struct ArrowButtonBundle {
    animator: Animator<ArrowButton>,
    animation_selector: AnimationSelector<Interaction, ArrowButton>,
    button: ArrowButton,
    pointer_interaction: PointerInteractionBundle,
    spatial: SpatialBundle,
}

impl ArrowButtonBundle {
    pub fn new(direction: ArrowDirection, x: f32, size: f32) -> Self {
        Self {
            spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3::new(
                x, 0.0, 0.0,
            ))),
            animator: Animator::new(),
            animation_selector: AnimationSelectorBuilder::new()
                .add(
                    Interaction::None,
                    timeline!(ArrowButton [
                        0.1s Easing::OutCubic to { background_lightness: 0.0 },
                        0.5s Easing::OutQuad to { background_alpha: 0.0, focus_ring_alpha: 0.0 },
                        0.1s after 0.5s to { focus_ring_rotation: 0.0 }
                    ]),
                )
                .add(
                    Interaction::Over,
                    timeline!(ArrowButton [
                        0.1s Easing::OutCubic to { background_lightness: 0.0 },
                        1s Easing::In to { background_alpha: 1.0 },
                        3s infinite Easing::In
                            from { focus_ring_alpha: 0.05 }
                            40% { focus_ring_alpha: 1.0 }
                            70% { focus_ring_alpha: 1.0 }
                            to { focus_ring_alpha: 0.05 },
                        10s infinite 1% { focus_ring_rotation: 0.0 } to { focus_ring_rotation: PI },
                    ]),
                )
                .add(
                    Interaction::Down,
                    timeline!(ArrowButton [
                        0.5s Easing::OutCubic to {
                            background_alpha: 0.8,
                            focus_ring_alpha: 1.0,
                            focus_ring_rotation: 0.0
                        },
                        2s Easing::InOutCubic to { background_lightness: 0.15 } reverse infinite,
                    ]),
                )
                .build(),
            button: ArrowButton::new(direction, size),
            pointer_interaction: PointerInteractionBundle::new::<ArrowButton>(),
        }
    }
}

fn arrow_button_picking(
    arrow_buttons: Query<(Entity, &ArrowButton, &GlobalTransform, &ComputedVisibility)>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut output: EventWriter<PointerHits>,
) {
    // Normally we should sort by Z order. In our toy example here, they'll never overlap.
    for (pointer, location) in pointers.iter().filter_map(|(pointer, pointer_location)| {
        pointer_location.location().map(|loc| (pointer, loc))
    }) {
        let (cam_entity, camera, cam_transform) = cameras
            .iter()
            .find(|(_, camera, _)| {
                camera
                    .target
                    .normalize(Some(primary_window.single()))
                    .unwrap()
                    == location.target
            })
            .unwrap_or_else(|| panic!("No camera found associated with pointer {:?}", pointer));
        let Some(cursor_pos_world) = camera.viewport_to_world_2d(cam_transform, location.position)
        else {
            continue;
        };
        let picks = arrow_buttons
            .iter()
            .filter_map(|(entity, button, transform, visibility)| {
                if !visibility.is_visible() {
                    return None;
                }
                let position = transform.translation().truncate();
                let distance = position.distance(cursor_pos_world);
                if distance <= button.selection_radius() {
                    Some((
                        entity,
                        HitData {
                            camera: cam_entity,
                            depth: 0.0,
                            position: None,
                            normal: None,
                        },
                    ))
                } else {
                    None
                }
            });
        output.send(PointerHits {
            pointer: *pointer,
            picks: picks.collect(),
            order: 0.,
        });
    }
}

fn draw_arrows(arrow_buttons: Query<(&ArrowButton, &Transform)>, mut painter: ShapePainter) {
    const DASH_SEGMENTS: u32 = 16;

    let dash_angle = PI / DASH_SEGMENTS as f32;
    for (arrow_button, transform) in &arrow_buttons {
        painter.transform = *transform;
        painter.color = lighten(
            Color::rgba(0.05, 0.15, 0.2, arrow_button.background_alpha),
            arrow_button.background_lightness,
        );
        painter.hollow = false;
        painter.circle(arrow_button.selection_radius() - 4.0);

        painter.reset();
        painter.transform = *transform;
        painter.color = Color::SEA_GREEN;
        painter.thickness = 4.0;
        let rotation = match arrow_button.direction {
            ArrowDirection::Left => PI / 2.0,
            ArrowDirection::Right => -PI / 2.0,
        };
        painter.rotate_z(rotation);
        painter.ngon(3.0, arrow_button.size);

        painter.reset();
        painter.transform = *transform;
        painter.color = Color::rgba(0.8, 0.8, 0.8, arrow_button.focus_ring_alpha);
        painter.cap = Cap::None;
        painter.thickness = 4.0;
        painter.hollow = true;
        painter.rotate_z(arrow_button.focus_ring_rotation);
        let mut arc_angle = 0.0;
        for _i in 0..DASH_SEGMENTS {
            let next_angle = arc_angle + dash_angle;
            painter.arc(arrow_button.selection_radius(), arc_angle, next_angle);
            arc_angle = next_angle + dash_angle;
        }
    }
}

fn lighten(color: Color, added_lightness: f32) -> Color {
    let Color::Lcha {
        lightness,
        chroma,
        hue,
        alpha,
    } = color.as_lcha()
    else {
        return color;
    };
    Color::Lcha {
        lightness: lightness + added_lightness,
        chroma,
        hue,
        alpha,
    }
}
