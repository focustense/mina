use crate::arrow_button::{ArrowButtonBundle, ArrowButtonPlugin, ArrowDirection};
use crate::carousel::{Carousel, CarouselPlugin};
use crate::characters::{Character, CharacterSprites};
use bevy::{prelude::*, time::common_conditions::on_timer, winit::WinitSettings};
use bevy_mod_picking::prelude::*;
use bevy_vector_shapes::prelude::*;
use enum_map::enum_map;
use mina::prelude::*;
use std::cmp::Ordering;
use std::time::Duration;

mod animator_plugin;
mod arrow_button;
mod carousel;
mod characters;
mod registry;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(Shape2dPlugin::default())
        .add_plugin(ArrowButtonPlugin)
        .add_plugin(CarouselPlugin::new().add_timeline::<CarouselItemTimeline>())
        .insert_resource(WinitSettings::game())
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .init_resource::<CharacterSprites>()
        .add_event::<NextCharacter>()
        .add_event::<PreviousCharacter>()
        .add_startup_system(setup)
        .add_system(character_animation.run_if(on_timer(Duration::from_millis(80))))
        .add_systems((character_carousel, character_navigate))
        .run();
}

#[derive(Animate, Clone, Component, Debug, Default)]
struct CarouselItem {
    alpha: f32,
    x: f32,
}

struct NextCharacter;
impl From<ListenedEvent<Click>> for NextCharacter {
    fn from(_: ListenedEvent<Click>) -> Self {
        NextCharacter
    }
}

struct PreviousCharacter;
impl From<ListenedEvent<Click>> for PreviousCharacter {
    fn from(_: ListenedEvent<Click>) -> Self {
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
        OnPointer::<Click>::send_event::<PreviousCharacter>(),
    ));
    commands.spawn((
        ArrowButtonBundle::new(ArrowDirection::Right, 300.0, 25.0),
        OnPointer::<Click>::send_event::<NextCharacter>(),
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
                .alpha(0.0)
                .easing(Easing::InQuint),
        )
        .keyframe(CarouselItem::keyframe(0.05).alpha(0.0))
        .keyframe(
            CarouselItem::keyframe(0.5)
                .x(0.0)
                .alpha(1.0)
                .easing(Easing::OutQuint),
        )
        .keyframe(CarouselItem::keyframe(0.95).alpha(0.0))
        .keyframe(CarouselItem::keyframe(1.0).x(width / 2.0).alpha(0.0))
        .build();
    Carousel::new(timeline, move_duration_seconds)
}

fn spawn_title(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                size: Size::width(Val::Percent(100.0)),
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
                    position_type: PositionType::Relative,
                    position: UiRect {
                        bottom: Val::Px(100.0),
                        ..default()
                    },
                    ..default()
                })
                .with_text_alignment(TextAlignment::Center),
            );
        });
}