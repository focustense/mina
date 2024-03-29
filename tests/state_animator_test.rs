use mina::prelude::*;

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

    #[test]
    fn when_state_changed_and_repeating_then_loops_from_non_blended_values() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(0))
                .keyframe(Style::keyframe(1.0).x(100)))
            .on(Interaction::B, Style::timeline()
                .duration_seconds(5.0)
                .repeat(Repeat::Infinite)
                .keyframe(Style::keyframe(0.0).x(120).y(50))
                .keyframe(Style::keyframe(1.0).x(20).y(80)))
            .build();

        let frame_values_a = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::B);
        let frame_values_b = run_animator(&mut animator, 1.0, 11.0);

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
            Style { x: 20, y: 80 }, // 8s
            Style { x: 100, y: 56 }, // 9s
            Style { x: 80, y: 62 }, // 10s
            Style { x: 60, y: 68 }, // 11s
            Style { x: 40, y: 74 }, // 12s
            Style { x: 20, y: 80 }, // 13s
            Style { x: 100, y: 56 }, // 14s
        ]);
    }

    #[test]
    fn when_state_changed_and_reversing_then_returns_to_non_blended_values() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(0.0).x(0))
                .keyframe(Style::keyframe(1.0).x(100)))
            .on(Interaction::B, Style::timeline()
                .duration_seconds(10.0)
                .reverse(true)
                .keyframe(Style::keyframe(0.0).x(120).y(50))
                .keyframe(Style::keyframe(1.0).x(20).y(80)))
            .build();

        let frame_values_a = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::B);
        let frame_values_b = run_animator(&mut animator, 1.0, 11.0);

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
            Style { x: 20, y: 80 }, // 8s
            Style { x: 40, y: 74 }, // 9s
            Style { x: 60, y: 68 }, // 10s
            Style { x: 80, y: 62 }, // 11s
            Style { x: 100, y: 56 }, // 12s
            Style { x: 120, y: 50 }, // 13s (ended)
            Style { x: 120, y: 50 }, // 14s
        ]);
    }

    #[test]
    fn when_state_is_not_animated_pauses_previous_animation() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.0)
                .keyframe(Style::keyframe(1.0).x(80).y(20)))
            .build();

        let frame_values_a1 = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::B);
        let frame_values_b = run_animator(&mut animator, 1.0, 3.0);
        animator.set_state(&Interaction::A);
        let frame_values_a2 = run_animator(&mut animator, 1.0, 3.0);

        assert_eq!(frame_values_a1, &[
            Style { x: 0, y: 0 },
            Style { x: 16, y: 4 },
            Style { x: 32, y: 8 },
            Style { x: 48, y: 12 },
        ]);
        assert_eq!(frame_values_b, &[
            Style { x: 48, y: 12 },
            Style { x: 48, y: 12 },
            Style { x: 48, y: 12 },
            Style { x: 48, y: 12 },
        ]);
        assert_eq!(frame_values_a2, &[
            Style { x: 48, y: 12 },
            Style { x: 64, y: 16 },
            Style { x: 80, y: 20 },
            Style { x: 80, y: 20 },
        ]);
    }

    #[test]
    fn when_state_is_animated_and_animation_still_running_then_is_not_ended() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.)
                .delay_seconds(3.)
                .repeat(Repeat::Times(2))
                .keyframe(Style::keyframe(0.0).x(40).y(10))
                .keyframe(Style::keyframe(1.0).x(80).y(20)))
            .build();
        animator.set_state(&Interaction::A);
        animator.advance(17.);

        assert_eq!(animator.is_ended(), false);
    }

    #[test]
    fn when_state_is_animated_and_animation_reached_duration_then_is_ended() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.)
                .delay_seconds(3.)
                .repeat(Repeat::Times(2))
                .keyframe(Style::keyframe(0.0).x(40).y(10))
                .keyframe(Style::keyframe(1.0).x(80).y(20)))
            .build();
        animator.set_state(&Interaction::A);
        animator.advance(23.);

        assert_eq!(animator.is_ended(), true);
    }

    #[test]
    fn when_state_is_not_animated_then_is_ended() {
        let mut animator = StateAnimatorBuilder::new()
            .from_state(Interaction::A)
            .on(Interaction::A, Style::timeline()
                .duration_seconds(5.)
                .keyframe(Style::keyframe(1.0).x(80).y(20)))
            .build();
        animator.set_state(&Interaction::B);

        assert_eq!(animator.is_ended(), true);
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
            Style { x: 27, y: 48 }, // 6s
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
