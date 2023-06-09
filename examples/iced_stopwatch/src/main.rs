use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text};
use iced::window::frames;
use iced::{
    executor, theme, Alignment, Application, Background, Color, Command, Element, Length, Renderer,
    Settings, Subscription, Theme, Vector,
};
use mina::prelude::*;
use mina::Lerp;
use std::time::{Duration, Instant};

/// Animated version of the Iced stopwatch example.
///
/// Mainly for comparison purposes with other libraries/frameworks.
///
/// - Original example: <https://github.com/iced-rs/iced/tree/master/examples/stopwatch>
/// - Cosmic Time: <https://github.com/pop-os/cosmic-time/tree/main/examples/stopwatch>
fn main() -> iced::Result {
    App::run(Settings {
        antialiasing: true,
        ..Default::default()
    })
}

#[derive(Clone, Default, Eq, PartialEq, State)]
enum StopwatchState {
    #[default] Stopped,
    Started,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    Start,
    Stop,
    Reset,
}

struct App {
    last_tick: Instant,
    elapsed_time: Duration,
    app_style: EnumStateAnimator<StopwatchState, AppStyleTimeline>,
    toggle_button_style: EnumStateAnimator<StopwatchState, ButtonStyleTimeline>,
}

impl App {
    fn advance_to(&mut self, now: Instant) {
        let delta = now - self.last_tick;
        if self.app_style.current_state() == &StopwatchState::Started {
            self.elapsed_time += delta;
        }
        self.last_tick = now;
        self.app_style.advance(delta.as_secs_f32());
        self.toggle_button_style.advance(delta.as_secs_f32());
    }

    fn set_state(&mut self, state: &StopwatchState) {
        self.app_style.set_state(state);
        self.toggle_button_style.set_state(state);
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = Self {
            last_tick: Instant::now(),
            elapsed_time: Duration::ZERO,
            app_style: animator!(AppStyle {
                default(StopwatchState::Stopped, { background_color: Rgba::new_rgb(0xfca5a5) }),
                StopwatchState::Started => 6s Easing::InSine infinite
                    from default
                    16% { background_color: Rgba::new_rgb(0xb3f264) }
                    50% { background_color: Rgba::new_rgb(0x93c5fd) }
                    to default,
            }),
            toggle_button_style: animator!(ButtonStyle {
                default(StopwatchState::Stopped, {
                    background_color: Rgba::new_rgb(0x2563eb),
                    shadow_offset: 3.0,
                }),
                StopwatchState::Stopped => 500ms to default,
                StopwatchState::Started => 500ms to {
                    background_color: Rgba::new_rgb(0xdc2626),
                    shadow_offset: 5.0,
                }
            }),
        };
        (app, Command::none())
    }

    fn title(&self) -> String {
        "Mina-Iced Stopwatch Example".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Tick(now) => self.advance_to(now),
            Message::Start => self.set_state(&StopwatchState::Started),
            Message::Stop => self.set_state(&StopwatchState::Stopped),
            Message::Reset => self.elapsed_time = Duration::ZERO,
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let app_style = self.app_style.current_values();
        let toggle_button_style = self.toggle_button_style.current_values();
        let duration = text(format_duration(&self.elapsed_time)).size(40);
        let button = |label| {
            button(text(label).horizontal_alignment(Horizontal::Center))
                .padding(10)
                .width(80)
        };
        let toggle_button = match self.toggle_button_style.current_state() {
            StopwatchState::Stopped => button("Start").on_press(Message::Start),
            StopwatchState::Started => button("Stop").on_press(Message::Stop),
        }
        .style(theme::Button::Custom(Box::new(toggle_button_style.clone())));
        let reset_button = button("Reset")
            .style(theme::Button::Custom(Box::new(ButtonStyle {
                background_color: Rgba::new_rgb(0x3c3836),
                shadow_offset: 3.0,
            })))
            .on_press(Message::Reset);
        container(
            column![duration, row![toggle_button, reset_button].spacing(20),]
                .align_items(Alignment::Center)
                .spacing(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme::Container::Custom(Box::new(app_style.clone())))
        .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        frames().map(Message::Tick)
    }
}

#[derive(Animate, Clone, Default)]
struct AppStyle {
    background_color: Rgba,
}

impl container::StyleSheet for AppStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.background_color.into())),
            ..Default::default()
        }
    }
}

#[derive(Animate, Clone, Default)]
struct ButtonStyle {
    background_color: Rgba,
    shadow_offset: f32,
}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(self.background_color.into())),
            border_color: self.background_color.into(),
            border_radius: 10.0,
            border_width: 10.0,
            shadow_offset: Vector::new(self.shadow_offset, self.shadow_offset),
            text_color: Color::WHITE,
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        self.active(_style)
    }

    fn pressed(&self, _style: &Self::Style) -> button::Appearance {
        self.active(_style)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Rgba(Color);

impl Rgba {
    fn new_rgb(rgb: u32) -> Self {
        let (r, g, b) = (rgb >> 16 & 0xff, rgb >> 8 & 0xff, rgb & 0xff);
        Self(Color::from_rgb8(r as u8, g as u8, b as u8))
    }
}

impl From<Rgba> for Color {
    fn from(value: Rgba) -> Self {
        value.0
    }
}

// Interpolating over the RGB space is not a pleasing visual effect; this should normally be done
// over HSV/HSL, or better yet, LCh. However, this example is done for comparison purposes with
// the examples from other libraries, so we stick to RGB.
impl Lerp for Rgba {
    fn lerp(&self, y1: &Self, x: f32) -> Self {
        Self(Color::from_rgb(
            self.0.r.lerp(&y1.0.r, x),
            self.0.g.lerp(&y1.0.g, x),
            self.0.b.lerp(&y1.0.b, x),
        ))
    }
}

fn format_duration(duration: &Duration) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;

    let seconds = duration.as_secs();
    format!(
        "{:0>2}:{:0>2}:{:0>2}.{:0>2}",
        seconds / HOUR,
        (seconds % HOUR) / MINUTE,
        seconds % MINUTE,
        duration.subsec_millis() / 10,
    )
}
