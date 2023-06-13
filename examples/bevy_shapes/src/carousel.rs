use crate::registry::Registry;
use bevy::prelude::*;
use mina::prelude::*;

pub struct CarouselPlugin {
    registry: Registry,
}

impl CarouselPlugin {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
        }
    }

    pub fn add_timeline<T>(mut self) -> Self
    where
        T: Timeline + Send + Sync + 'static,
        T::Target: Clone + Component + Send + Sync,
    {
        self.registry.add(|app| {
            app.add_system(update_carousels::<T>);
        });
        self
    }
}

impl Plugin for CarouselPlugin {
    fn build(&self, app: &mut App) {
        self.registry.apply(app);
    }
}

#[derive(Component)]
pub struct Carousel<T>
where
    T: Timeline,
    T::Target: Component,
{
    child_count: usize,
    move_duration_seconds: f32,
    // Trying to determine if we hit or went past the target in modulo arithmetic is a PITA - not
    // impossible, but not really worth the savings of 4 bytes.
    move_time_remaining: f32,
    move_transient_position: f32,
    move_velocity: f32,
    selected_entity: Option<Entity>,
    pub selected_index: usize,
    target_index: usize, // What position are we animating to right now?
    timeline: T,
}

impl<T> Carousel<T>
where
    T: Timeline,
    T::Target: Component,
{
    pub fn new(timeline: T, move_duration_seconds: f32) -> Self {
        Self {
            timeline,
            child_count: 0,
            move_duration_seconds,
            move_time_remaining: 0.0,
            move_transient_position: 0.0,
            move_velocity: 0.0,
            selected_entity: None,
            selected_index: 0,
            target_index: 0,
        }
    }

    pub fn move_next(&mut self) {
        if self.child_count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.child_count;
    }

    pub fn move_previous(&mut self) {
        if self.child_count == 0 {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.child_count - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected_entity
    }
}

fn update_carousels<T>(
    time: Res<Time>,
    mut carousels: Query<(&mut Carousel<T>, &Children)>,
    mut targets: Query<&mut T::Target>,
) where
    T: Timeline + Send + Sync + 'static,
    T::Target: Component,
{
    let delta_time = time.delta_seconds();
    for (mut carousel, children) in carousels.iter_mut() {
        if carousel.child_count != children.len() {
            carousel.child_count = children.len();
        }
        let selected_entity = children.get(carousel.selected_index);
        if selected_entity != carousel.selected_entity.as_ref() {
            carousel.selected_entity = selected_entity.copied();
        }

        let interval_count = if carousel.child_count % 2 == 0 {
            carousel.child_count
        } else {
            carousel.child_count - 1
        } as f32;
        if children.len() > 0 && carousel.target_index != carousel.selected_index {
            carousel.target_index = carousel.selected_index;
            if carousel.move_duration_seconds > 0.0 {
                // Choose the shortest distance to animate, regardless of which direction was
                // originally used to move the index.
                let df = (carousel.selected_index as f32 + interval_count
                    - carousel.move_transient_position)
                    % interval_count;
                let dr = -((carousel.move_transient_position + interval_count
                    - carousel.selected_index as f32)
                    % interval_count);
                let distance = if df.abs() < dr.abs() { df } else { dr };
                carousel.move_time_remaining = carousel.move_duration_seconds;
                carousel.move_velocity = distance / carousel.move_duration_seconds;
            } else {
                carousel.move_time_remaining = 0.0;
                carousel.move_transient_position = carousel.target_index as f32;
            }
        }

        if carousel.move_velocity != 0.0 && carousel.move_time_remaining > 0.0 {
            let move_distance = carousel.move_velocity * delta_time;
            carousel.move_transient_position =
                (carousel.move_transient_position + move_distance).rem_euclid(interval_count);
            carousel.move_time_remaining -= delta_time;
            if carousel.move_time_remaining < 0.0 {
                carousel.move_transient_position = carousel.selected_index as f32;
            }
        }

        if carousel.is_changed() {
            // We want symmetry, so if the interval count is odd, add a fake slot to turn it even.
            let mid_index = interval_count / 2.0;
            for (child_index, child) in children.iter().enumerate() {
                let Ok(mut target) = targets.get_mut(*child) else { continue; };
                // First orient to the selection being in the middle position. For example, if the
                // selection is on item #2 then item #0 has an offset of -2, i.e. it is two positions to
                // the left of center.
                // Then add the midpoint index, so we get a value between 0 and the total count, which
                // is easy to normalize between the timeline range 0..1.
                // To avoid overflow, we can add a full interval before taking remainder.
                let slot_position = (interval_count + mid_index + child_index as f32
                    - carousel.move_transient_position)
                    % interval_count;
                let normalized_position = slot_position as f32 / interval_count as f32;
                carousel.timeline.update(&mut target, normalized_position);
            }
        }
    }
}
