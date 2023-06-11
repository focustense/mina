use crate::animator_plugin::{Animator, AnimatorBundle, AnimatorPlugin, Interaction};
use bevy::{prelude::*, window::PrimaryWindow, winit::WinitSettings};
use bevy_mod_picking::{
    backend::{HitData, PointerHits},
    picking_core::PickSet,
    prelude::*,
};
use bevy_vector_shapes::prelude::*;
use mina::prelude::*;
use std::f32::consts::PI;

mod animator_plugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(Shape2dPlugin::default())
        .add_plugin(AnimatorPlugin::new().add_timeline::<ArrowButtonTimeline>())
        .insert_resource(WinitSettings::game())
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .add_startup_system(setup)
        .add_system(arrow_button_picking.in_set(PickSet::Backend))
        .add_system(ui)
        .run();
}

const DASH_SEGMENTS: u32 = 16;

#[derive(Animate, Clone)]
struct ArrowButton {
    #[animate] background_alpha: f32,
    direction: ArrowDirection,
    #[animate] focus_ring_alpha: f32,
    #[animate] focus_ring_rotation: f32,
    size: f32,
}

// Required for animator. Can we do anything to eliminate the requirement?
impl Default for ArrowButton {
    fn default() -> Self {
        Self {
            direction: ArrowDirection::Right,
            background_alpha: 0.0,
            focus_ring_alpha: 0.0,
            focus_ring_rotation: 0.0,
            size: 0.0,
        }
    }
}

impl ArrowButton {
    fn new(direction: ArrowDirection, size: f32) -> Self {
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
enum ArrowDirection {
    Left,
    Right,
}

#[derive(Bundle)]
struct ArrowButtonBundle {
    animator: AnimatorBundle<ArrowButtonTimeline>,
    spatial: SpatialBundle,
}

impl ArrowButtonBundle {
    pub fn new(direction: ArrowDirection, x: f32, size: f32) -> Self {
        let button = ArrowButton::new(direction, size);
        Self {
            spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3::new(
                x, 0.0, 0.0,
            ))),
            animator: AnimatorBundle::new(animator!(ArrowButton {
                default(Interaction::None, button),
                Interaction::None => [
                    0.5s Easing::OutQuad to { background_alpha: 0.0, focus_ring_alpha: 0.0 },
                    0.1s after 0.5s to { focus_ring_rotation: 0.0 }
                ],
                Interaction::Over => [
                    1s Easing::In to { background_alpha: 1.0 },
                    3s infinite Easing::In
                        from { focus_ring_alpha: 0.05 }
                        40% { focus_ring_alpha: 1.0 }
                        70% { focus_ring_alpha: 1.0 }
                        to { focus_ring_alpha: 0.05 },
                    10s infinite 1% { focus_ring_rotation: 0.0 } to { focus_ring_rotation: PI },
                ],
                Interaction::Down => 0.5s Easing::OutCubic to {
                    focus_ring_alpha: 1.0,
                    focus_ring_rotation: 0.0
                }
            })),
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(ArrowButtonBundle::new(ArrowDirection::Left, -200.0, 25.0));
    commands.spawn(ArrowButtonBundle::new(ArrowDirection::Right, 200.0, 25.0));
}

fn ui(
    arrow_buttons: Query<(&Animator<ArrowButtonTimeline>, &Transform)>,
    mut painter: ShapePainter,
) {
    let dash_angle = PI / DASH_SEGMENTS as f32;
    for (animator, transform) in &arrow_buttons {
        let arrow_button = animator.current_values();

        painter.transform = *transform;
        painter.color = Color::rgba(0.05, 0.15, 0.2, arrow_button.background_alpha);
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

fn arrow_button_picking(
    arrow_buttons: Query<(
        Entity,
        &Animator<ArrowButtonTimeline>,
        &GlobalTransform,
        &ComputedVisibility,
    )>,
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
        let Some(cursor_pos_world) = camera.viewport_to_world_2d(cam_transform, location.position) else { continue; };
        let picks = arrow_buttons
            .iter()
            .filter_map(|(entity, animator, transform, visibility)| {
                if !visibility.is_visible() {
                    return None;
                }
                let position = transform.translation().truncate();
                let distance = position.distance(cursor_pos_world);
                let button = animator.current_values();
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
            order: 0,
        });
    }
}
