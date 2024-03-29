use crate::arrow_button::{ArrowButtonBundle, ArrowButtonPlugin, ArrowDirection};
use crate::carousel::{Carousel, CarouselPlugin};
use crate::characters::{Character, CharacterPlugin, CharacterSprites};
use bevy::{prelude::*, time::common_conditions::on_timer, winit::WinitSettings};
use bevy_mina::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_vector_shapes::prelude::*;
use enum_map::enum_map;
use mina::prelude::*;
use std::cmp::Ordering;
use std::time::Duration;

mod arrow_button;
mod carousel;
mod characters;
mod interaction;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            DefaultPickingPlugins,
            Shape2dPlugin::default(),
            ArrowButtonPlugin,
            CharacterPlugin,
            CarouselPlugin::<CarouselItemTimeline>::new(),
            AnimationPlugin::<Transform>::new(),
        ))
        .insert_resource(WinitSettings::game())
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .add_event::<NextCharacter>()
        .add_event::<PreviousCharacter>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (character_carousel, character_navigate),
                character_animation.run_if(on_timer(Duration::from_millis(80))),
            ),
        )
        .run();
}

#[derive(Animate)]
#[animate(remote = "Transform")]
struct TransformProxy {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Animate, Clone, Component, Debug, Default)]
struct CarouselItem {
    alpha: f32,
    scale: f32,
    x: f32,
}

#[derive(Event)]
struct NextCharacter;
impl From<ListenerInput<Pointer<Click>>> for NextCharacter {
    fn from(_: ListenerInput<Pointer<Click>>) -> Self {
        NextCharacter
    }
}

#[derive(Event)]
struct PreviousCharacter;
impl From<ListenerInput<Pointer<Click>>> for PreviousCharacter {
    fn from(_: ListenerInput<Pointer<Click>>) -> Self {
        PreviousCharacter
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    character_sprites: Res<CharacterSprites>,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_title(&mut commands, asset_server);

    commands.spawn((
        ArrowButtonBundle::new(ArrowDirection::Left, -300.0, 25.0),
        On::<Pointer<Click>>::send_event::<PreviousCharacter>(),
    ));
    commands.spawn((
        ArrowButtonBundle::new(ArrowDirection::Right, 300.0, 25.0),
        On::<Pointer<Click>>::send_event::<NextCharacter>(),
    ));

    let mut spawn_character = |character| {
        commands
            .spawn((character_sprites.create(character), CarouselItem::default()))
            .id()
    };
    let available_characters = enum_map! {
        Character::Caveman => spawn_character(Character::Caveman),
        Character::Eggshell => spawn_character(Character::Eggshell),
        Character::Girl => spawn_character(Character::Girl),
        Character::Lion => spawn_character(Character::Lion),
    };
    let carousel_id = commands
        .spawn((create_carousel(400.0, 0.2), SpatialBundle::default()))
        .id();
    for (_, character_id) in &available_characters {
        commands.entity(carousel_id).add_child(*character_id);
    }
}

fn character_animation(
    carousel: Query<&Carousel<CarouselItemTimeline>>,
    mut sprites: Query<(Entity, &mut TextureAtlasSprite), With<Character>>,
) {
    let carousel = carousel.single();
    for (entity, mut sprite) in sprites.iter_mut() {
        if carousel.selected_entity() != Some(entity) {
            continue;
        }
        sprite.index = (sprite.index + 1) % 5;
    }
}

fn character_carousel(mut items: Query<(&CarouselItem, &mut Transform, &mut TextureAtlasSprite)>) {
    for (item, mut transform, mut sprite) in items.iter_mut() {
        transform.scale = Vec3::splat(item.scale);
        transform.translation.x = item.x;
        sprite.color = Color::rgba(1.0, 1.0, 1.0, item.alpha);
    }
}

fn character_navigate(
    mut carousel: Query<&mut Carousel<CarouselItemTimeline>>,
    mut prev_events: EventReader<PreviousCharacter>,
    mut next_events: EventReader<NextCharacter>,
) {
    let mut carousel = carousel.single_mut();
    let offset = next_events.iter().count() as i32 - prev_events.iter().count() as i32;
    match offset.cmp(&0) {
        Ordering::Greater => carousel.move_next(),
        Ordering::Less => carousel.move_previous(),
        _ => {}
    }
}

fn create_carousel(width: f32, move_duration_seconds: f32) -> Carousel<CarouselItemTimeline> {
    let timeline = CarouselItem::timeline()
        .duration_seconds(1.0)
        .keyframe(
            CarouselItem::keyframe(0.0)
                .x(-width / 2.0)
                .scale(0.75)
                .alpha(0.0)
                .easing(Easing::InQuint),
        )
        .keyframe(CarouselItem::keyframe(0.05).alpha(0.0))
        .keyframe(
            CarouselItem::keyframe(0.5)
                .x(0.0)
                .scale(1.0)
                .alpha(1.0)
                .easing(Easing::OutQuint),
        )
        .keyframe(CarouselItem::keyframe(0.95).alpha(0.0))
        .keyframe(
            CarouselItem::keyframe(1.0)
                .x(width / 2.0)
                .scale(0.75)
                .alpha(0.0),
        )
        .build();
    Carousel::new(timeline, move_duration_seconds)
}

fn spawn_title(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                width: Val::Percent(100.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    "Select Character",
                    TextStyle {
                        font: asset_server.load("LuckiestGuy-Regular.ttf"),
                        font_size: 48.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                )
                .with_style(Style {
                    bottom: Val::Px(100.0),
                    position_type: PositionType::Relative,
                    ..default()
                })
                .with_text_alignment(TextAlignment::Center),
            );
        });

    let crystal = asset_server.load("crystal.png");
    commands.spawn((
        SpriteBundle {
            texture: crystal,
            transform: Transform {
                translation: Vec3::new(-240., 110., 0.),
                scale: Vec3::splat(0.1),
                ..default()
            },
            ..default()
        },
        Animator::<Transform>::with_timeline(timeline! {
            TransformProxy 2s reverse infinite Easing::OutBack
                from { scale: Vec3::splat(0.1) }
                to { scale: Vec3::splat(0.15) }
        }),
    ));
}
