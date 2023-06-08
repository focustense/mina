use mina::prelude::*;
use mina_core::animator::{StateAnimator, StateAnimatorBuilder};

#[derive(Clone, Debug, Default, Eq, PartialEq, State)]
enum Interaction {
    #[default] A,
    B,
}

#[derive(Animate, Clone, Debug, Default, PartialEq)]
struct Style {
    x: u8,
    y: u8,
}

impl Style {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

mod using_builder {
    use super::*;

    #[test]
    fn animates_initial_state_from_initial_values() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .from_values(Style::new(25, 123))
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(50))
                .keyframe(Style::keyframe(1.0).x(100)))
            // Not used, but we'll make sure it doesn't interfere somehow.
            .on(Interaction::B, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(100))
                .keyframe(Style::keyframe(1.0).x(80)))
            .build();

        let frame_values = run_animator(&mut animator, 1.0, 7.0);

        assert_eq!(frame_values, &[
            Style { x: 25, y: 123 },
            Style { x: 40, y: 123 },
            Style { x: 55, y: 123 },
            Style { x: 70, y: 123 },
            Style { x: 85, y: 123 },
            Style { x: 100, y: 123 },
            // Animation ended but state has not changed
            Style { x: 100, y: 123 },
            Style { x: 100, y: 123 },
        ]);
    }

    #[test]
    fn when_state_changed_then_animates_from_previous_values() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(0))
                .keyframe(Style::keyframe(1.0).x(100)))
            .on(Interaction::B, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(100).y(50))
                .keyframe(Style::keyframe(1.0).x(20).y(80)))
            .build();

        let frame_values_a = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::B);
        let frame_values_b = run_animator(&mut animator, 1.0, 6.0);

        assert_eq!(frame_values_a, &[
            Style { x: 0, y: 0 },  // 0s
            Style { x: 20, y: 0 }, // 1s
            Style { x: 40, y: 0 }, // 2s
            Style { x: 60, y: 0 }, // 3s
        ]);
        assert_eq!(frame_values_b, &[
            Style { x: 60, y: 0 },  // 3s (repeated)
            Style { x: 52, y: 16 }, // 4s
            Style { x: 44, y: 32 }, // 5s
            Style { x: 36, y: 48 }, // 6s
            Style { x: 28, y: 64 }, // 7s
            Style { x: 20, y: 80 }, // 8s (ended)
            Style { x: 20, y: 80 }, // 9s
        ]);
    }
}

mod using_macro {
    use super::*;

    #[test]
    fn animates_initial_state_from_initial_values() {
        let mut animator = animator!(Style {
            default(Interaction::A, { x: 25, y: 123 }),
            Interaction::A => 5s from { x: 50 } to { x: 100 },
            Interaction::B => 5s from { x: 100 } to { x: 80 }
        });

        let frame_values = run_animator(&mut animator, 1.0, 7.0);

        assert_eq!(frame_values, &[
            Style { x: 25, y: 123 },
            Style { x: 40, y: 123 },
            Style { x: 55, y: 123 },
            Style { x: 70, y: 123 },
            Style { x: 85, y: 123 },
            Style { x: 100, y: 123 },
            // Animation ended but state has not changed
            Style { x: 100, y: 123 },
            Style { x: 100, y: 123 },
        ]);
    }

    #[test]
    fn when_state_changed_then_animates_from_previous_values() {
        let mut animator = animator!(Style {
            default(Interaction::A),
            Interaction::A => 5.0s from default to { x: 100 },
            Interaction::B => 5.0s
                from { x: 100, y: 50 }
                40% { x: 30 }
                to { x: 20, y: 80 },
        });

        let frame_values_a = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::B);
        let frame_values_b = run_animator(&mut animator, 1.0, 6.0);

        assert_eq!(frame_values_a, &[
            Style { x: 0, y: 0 },  // 0s
            Style { x: 20, y: 0 }, // 1s
            Style { x: 40, y: 0 }, // 2s
            Style { x: 60, y: 0 }, // 3s
        ]);
        assert_eq!(frame_values_b, &[
            Style { x: 60, y: 0 },  // 3s (repeated)
            Style { x: 45, y: 16 }, // 4s
            Style { x: 30, y: 32 }, // 5s
            Style { x: 26, y: 48 }, // 6s
            Style { x: 23, y: 64 }, // 7s
            Style { x: 20, y: 80 }, // 8s (ended)
            Style { x: 20, y: 80 }, // 9s
        ]);
    }
}

fn run_animator(
    animator: &mut impl StateAnimator<State = Interaction, Values = Style>,
    time_step: f32,
    duration: f32,
) -> Vec<Style> {
    let mut results = Vec::new();
    results.push(animator.current_values().clone());
    let count = (duration / time_step) as u32;
    for _i in 0..count {
        animator.advance(time_step);
        results.push(animator.current_values().clone());
    }
    results
}
