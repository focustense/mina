use iced::widget::{button, column, container, row, text, vertical_space, Container};
use iced::window::frames;
use iced::{
    executor, theme, window, Alignment, Application, Background, Color, Command, Element, Length,
    Renderer, Settings, Subscription, Theme,
};
use mina::prelude::*;
use std::time::Instant;

/// Example combining the [`Animate`](mina::Animate) and [`animator`](mina::animator) macros to
/// create and animate a timeline for an Iced `StyleSheet`, which controls the visual appearance of
/// some widget.
///
/// This is the most basic method of running an animation, which is quite limited due to Iced
/// "stock" widgets not being designed for sophisticated transformations, but with some clever
/// layout hacks it is still possible to do some interesting, albeit janky things.
///
/// Much more elaborate effects are possible with custom widgets, i.e. with direct access to the
/// [`Renderer`] and such methods as [`Renderer::draw_primitive`], but for serious whizbang effects
/// we'll need to use a `Canvas`. This is the pure vanilla example for users who just want to do
/// something quick, like fade in some content, and aren't prepared to deal with the low-level APIs
/// and traits that Iced provides for advanced customization.
///
/// Most of what's here is boilerplate required for any Iced app and widget tree. The interesting
/// parts are the initialization of `card_animators`, and the `CardState` and `CardStyle` types
/// which define the animator state and animator values, respectively. The use of a `Tick` message
/// is common to many/most animation crates that try to work with Iced, since it's generally the
/// only way to trigger frame-level events without making changes to the library.
fn main() -> iced::Result {
    App::run(Settings {
        antialiasing: true,
        window: window::Settings {
            size: (1280, 720),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    ShowCards,
    HideCards,
}

struct App {
    last_tick: Instant,
    card_animators: [EnumStateAnimator<CardState, CardStyleTimeline>; 3],
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = Self {
            last_tick: Instant::now(),
            card_animators: [
                animator!(CardStyle {
                    default(CardState::Hidden, CardStyle::new(Color::from_rgb(0.5, 0.5, 0.25))),
                    CardState::Hidden => 0.5s Easing::OutQuart to default,
                    CardState::Visible => [
                        2s Easing::In to { alpha: 1.0 },
                        1s Easing::OutCubic to { scale: 1.0 },
                    ]
                }),
                animator!(CardStyle {
                    default(CardState::Hidden, CardStyle::new(Color::from_rgb(0.35, 0.56, 0.32))),
                    CardState::Hidden => 0.5s Easing::OutQuart to default,
                    CardState::Visible => [
                        2.5s after 0.1s Easing::In to { alpha: 1.0 },
                        1.25s after 0.1s Easing::OutCubic to { scale: 1.0 },
                    ]
                }),
                animator!(CardStyle {
                    default(CardState::Hidden, CardStyle::new(Color::from_rgb(0.21, 0.55, 0.77))),
                    CardState::Hidden => 0.5s Easing::OutQuart to default,
                    CardState::Visible => [
                        3s after 0.2s Easing::In to { alpha: 1.0 },
                        1.5s after 0.2s Easing::OutCubic to { scale: 1.0 },
                    ]
                }),
            ],
        };
        (app, Command::none())
    }

    fn title(&self) -> String {
        "Mina-Iced app-controlled animation".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Tick(time) => {
                let elapsed_seconds = (time - self.last_tick).as_secs_f32();
                self.last_tick = time;
                for animator in self.card_animators.iter_mut() {
                    animator.advance(elapsed_seconds);
                }
            }
            Message::HideCards => {
                for animator in self.card_animators.iter_mut() {
                    animator.set_state(&CardState::Hidden);
                }
            }
            Message::ShowCards => {
                for animator in self.card_animators.iter_mut() {
                    animator.set_state(&CardState::Visible)
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let toggle_button = if self.card_animators[0].current_state() == &CardState::Hidden {
            button(text("Show cards").size(36))
                .padding(16.0)
                .on_press(Message::ShowCards)
        } else {
            button(text("Hide cards").size(36))
                .padding(16.0)
                .on_press(Message::HideCards)
        };
        container(
            column![
                row![
                    card("Hello", self.card_animators[0].current_values()),
                    card("From", self.card_animators[1].current_values()),
                    card("Mina", self.card_animators[2].current_values())
                ]
                .height(300.0)
                .spacing(16),
                vertical_space(24),
                toggle_button,
            ]
            .align_items(Alignment::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .center_y()
        .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        frames().map(Message::Tick)
    }
}

fn card<'a>(title: &'a str, style: &'a CardStyle) -> Container<'a, Message> {
    let card = container(text(title).size(48.0 * style.scale))
        .width(300.0 * style.scale)
        .height(250.0 * style.scale)
        .center_x()
        .center_y()
        .style(theme::Container::Custom(Box::new(style.clone())));
    // Use an outer container to ensure that the space used is constant. Iced wasn't exactly
    // designed with widget transforms in mind so we have to fake these layout-preserving effects.
    container(card).width(300).height(250).center_x().center_y()
}

#[derive(Clone, Default, Eq, PartialEq, State)]
enum CardState {
    #[default] Hidden,
    Visible,
}

#[derive(Animate, Clone, Default)]
struct CardStyle {
    #[animate] alpha: f32,
    background_color: Color,
    #[animate] scale: f32,
}

impl CardStyle {
    pub fn new(background_color: Color) -> Self {
        Self {
            background_color,
            alpha: 0.0,
            scale: 0.0,
        }
    }
}

impl container::StyleSheet for CardStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let Color { r, g, b, .. } = self.background_color;
        let background_color = Color::from_rgba(r, g, b, self.alpha);
        let text_color = Color::from_rgba(1.0, 1.0, 1.0, self.alpha);
        container::Appearance {
            background: Some(Background::Color(background_color)),
            border_radius: 16.0,
            text_color: Some(text_color),
            ..Default::default()
        }
    }
}
